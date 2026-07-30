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
    pub annual_return: Option<f64>,
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
    pub agg: FactorAgg,
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
    /// 时段名 → 指标（中文键）→ Value。
    #[serde(default)]
    pub segments: Value,
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
}

/* ---------------- factors/{name} 软删（DELETE） ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSoftDeleted {
    pub factor_name: String,
    pub is_deleted: bool,
}
