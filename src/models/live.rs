//! 实盘策略库（研究后端「宇宙 A」，经 `/research/strategies*`）响应类型。
//! 照 skz-client `strategy-management/types.ts` 移植：snake_case。
//! 中文键指标（segments 各时段指标、status_counts、market_distribution 三态、metrics.json）
//! 用 `#[serde(flatten)]` map 或 `serde_json::Value` 原样承载，避免中文标识符且不丢字段。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/* ---------------- GET /research/strategies（实盘库列表） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyListItem {
    pub base_freq: String,
    pub code: String,
    pub description: String,
    #[serde(default)]
    pub factor_count: Option<i64>,
    pub last_heartbeat: Option<String>,
    pub latest_weight_date: Option<String>,
    #[serde(default)]
    pub problem_code: Option<String>,
    #[serde(default)]
    pub problem_name: Option<String>,
    #[serde(default)]
    pub problem_description: Option<String>,
    /// with_metrics=true 时内嵌（中文键 map）；无回测产物为 null。
    #[serde(default)]
    pub metrics: Option<Value>,
    #[serde(default)]
    pub nav_preview: Option<StrategyNav>,
    pub outsample_sdt: Option<String>,
    /// 实盘/暂停/废弃（Chinese enum 值，字符串承载）。
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub weight_type: String,
}

/// 分市场三态分布：market + 中文键（实盘/暂停/废弃）计数 → flatten map。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMarketDist {
    pub market: String,
    #[serde(flatten)]
    pub counts: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyList {
    #[serde(default)]
    pub items: Vec<StrategyListItem>,
    #[serde(default)]
    pub market_distribution: Option<Vec<StrategyMarketDist>>,
    pub page: i64,
    pub page_size: i64,
    /// 三态计数（中文键）→ Value。
    #[serde(default)]
    pub status_counts: Value,
    pub total: i64,
}

/* ---------------- GET /research/strategies/{code}（详情） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecentUpdate {
    pub last_heartbeat: Option<String>,
    pub latest_weight_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDetail {
    pub base_freq: String,
    pub code: String,
    #[serde(default)]
    pub death_time: Option<String>,
    pub description: String,
    pub outsample_sdt: Option<String>,
    pub recent_update: StrategyRecentUpdate,
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub update_time: Option<String>,
    pub weight_type: String,
}

/* ---------------- 各子资源读 ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyNav {
    #[serde(default)]
    pub dates: Vec<String>,
    #[serde(default)]
    pub drawdown: Vec<f64>,
    #[serde(default)]
    pub nav: Vec<f64>,
    pub oos_start: Option<String>,
}

/// 分时段项：ASCII 字段 typed，中文指标键（夏普比率/年化收益/…）flatten 保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySegment {
    pub edt: String,
    pub is_live: bool,
    pub sdt: String,
    pub segment_name: String,
    #[serde(flatten)]
    pub metrics: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySegments {
    #[serde(default)]
    pub items: Vec<StrategySegment>,
}

/// 月度矩阵 / 年度分组均较松（后端 z 矩阵、按指标分组数组），整体用 Value 承载。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPeriodic {
    #[serde(default)]
    pub monthly: Value,
    #[serde(default)]
    pub yearly: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPosition {
    pub dt: String,
    pub symbol: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPositions {
    #[serde(default)]
    pub items: Vec<StrategyPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecentEval {
    #[serde(default)]
    pub history: Option<Value>,
    #[serde(default)]
    pub history_ok: Option<bool>,
    pub is_good: bool,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub recent: Option<Value>,
    #[serde(default)]
    pub recent_ok: Option<bool>,
}

/// live 关键交易复盘 items 形状较松（回测透传）→ Value 项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradesResponse {
    #[serde(default)]
    pub items: Vec<Value>,
}

/* ---------------- 写回执 ---------------- */

/// `PATCH /strategy/realtime/strategies/{code}/status` 回执。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdated {
    pub code: String,
    pub status: String,
}

/// tag 增删回执。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUpdated {
    pub code: String,
    pub tag: String,
}
