//! 分腿曲线 → 前端同款图表行。
//!
//! 照 skz-client `performance-report-utils.ts` 移植的纯函数层：按日期对齐拼行、
//! 区间 rebase、归一化图的超额腿派生、缩放/年化摘要。两个数据源共用：
//! live/analysis 的 `rebuilt`（非 ready 时降级 `persisted.nav`）与 experiment
//! performance-report 的 `curves`/`normalized_20`。
//!
//! 口径提醒（容易踩的坑都在这里）：后端 `cum` 是 daily 的**单利累加**，区间收益 =
//! 两端相减，不是复利净值；空头是反向暴露，剥 beta 求「空头超额」要**加回**基准。
//! rebase 与超额派生都是线性运算，先后顺序不影响结果。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::common::LegCurve;
use crate::models::experiment::PerformanceReport;
use crate::models::live::LiveAnalysis;

/// 多空收益对比图的腿（策略自身三腿；基准与超额留给归一化图）。
pub const RETURN_CHART_LEGS: &[&str] = &["多空", "多头", "空头"];

/// 归一化图的后端原始腿；两条超额腿由它们派生（见 [`derive_excess_legs`]）。
pub const NORMALIZED_BASE_LEGS: &[&str] = &["多空", "多头", "空头", "基准"];

/// 日期轴取全部五腿的并集，只画三腿不应让 X 轴变短。
const ALL_LEGS: &[&str] = &["多空", "多头", "空头", "基准", "超额"];

const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// 一行 = 一个交易日 × 各腿取值。值flatten进对象：`{"date":"…","多空":0.01,…}`。
/// 只装有限数值；某腿该日缺失/非有限就不出现这个键（前端同款）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChartRow {
    pub date: String,
    #[serde(flatten)]
    pub values: BTreeMap<String, f64>,
}

impl ChartRow {
    fn empty(date: &str) -> Self {
        ChartRow {
            date: date.to_string(),
            values: BTreeMap::new(),
        }
    }
}

/// 归一化图区间摘要：`scale` 是反推的后端缩放倍数，`annualized_return` 是
/// 当前区间的单利折算年化。两者都可能缺（样本不足 / 区间为空）。
#[derive(Debug, Clone, Serialize)]
pub struct NormalizedSummary {
    pub annualized_return: Option<f64>,
    pub scale: Option<f64>,
}

/// `--chart-rows` 的输出体。
#[derive(Debug, Clone, Serialize)]
pub struct ChartRowsOutput {
    pub dates: Vec<String>,
    pub normalized_rows: Vec<ChartRow>,
    pub normalized_summary: Option<NormalizedSummary>,
    pub rows: Vec<ChartRow>,
}

/// `--from`/`--to` 解析出的可见区间（对 dates 下标，双闭）。
struct VisibleRange {
    end: usize,
    start: usize,
}

/// `YYYY-MM-DD HH:MM:SS` / `YYYY-MM-DDTHH:MM:SS` 条目的日期前缀（比较与展示都用它）。
fn date_prefix(date: &str) -> &str {
    date.get(..10).unwrap_or(date)
}

/// 用 `--from`/`--to`（YYYY-MM-DD，含端点）在日期轴上解析可见区间。
/// 轴为空或区间无交集时返回 None（输出空图表）。
fn resolve_visible_range(
    dates: &[String],
    from: Option<&str>,
    to: Option<&str>,
) -> Option<VisibleRange> {
    if dates.is_empty() {
        return None;
    }
    let start = match from {
        Some(f) => dates
            .iter()
            .position(|d| date_prefix(d).ge(f))
            .unwrap_or(dates.len()),
        None => 0,
    };
    let end = match to {
        Some(t) => dates.iter().rposition(|d| date_prefix(d).le(t))?,
        None => dates.len() - 1,
    };
    if start > end {
        return None;
    }
    Some(VisibleRange { end, start })
}

/// 拼行：每个日期一行，取各腿 `cum[index]`；某腿该日非有限就不写键。
/// `dates` 可能长于曲线（实盘侧见过），全空的日期整行丢弃——
/// 否则 X 轴会延伸到没有数据的未来区间，图表右侧留白（前端同款注释）。
pub fn build_curve_rows(
    dates: &[String],
    curves: &BTreeMap<String, LegCurve>,
    legs: &[&str],
) -> Vec<ChartRow> {
    dates
        .iter()
        .enumerate()
        .filter_map(|(index, date)| {
            let mut values = BTreeMap::new();
            for leg in legs {
                if let Some(v) = curves
                    .get(*leg)
                    .and_then(|c| c.cum.get(index))
                    .filter(|v| v.is_finite())
                {
                    values.insert((*leg).to_string(), *v);
                }
            }
            (!values.is_empty()).then(|| ChartRow {
                date: date.clone(),
                values,
            })
        })
        .collect()
}

/// 主图与归一化图共用一条 X 轴：缺日期的行补空行占位，避免两图 series 与轴错位。
fn align_rows_to_dates(rows: Vec<ChartRow>, dates: &[String]) -> Vec<ChartRow> {
    let by_date: BTreeMap<String, ChartRow> = rows
        .into_iter()
        .map(|row| (row.date.clone(), row))
        .collect();
    dates
        .iter()
        .map(|date| {
            by_date
                .get(date)
                .cloned()
                .unwrap_or_else(|| ChartRow::empty(date))
        })
        .collect()
}

/// 切时间范围后所有腿都要从 0 重新累积：各腿减去区间起点值。
/// 起点行缺值的腿保持原值（前端同款）；单利累加下区间收益就是两端相减。
fn rebase_rows(rows: Vec<ChartRow>, start_index: usize) -> Vec<ChartRow> {
    let base_values = rows
        .get(start_index)
        .map(|base| base.values.clone())
        .unwrap_or_default();
    rows.into_iter()
        .map(|row| {
            let values: BTreeMap<String, f64> = row
                .values
                .into_iter()
                .map(|(leg, value)| match base_values.get(&leg) {
                    Some(base_value) => (leg, value - base_value),
                    None => (leg, value),
                })
                .collect();
            ChartRow {
                date: row.date,
                values,
            }
        })
        .collect()
}

/// 归一化图的派生腿。超额只在同一 20% 风险预算下才可比；空头相对基准是反向暴露，
/// 剥离 beta 要**加回**基准而不是减去。派生后读出来的就是「区间内跑赢基准多少」。
fn derive_excess_legs(rows: Vec<ChartRow>) -> Vec<ChartRow> {
    rows.into_iter()
        .map(|row| {
            let mut values = row.values;
            if let Some(benchmark) = values.get("基准").copied() {
                if let Some(long) = values.get("多头").copied() {
                    values.insert("多头超额".to_string(), long - benchmark);
                }
                if let Some(short) = values.get("空头").copied() {
                    values.insert("空头超额".to_string(), short + benchmark);
                }
            }
            ChartRow {
                date: row.date,
                values,
            }
        })
        .collect()
}

/// 样本标准差（n-1 分母）。样本 < 2 或含非有限值 → None；方差为 0 → None（除零保护）。
fn sample_standard_deviation(values: &[f64]) -> Option<f64> {
    if values.len() < 2 || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = variance.sqrt();
    (sd > f64::EPSILON).then_some(sd)
}

/// 归一化图区间摘要。波动率/缩放用**全样本** daily 反推（前端同款，不随区间变）；
/// 年化按当前区间的归一化 cum 端点差单利折算：`(cum_end − cum_start) / 区间天数 × 252`。
fn normalized_range_summary(
    raw: Option<&LegCurve>,
    normalized: Option<&LegCurve>,
    start_index: usize,
    end_index: usize,
) -> NormalizedSummary {
    let raw_sd = raw.and_then(|c| sample_standard_deviation(&c.daily));
    let norm_sd = normalized.and_then(|c| sample_standard_deviation(&c.daily));
    // 两条 daily 等长才谈得上「后端把 daily 缩放了几倍」。
    let same_length = raw.map_or(0, |c| c.daily.len()) == normalized.map_or(0, |c| c.daily.len());
    let scale = match (raw_sd, norm_sd, same_length) {
        (Some(raw_vol), Some(norm_vol), true) => Some(norm_vol / raw_vol),
        _ => None,
    };
    let periods = end_index.saturating_sub(start_index);
    let annualized_return = if periods == 0 {
        None
    } else {
        normalized.and_then(|c| {
            let start = *c.cum.get(start_index)?;
            let end = *c.cum.get(end_index)?;
            (start.is_finite() && end.is_finite())
                .then(|| (end - start) / periods as f64 * TRADING_DAYS_PER_YEAR)
        })
    };
    NormalizedSummary {
        annualized_return,
        scale,
    }
}

fn empty_output() -> ChartRowsOutput {
    ChartRowsOutput {
        dates: Vec::new(),
        normalized_rows: Vec::new(),
        normalized_summary: None,
        rows: Vec::new(),
    }
}

/// 分腿曲线 → 图表行的总装（对应前端 StrategyPerformanceReport 组件的数据准备段）。
fn curves_chart(
    dates: &[String],
    curves: &BTreeMap<String, LegCurve>,
    normalized_20: &BTreeMap<String, LegCurve>,
    from: Option<&str>,
    to: Option<&str>,
) -> ChartRowsOutput {
    // 日期轴用全部五腿的并集行；三腿图、四腿图都对齐到同一条轴。
    let axis = build_curve_rows(dates, curves, ALL_LEGS);
    let axis_dates: Vec<String> = axis.iter().map(|r| r.date.clone()).collect();
    let Some(range) = resolve_visible_range(&axis_dates, from, to) else {
        return empty_output();
    };
    let rows = rebase_rows(
        align_rows_to_dates(
            build_curve_rows(dates, curves, RETURN_CHART_LEGS),
            &axis_dates,
        ),
        range.start,
    );
    let normalized_rows = derive_excess_legs(rebase_rows(
        align_rows_to_dates(
            build_curve_rows(dates, normalized_20, NORMALIZED_BASE_LEGS),
            &axis_dates,
        ),
        range.start,
    ));
    ChartRowsOutput {
        dates: axis_dates[range.start..=range.end].to_vec(),
        normalized_rows: normalized_rows[range.start..=range.end].to_vec(),
        normalized_summary: Some(normalized_range_summary(
            curves.get("多空"),
            normalized_20.get("多空"),
            range.start,
            range.end,
        )),
        rows: rows[range.start..=range.end].to_vec(),
    }
}

/// `skz strategy live-analysis --chart-rows`：rebuilt ready 走分腿曲线；
/// 非 ready（unavailable/inconsistent）按前端 live-strategy-performance 降级——
/// 用 persisted.nav 画单腿（多空 = nav − 1），归一化图与摘要置空。
pub fn live_analysis_chart(
    data: &LiveAnalysis,
    from: Option<&str>,
    to: Option<&str>,
) -> ChartRowsOutput {
    if data.rebuilt.status == "ready" {
        return curves_chart(
            &data.rebuilt.dates,
            &data.rebuilt.curves,
            &data.rebuilt.normalized_20,
            from,
            to,
        );
    }
    let dates = &data.persisted.dates;
    let Some(range) = resolve_visible_range(dates, from, to) else {
        return empty_output();
    };
    let rows: Vec<ChartRow> = dates
        .iter()
        .enumerate()
        .filter_map(|(i, date)| {
            let nav = data.persisted.nav.get(i).filter(|v| v.is_finite())?;
            Some(ChartRow {
                date: date.clone(),
                values: BTreeMap::from([("多空".to_string(), nav - 1.0)]),
            })
        })
        .collect();
    ChartRowsOutput {
        dates: dates[range.start..=range.end].to_vec(),
        normalized_rows: Vec::new(),
        normalized_summary: None,
        rows: rows[range.start..=range.end].to_vec(),
    }
}

/// `skz experiment performance-report --chart-rows`：回测快照没有降级路径，
/// 曲线缺失就是空输出。
pub fn performance_report_chart(
    data: &PerformanceReport,
    from: Option<&str>,
    to: Option<&str>,
) -> ChartRowsOutput {
    curves_chart(&data.dates, &data.curves, &data.normalized_20, from, to)
}

/// `--from`/`--to` 的本地校验：必须是 `YYYY-MM-DD`（比较只看 10 字节前缀，
/// 提前拦住糊糊日期，免得区间静默解析成空）。
pub fn validate_date_flag(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    let ok = bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        });
    if ok {
        Ok(())
    } else {
        Err(format!("日期须为 YYYY-MM-DD，收到：{value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::LegCurves;

    fn leg(cum: Vec<f64>, daily: Vec<f64>) -> LegCurve {
        let drawdown = vec![0.0; cum.len()];
        LegCurve {
            cum,
            daily,
            drawdown,
        }
    }

    fn dates(n: usize) -> Vec<String> {
        (0..n)
            .map(|i| format!("2024-01-{:02} 00:00:00", i + 1))
            .collect()
    }

    #[test]
    fn build_rows_drops_dates_without_any_finite_value() {
        let mut curves: LegCurves = BTreeMap::new();
        curves.insert(
            "多空".to_string(),
            leg(vec![0.1, f64::NAN, 0.3], vec![0.1; 3]),
        );
        let rows = build_curve_rows(&dates(3), &curves, &["多空"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-01 00:00:00");
        assert_eq!(rows[0].values["多空"], 0.1);
        assert_eq!(rows[1].date, "2024-01-03 00:00:00");
    }

    #[test]
    fn rebase_subtracts_base_and_keeps_legs_missing_at_base() {
        let base_row = ChartRow {
            date: "d0".into(),
            values: BTreeMap::from([("多空".to_string(), 1.0)]),
        };
        let rows = vec![
            base_row,
            ChartRow {
                date: "d1".into(),
                values: BTreeMap::from([("多空".to_string(), 1.25), ("超额".to_string(), 0.5)]),
            },
        ];
        let rebased = rebase_rows(rows, 0);
        assert_eq!(rebased[1].values["多空"], 0.25);
        // 起点行没有「超额」腿 → 保持原值（前端同款）。
        assert_eq!(rebased[1].values["超额"], 0.5);
    }

    #[test]
    fn excess_legs_long_minus_benchmark_short_plus_benchmark() {
        let rows = vec![ChartRow {
            date: "d0".into(),
            values: BTreeMap::from([
                ("多头".to_string(), 0.30),
                ("空头".to_string(), 0.10),
                ("基准".to_string(), 0.05),
            ]),
        }];
        let derived = derive_excess_legs(rows);
        // 空头是反向暴露：剥 beta 是加回基准，不是减。
        assert!((derived[0].values["多头超额"] - 0.25).abs() < 1e-12);
        assert!((derived[0].values["空头超额"] - 0.15).abs() < 1e-12);
    }

    #[test]
    fn excess_legs_skipped_without_benchmark() {
        let rows = vec![ChartRow {
            date: "d0".into(),
            values: BTreeMap::from([("多头".to_string(), 0.3)]),
        }];
        let derived = derive_excess_legs(rows);
        assert!(!derived[0].values.contains_key("多头超额"));
    }

    #[test]
    fn normalized_summary_scale_and_annualization() {
        // normalized 的 daily 是 raw 的 2 倍 → 缩放 2.0；
        // 区间 3 天 cum 端点差 0.02 → 年化 0.02/3*252 = 1.68。
        let raw_daily = [0.01, -0.01, 0.01, -0.01];
        let raw = leg(vec![0.0, 0.01, 0.0, 0.01], raw_daily.to_vec());
        let norm = leg(
            vec![0.0, 0.02, 0.0, 0.02],
            raw_daily.iter().map(|v| v * 2.0).collect(),
        );
        let summary = normalized_range_summary(Some(&raw), Some(&norm), 0, 3);
        assert_eq!(summary.scale, Some(2.0));
        assert!((summary.annualized_return.unwrap() - 1.68).abs() < 1e-9);
    }

    #[test]
    fn normalized_summary_none_on_degenerate_samples() {
        // 单点样本（<2）没有标准差 → scale None；periods=0 → 年化 None。
        let raw = leg(vec![0.0], vec![0.0]);
        let norm = leg(vec![0.0], vec![0.0]);
        let summary = normalized_range_summary(Some(&raw), Some(&norm), 0, 0);
        assert_eq!(summary.scale, None);
        assert_eq!(summary.annualized_return, None);
        // 零方差（全相同值）同样拿不到缩放。
        let flat = leg(vec![0.0, 0.0], vec![0.0, 0.0]);
        let summary = normalized_range_summary(Some(&flat), Some(&flat), 0, 1);
        assert_eq!(summary.scale, None);
    }

    #[test]
    fn curves_chart_rebases_to_range_start_and_derives_excess() {
        let mut curves: LegCurves = BTreeMap::new();
        curves.insert(
            "多空".to_string(),
            leg(vec![0.10, 0.20, 0.40], vec![0.1; 3]),
        );
        curves.insert(
            "多头".to_string(),
            leg(vec![0.06, 0.12, 0.25], vec![0.1; 3]),
        );
        curves.insert(
            "空头".to_string(),
            leg(vec![0.04, 0.08, 0.15], vec![0.1; 3]),
        );
        curves.insert(
            "基准".to_string(),
            leg(vec![0.02, 0.04, 0.06], vec![0.1; 3]),
        );
        let mut norm: LegCurves = BTreeMap::new();
        norm.insert("多空".to_string(), leg(vec![0.2, 0.4, 0.8], vec![0.2; 3]));
        norm.insert("多头".to_string(), leg(vec![0.12, 0.24, 0.5], vec![0.1; 3]));
        norm.insert(
            "基准".to_string(),
            leg(vec![0.04, 0.08, 0.12], vec![0.1; 3]),
        );
        let dates = dates(3);
        let out = curves_chart(&dates, &curves, &norm, Some("2024-01-02"), None);
        // --from 命中第二行：日期轴只剩 2 天。
        assert_eq!(
            out.dates,
            vec!["2024-01-02 00:00:00", "2024-01-03 00:00:00"]
        );
        // rebase 到区间起点：多空 0.20→0、0.40→0.20。
        assert_eq!(out.rows[0].values["多空"], 0.0);
        assert_eq!(out.rows[1].values["多空"], 0.20);
        // 归一化图派生腿：多头 0.5−0.24=0.26，基准 0.12−0.08=0.04 → 超额 0.22。
        assert!((out.normalized_rows[1].values["多头超额"] - 0.22).abs() < 1e-12);
        // 摘要：区间 1 天，归一化 cum 端点差 0.4 → 年化 0.4*252=100.8。
        let summary = out.normalized_summary.unwrap();
        assert!((summary.annualized_return.unwrap() - 100.8).abs() < 1e-9);
    }

    #[test]
    fn curves_chart_empty_when_range_outside_axis() {
        let mut curves: LegCurves = BTreeMap::new();
        curves.insert("多空".to_string(), leg(vec![0.1, 0.2], vec![0.1; 2]));
        let out = curves_chart(
            &dates(2),
            &curves,
            &BTreeMap::new(),
            Some("2030-01-01"),
            None,
        );
        assert!(out.dates.is_empty());
        assert!(out.rows.is_empty());
        assert!(out.normalized_summary.is_none());
    }

    #[test]
    fn date_flag_accepts_only_yyyy_mm_dd() {
        assert!(validate_date_flag("2024-01-02").is_ok());
        assert!(validate_date_flag("2024/01/02").is_err());
        assert!(validate_date_flag("2024-1-2").is_err());
        assert!(validate_date_flag("").is_err());
    }
}
