//! 因子库（研究后端 pkg 05，经 `/research/*`）响应类型。
//! 照 skz-client `factor-management/types.ts` 移植：研究面 snake_case，故不加 `rename_all`；
//! nullable 用 `Option`，可缺省字段加 `#[serde(default)]`。指标 map（中文键/动态键）用
//! `serde_json::Value` 原样承载——后端本就是松散 map，CLI 只透传给 agent。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Timestamp;

/* ---------------- summary（GET /research/factors/summary） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorEngineDist {
    pub engine: String,
    pub engine_full: String,
    pub count: i64,
}

/// 路线按夏普降序的 TOP 因子极简视图（summary 直接下发）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRouteTopFactor {
    pub factor_name: String,
    pub sharpe: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRouteDist {
    pub route_code: String,
    pub route_name: String,
    pub engine: String,
    pub factor_count: i64,
    pub total: i64,
    pub avg_sharpe: Option<f64>,
    #[serde(default)]
    pub top_factors: Vec<FactorRouteTopFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTagDist {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSummary {
    pub total_routes: i64,
    pub total_factors: i64,
    pub deleted_factors: i64,
    pub total_evaluations: i64,
    #[serde(default)]
    pub engine_distribution: Vec<FactorEngineDist>,
    #[serde(default)]
    pub route_distribution: Vec<FactorRouteDist>,
    #[serde(default)]
    pub tag_distribution: Vec<FactorTagDist>,
    pub generated_at: Option<Timestamp>,
}

/* ---------------- factor-routes（GET /research/factor-routes） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRoute {
    pub code: String,
    pub name: String,
    pub compute_engine: String,
    pub key_inspect: String,
    pub economic_logic: String,
    pub why_effective: String,
    pub market_mechanism: String,
    #[serde(default)]
    pub failure_scenarios: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub creator: Option<String>,
    pub create_time: Timestamp,
    #[serde(default)]
    pub gifted_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRoutesResponse {
    #[serde(default)]
    pub items: Vec<FactorRoute>,
    pub total: i64,
}

/* ---------------- factors 列表（GET /research/factors） ---------------- */

/// 单因子跨 problem 的聚合摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAgg {
    pub best_sharpe: Option<f64>,
    pub mean_sharpe: Option<f64>,
    pub median_sharpe: Option<f64>,
    #[serde(default)]
    pub median_calmar: Option<f64>,
    pub pos_sharpe_ratio: Option<f64>,
    pub problem_count: i64,
    #[serde(default)]
    pub best_problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorListItem {
    pub factor_name: String,
    pub factor_code: String,
    pub compute_engine: String,
    pub engine_full: String,
    pub description: String,
    pub creator: Option<String>,
    pub create_time: Timestamp,
    pub route: String,
    pub route_name: String,
    pub is_deleted: bool,
    pub delete_reason: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 跨 problem 聚合均值指标（中文键 map）→ Value 原样承载。
    #[serde(default)]
    pub metrics: Value,
    pub agg: Value,
    #[serde(default)]
    pub gifted_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorList {
    #[serde(default)]
    pub items: Vec<FactorListItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub sampled: i64,
}

/* ---------------- factors/{name} 详情（GET /research/factors/{factor_name}） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTagDetail {
    pub tag: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorEvaluation {
    pub problem: String,
    pub method: String,
    pub status: String,
    #[serde(default)]
    pub sharpe: Option<f64>,
    #[serde(default)]
    pub calmar: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorDetail {
    pub factor_name: String,
    pub factor_code: String,
    pub compute_engine: String,
    pub engine_full: String,
    pub description: String,
    pub creator: Option<String>,
    pub create_time: Timestamp,
    pub route: String,
    pub route_name: String,
    pub is_deleted: bool,
    pub delete_reason: Option<String>,
    #[serde(default)]
    pub tags: Vec<FactorTagDetail>,
    #[serde(default)]
    pub evaluations: Vec<FactorEvaluation>,
    #[serde(default)]
    pub gifted_from: Option<String>,
}

/* ---------------- factors/{name} 软删（DELETE） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSoftDeleted {
    pub factor_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFactorsBody {
    pub factor_names: Vec<String>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorDeleteResult {
    pub factor_name: String,
    pub success: bool,
    pub code: i32,
    pub msg: String,
}

/// 批次处理完成不代表全部成功；必须检查 failed_count 和逐项回执。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorsSoftDeleted {
    pub items: Vec<FactorDeleteResult>,
    pub succeeded_count: usize,
    pub failed_count: usize,
}

/* ---------------- factor-routes/{code} 删路线（DELETE） ---------------- */

/// 路线是**物理删除**（不同于因子的软删），并级联删掉该路线名下的挖掘执行目录。
///
/// 新后端将路线、全部关联因子（含已软删）、标签、评估和汇总在同一事务内物理删除，
/// 随后清理挖掘执行目录。`deleted_factors` 在预演/实删中分别是预计/实际删除数。
/// 旧部署缺少该字段时为 None（JSON null），表示删除数量未知，绝不能解释成 0 或因子可保留。
///
/// `failed_mining_runs` 非空的完整实删回执表示主库已提交、部分执行目录未清理，退出码仍为 0。
/// Agent 须读回、重新预演并取得一次性续作确认；不可自动重试。未知结果不授权续作。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDeleted {
    pub route_code: String,
    /// `dry_run=true` 时恒为 false。
    pub deleted: bool,
    pub dry_run: bool,
    /// 预演时是「将删除」，实删时是「已成功删除」。
    pub mining_runs: usize,
    pub failed_mining_runs: Vec<String>,
    /// None 表示旧/未知响应；不得用 orphaned_factors 代替或推断零删除。
    #[serde(default)]
    pub deleted_factors: Option<i64>,
    /// 兼容旧响应。新版后端恒为 0，不再表示可保留因子的数量。
    pub orphaned_factors: i64,
}
