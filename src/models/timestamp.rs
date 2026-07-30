//! 时间戳字段类型：**入口原样收下，出口统一成东八区**。
//!
//! 后端发的时间戳是 UTC，但本工具面向 A 股用户与 agent——`2026-07-25 17:20:41` 这种
//! 不带时区标记的串会被直接当北京时间读，实际差 8 小时。判断「策略心跳停没停」
//! 「这批因子是不是昨晚挖的」时，这 8 小时足以把结论读反，所以在输出侧统一换算。
//!
//! 不引 chrono/jiff：东八区是固定偏移、中国不实行夏令时，所需的全部计算就是一段
//! 整数历法换算（Howard Hinnant 的 days_from_civil / civil_from_days）。为一个常量偏移
//! 拉一个时区数据库，跟出货 profile 的 `opt-level = "z"` 与本项目的依赖克制取向都不搭。
//!
//! **只有事件发生时刻用这个类型**。纯日期（交易日 `cal_date`、区间边界 `sdt`/`edt`、
//! 权重日 `dt`）保持 `String`：±8h 会让它们整体跨日，把「7月24日的持仓」变成 25 日。
//! `serde_json::Value` 透传块同理不碰——`strategy trades` 的 `kline_key` 内嵌时间，
//! 却是要原样回传给 `strategy kline` 的路径参数，改写即查不到那根 K 线。

use serde::{Deserialize, Serialize, Serializer};

/// 东八区偏移（分钟）。固定值，无夏令时。
const CST_OFFSET_MIN: i64 = 8 * 60;

/// 时间戳字段。反序列化时**原样收下任何字符串、不做校验**（后端某天发个没见过的形状，
/// 也只是这一个字段照原样透传，不该让整条响应 exit 6）；序列化时才尝试换算。
/// `Default` 是给 `#[serde(default)]` 字段用的（如 `PortfolioMeta.generated_at`）：
/// 空串解析不了、原样输出空串，与改造前 `String::default()` 的行为一致。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// 换算后的东八区表示；解析不了就是后端原文。输出走的就是这个值。
    pub fn to_cst_string(&self) -> String {
        to_cst(&self.0).unwrap_or_else(|| self.0.clone())
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_cst_string())
    }
}

/// 换算成 `YYYY-MM-DDTHH:MM:SS[.frac]+08:00`；解析不了返回 `None`（调用方原样透传）。
fn to_cst(input: &str) -> Option<String> {
    let (utc, frac) = parse(input)?;
    let cst = utc + CST_OFFSET_MIN * 60;
    let (y, mo, d) = civil_from_days(cst.div_euclid(86_400));
    let rem = cst.rem_euclid(86_400);
    Some(format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}{frac}+08:00",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
    ))
}

/// 解析 `YYYY-MM-DD`(`T`|空格)`HH:MM:SS`[`.frac`][`Z`|`±HH:MM`] → (UTC 纪元秒, 小数部分)。
/// 只认后端真实发过的这几种形状；别的（含**纯日期**）返回 `None` 让调用方原样透传——
/// 交易日绝不该被移位，这也是白名单万一挂错字段时的第二道防线。
/// 无偏移标记视为 UTC（后端口径）；小数秒原样保留，不重排精度。
fn parse(input: &str) -> Option<(i64, &str)> {
    let b = input.as_bytes();
    if !input.is_ascii() || b.len() < 19 {
        return None;
    }
    // 定长切片直接取数：非全数字（含空串）一律 None，交给调用方原样透传。
    let num = |s: &str| -> Option<i64> {
        (!s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };
    if b[4] != b'-' || b[7] != b'-' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    if !matches!(b[10], b'T' | b't' | b' ') {
        return None;
    }
    let (y, mo, d) = (num(&input[0..4])?, num(&input[5..7])?, num(&input[8..10])?);
    let (h, mi, s) = (
        num(&input[11..13])?,
        num(&input[14..16])?,
        num(&input[17..19])?,
    );
    // 闰秒 60 照收：算术上会自然滚进下一分钟。
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }

    // 小数秒（可有可无），随后必须正好是时区标记或什么都没有。
    let tail = &input[19..];
    let frac_len = match tail.strip_prefix('.') {
        Some(rest) => match rest.bytes().take_while(u8::is_ascii_digit).count() {
            0 => return None, // 光一个点没数字
            n => n + 1,
        },
        None => 0,
    };
    let (frac, tz) = tail.split_at(frac_len);
    let off_min = match tz {
        "" | "Z" | "z" => 0,
        _ => {
            let (sign, hm) = match tz.split_at(1) {
                ("+", hm) => (1, hm),
                ("-", hm) => (-1, hm),
                _ => return None,
            };
            let (oh, om) = hm.split_once(':')?;
            if oh.len() != 2 || om.len() != 2 {
                return None;
            }
            let (oh, om) = (num(oh)?, num(om)?);
            if oh > 23 || om > 59 {
                return None;
            }
            sign * (oh * 60 + om)
        }
    };

    // 先按字面量算成「本地」秒，再减去自身偏移回到 UTC——这样 `+00:00` / `Z` / 无标记
    // 三种形状走同一条路，不用分别处理。
    let local = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s;
    Some((local - off_min * 60, frac))
}

/// 民用日期 → 距 1970-01-01 的天数（Howard Hinnant, chrono 系算法的公有领域原型）。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 天数 → 民用日期（`days_from_civil` 的逆）。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cst(s: &str) -> String {
        Timestamp(s.to_string()).to_cst_string()
    }

    #[test]
    fn converts_the_four_shapes_the_backend_sends() {
        // 带偏移 + 小数秒（market symbols 的 updateAt）
        assert_eq!(
            cst("2026-07-08T12:31:22.989173+00:00"),
            "2026-07-08T20:31:22.989173+08:00"
        );
        // Z（promote 回执）
        assert_eq!(cst("2026-07-24T00:00:00Z"), "2026-07-24T08:00:00+08:00");
        // 空格分隔、无时区标记（strategy get 的 update_time）
        assert_eq!(cst("2026-07-25 17:20:41"), "2026-07-26T01:20:41+08:00");
        // T 分隔、无时区标记
        assert_eq!(cst("2016-09-28T16:00:00"), "2016-09-29T00:00:00+08:00");
    }

    #[test]
    fn rolls_over_month_year_and_leap_day() {
        assert_eq!(cst("2026-07-31T20:00:00Z"), "2026-08-01T04:00:00+08:00");
        assert_eq!(cst("2025-12-31T16:00:00Z"), "2026-01-01T00:00:00+08:00");
        assert_eq!(cst("2024-02-28T20:00:00Z"), "2024-02-29T04:00:00+08:00");
        assert_eq!(cst("2023-02-28T20:00:00Z"), "2023-03-01T04:00:00+08:00");
    }

    #[test]
    fn non_utc_offset_aligns_by_real_instant() {
        // 按真实时刻对齐，不是对字面量加减：+09:00 的 09:00 = UTC 00:00 = 东八区 08:00
        assert_eq!(
            cst("2026-07-24T09:00:00+09:00"),
            "2026-07-24T08:00:00+08:00"
        );
        assert_eq!(
            cst("2026-07-23T20:00:00-04:00"),
            "2026-07-24T08:00:00+08:00"
        );
    }

    #[test]
    fn date_only_passes_through() {
        // 交易日绝不移位——白名单挂错字段时的第二道防线
        assert_eq!(cst("2026-07-24"), "2026-07-24");
    }

    #[test]
    fn unparsable_passes_through() {
        for s in [
            "",
            "not a time",
            "2026-13-01T00:00:00Z",      // 月份越界
            "2026-07-24T25:00:00Z",      // 小时越界
            "2026-07-24T00:00:00+99:00", // 偏移越界
            "2026-07-24T00:00:00.Z",     // 光一个点没数字
            "2026-07-24T00:00:00Zzz",    // 时区标记后有残留
            "2026-07-24T00:00:00+0900",  // 只认 ±HH:MM
            "2026-07-24T00:00Z",         // 只认带秒
            "2026/07/24 00:00:00",
        ] {
            assert_eq!(cst(s), s, "应原样透传: {s}");
        }
    }

    #[test]
    fn serde_roundtrip_converts_on_output() {
        let t: Timestamp = serde_json::from_str(r#""2026-07-24T00:00:00Z""#).unwrap();
        assert_eq!(
            serde_json::to_string(&t).unwrap(),
            r#""2026-07-24T08:00:00+08:00""#
        );
    }
}
