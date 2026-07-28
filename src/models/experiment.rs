//! 策略研究执行结果（研究后端 experiments/review/promotions，经 `/research/*`）响应类型。
//! 照 skz-client `experiments/types.ts` 移植：snake_case。松散/回测透传字段用 `serde_json::Value`；
//! review-matrix 的中文指标键用 `#[serde(flatten)]` map 保留。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/* ---------------- GET /research/experiments（列表） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentListItem {
    #[serde(default)]
    pub dataset: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub elapsed_s: Option<f64>,
    pub failed: Option<i64>,
    pub freq: Option<String>,
    pub id: String,
    #[serde(default)]
    pub n_backtests: Option<i64>,
    #[serde(default)]
    pub n_strategies: Option<i64>,
    pub pass_rate: Option<f64>,
    pub passed: Option<i64>,
    pub problem_code: Option<String>,
    pub problem_name: Option<String>,
    #[serde(default)]
    pub problem_type: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    pub run_at: Option<String>,
    pub scanned: Option<i64>,
    pub skipped: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    pub strategy_count: i64,
    pub symbols_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentList {
    #[serde(default)]
    pub items: Vec<ExperimentListItem>,
    pub total: i64,
}

/* ---------------- GET /research/experiments/{id}（顶层 { overview }） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTimeSegment {
    #[serde(default)]
    pub edt: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sdt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentOverview {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dataset: Option<String>,
    #[serde(default)]
    pub elapsed_s: Option<f64>,
    #[serde(default)]
    pub errors: Option<Value>,
    #[serde(default)]
    pub failed: Option<i64>,
    #[serde(default)]
    pub freq: Option<String>,
    #[serde(default)]
    pub model_configs_used: Option<Value>,
    #[serde(default)]
    pub n_backtests: Option<i64>,
    #[serde(default)]
    pub n_strategies: Option<i64>,
    #[serde(default)]
    pub passed: Option<i64>,
    #[serde(default)]
    pub pass_rate: Option<f64>,
    #[serde(default)]
    pub problem_code: Option<String>,
    #[serde(default)]
    pub problem_name: Option<String>,
    #[serde(default)]
    pub problem_type: Option<String>,
    #[serde(default)]
    pub review_fn: Option<String>,
    #[serde(default)]
    pub run_at: Option<String>,
    #[serde(default)]
    pub scanned: Option<i64>,
    #[serde(default)]
    pub skipped: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    #[serde(default)]
    pub time_segments: Option<Vec<ExperimentTimeSegment>>,
    #[serde(default)]
    pub total_elapsed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentDetail {
    pub overview: ExperimentOverview,
}

/* ---------------- GET /research/experiments/{id}/strategies（候选） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStrategyItem {
    pub code: String,
    #[serde(default)]
    pub end_date: Option<String>,
    pub factor_count: i64,
    /// 中文键指标 map → Value。
    #[serde(default)]
    pub metrics: Value,
    pub model: Option<String>,
    pub passed: bool,
    pub route: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub symbol_count: Option<i64>,
    #[serde(default)]
    pub verdict: Option<Value>,
    #[serde(default)]
    pub weight_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentStrategies {
    #[serde(default)]
    pub items: Vec<ExperimentStrategyItem>,
    pub total: i64,
}

/* ---------------- GET /research/experiments/{id}/review-matrix ---------------- */

/// review-matrix 项：ASCII 字段 typed，中文指标键 flatten 保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMatrixItem {
    pub edt: String,
    pub sdt: String,
    pub segment_name: String,
    pub strategy: String,
    #[serde(flatten)]
    pub metrics: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMatrix {
    #[serde(default)]
    pub items: Vec<ReviewMatrixItem>,
    #[serde(default)]
    pub segments: Vec<String>,
}

/* ---------------- DELETE /research/experiments/{id}/strategies/{code} -------- */

/// 删除探索候选回执。仅删除候选回测产物，实验汇总和策略定义仍保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDeleted {
    pub experiment_id: String,
    pub strategy_code: String,
    pub deleted: bool,
}

/* ---------------- promote / GET /research/promotions/{id}（PromotionView） ------- */

/// promote 同步写库 + 触发 FC 实盘部署，立即返回 status=running；终态靠轮询本资源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promotion {
    pub promotion_id: String,
    pub experiment_id: String,
    pub strategy_code: String,
    /// running / succeeded / failed。
    pub status: String,
    pub phase: String,
    pub registered: bool,
    pub lifecycle: Option<String>,
    /// FC 实盘回调透传（未回调为 null）。
    #[serde(default)]
    pub realtime: Option<Value>,
    /// 失败详情（成功为 null）。
    #[serde(default)]
    pub error: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}
