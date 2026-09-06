use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 分页响应外壳：`{page, size, total, items}`。翻页由 agent 驱动。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub page: u32,
    pub size: u32,
    pub total: u64,
    pub items: Vec<T>,
}

/* ---------------- 分腿收益曲线（live/analysis 与 experiment performance-report 共用） ---------------- */

/// 单腿曲线三元组。后端 `cum` 是 `daily` 的**单利累加**（`cum(t) = Σ daily[0..t]`），
/// 不是复利净值——区间收益=两端相减，别套净值换算公式（skz-client 同一口径）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegCurve {
    #[serde(default)]
    pub cum: Vec<f64>,
    #[serde(default)]
    pub daily: Vec<f64>,
    #[serde(default)]
    pub drawdown: Vec<f64>,
}

/// 腿 → 曲线。键是中文腿名（多空/多头/空头/基准/超额）；BTreeMap 让输出键序稳定。
pub type LegCurves = BTreeMap<String, LegCurve>;

/// 品种收益贡献行。`return` 是 Rust 关键字，serde 层 rename。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReturn {
    #[serde(rename = "return")]
    pub ret: f64,
    pub symbol: String,
}
