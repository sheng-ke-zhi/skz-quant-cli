//! 组合（portfolio，研究后端 `/research/portfolios*`）响应类型。
//! 逐字段对照 `user-research-backend/src/services/portfolios.rs` 的响应结构体移植：
//! 这条面本来就是 snake_case，无需 camelCase rename。中文键 / 动态 map（`metrics`、
//! `compare_metrics`、`verdict`、`positions.weights`、`compare.series`）用
//! `serde_json::Value` 原样承载，避免中文标识符且不丢字段。
//!
//! 后端结构体本身不用 `skip_serializing_if`，字段键理论上总会出现；但后端仍在演进
//! （见 memory 里两个曾经的 model bug），这里对 `Option`/`Vec`/`Value` 统一加
//! `#[serde(default)]` 防漂移——比照 `live.rs` 的选择性加法，我们没有真机数据能像那边一样
//! 精确判断哪个字段"确实会缺席"，宁可全加、成本为零。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ---------------- GET /research/portfolios（组合库列表） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioListItem {
    pub code: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub base_market: String,
    pub base_freq: String,
    pub symbol_count: usize,
    pub strategy_count: usize,
    #[serde(default)]
    pub sdt: String,
    #[serde(default)]
    pub edt: String,
    #[serde(default)]
    pub annual_return: Option<f64>,
    #[serde(default)]
    pub sharpe: Option<f64>,
    #[serde(default)]
    pub max_drawdown: Option<f64>,
    #[serde(default)]
    pub abs_return: Option<f64>,
    /// 异步建组合任务态：`pending`/`failed`；已就绪（磁盘真实组合）时缺省为 `None`。
    /// **轮询建组合进度用这个字段，别用 `portfolio get`**——生成中/失败时 get 一律 404。
    #[serde(default)]
    pub job_status: Option<String>,
    #[serde(default)]
    pub job_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioList {
    #[serde(default)]
    pub items: Vec<PortfolioListItem>,
}

/* ---------------- GET /research/portfolios/{code}（详情） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioMeta {
    pub code: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub base_market: String,
    pub base_freq: String,
    pub price_field: String,
    pub rebalance_method: String,
    pub lookback_days: i64,
    #[serde(default)]
    pub config_hash: String,
    #[serde(default)]
    pub generated_at: String,
    pub fee_bp: f64,
    pub digits: i64,
    pub symbol_count: usize,
    pub sdt: String,
    pub edt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioStrategy {
    pub strategy_id: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetWeight {
    pub symbol: String,
    pub weight: f64,
}

/// 净值 / 回撤 / 累计收益时间序列（等长，对齐 dates）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavSeries {
    #[serde(default)]
    pub dates: Vec<String>,
    #[serde(default)]
    pub nav: Vec<f64>,
    #[serde(default)]
    pub drawdown: Vec<f64>,
    #[serde(default)]
    pub cum_return: Vec<f64>,
}

/// 多空归因累计曲线（多空 / 多头 / 基准 / 超额四条 leg，键为中文）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareSeries {
    #[serde(default)]
    pub dates: Vec<String>,
    #[serde(default)]
    pub series: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReturn {
    pub symbol: String,
    pub ret: f64,
}

/// 单年的 12 个月度收益（`values[i]` 对应第 `i+1` 月；当月无数据为 `null`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyRow {
    pub year: i64,
    #[serde(default)]
    pub values: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownRow {
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub recover: Option<String>,
    pub depth: f64,
    pub drawdown_days: i64,
    #[serde(default)]
    pub recover_days: Option<i64>,
    #[serde(default)]
    pub new_high_gap: Option<i64>,
}

/// 每日持仓权重矩阵（正=多 / 负=空）。`weights` 是 `symbol -> 每日权重数组`
/// （对齐 `dates`，缺失为 `null`），symbol 键本身就是标的代码，用 Value 原样承载。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Positions {
    #[serde(default)]
    pub dates: Vec<String>,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub weights: Value,
}

/// 组合详情读模型：全部数值由后端 wbt 权重回测引擎重算，与报告同源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioDetail {
    pub meta: PortfolioMeta,
    #[serde(default)]
    pub strategies: Vec<PortfolioStrategy>,
    #[serde(default)]
    pub rebalance_dates: Vec<String>,
    #[serde(default)]
    pub latest_weights: Vec<TargetWeight>,
    #[serde(default)]
    pub latest_weights_at: String,
    /// 全样本核心指标（中文键 → 数值）。
    #[serde(default)]
    pub metrics: Value,
    /// 多空关键指标对比（leg → 指标 map）。
    #[serde(default)]
    pub compare_metrics: Value,
    pub compare: CompareSeries,
    pub nav: NavSeries,
    #[serde(default)]
    pub monthly: Vec<MonthlyRow>,
    #[serde(default)]
    pub symbol_returns: Vec<SymbolReturn>,
    #[serde(default)]
    pub drawdowns: Vec<DrawdownRow>,
    /// 平台 is_good_strategy 判定（history + recent）。
    #[serde(default)]
    pub verdict: Value,
    pub positions: Positions,
    /// 有没有可下载的 HTML 报告；本 CLI 不提供拉取报告的命令（detail 已含同等结构化数据）。
    #[serde(default)]
    pub has_report: bool,
}

/* ---------------- POST /research/portfolios（建组合，写） ---------------- */

/// 建组合回执（202 Accepted，异步：真正生成在后台经 Function Compute 跑）。
/// 请求体由调用方从 stdin 原样转发（跟 `route create`/`problem create` 同型），
/// 故这里只建模响应，不建模请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePortfolioAck {
    pub portfolio_code: String,
    /// 固定 `pending`：已受理，正在后台生成；终态请轮询 `portfolio list` 的 `job_status`。
    pub status: String,
}
