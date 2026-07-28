//! 因子挖掘单次成果（研究后端 pkg 06，经 `/research/mining/*`）响应类型。
//! 照 skz-client `mining/types.ts` 移植：snake_case，动态指标 map 用 `serde_json::Value`。
//! 注意这是「成果柜」（挖出了什么），与 C# 面 `/strategy/miner/*`「任务台」（进度/状态）分工不同。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/* ---------------- GET /research/mining/runs（run 列表：目录 + 计数 + 派生状态） ------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningRunItem {
    /// 全程墙钟耗时（秒）。
    pub elapsed_s: f64,
    pub retain_rate: f64,
    pub retained: i64,
    pub route_code: String,
    pub route_name: String,
    pub run_id: String,
    pub started_at: Option<String>,
    /// 派生 run 状态：succeeded / no_factors / build_failed。
    pub status: String,
    pub total_candidates: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningRunList {
    #[serde(default)]
    pub items: Vec<MiningRunItem>,
    pub total: i64,
}

/* ---------------- GET /research/mining/{run_id}/overview ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningRoute {
    pub code: String,
    pub compute_engine: String,
    pub create_time: String,
    pub creator: Option<String>,
    pub economic_logic: String,
    pub failure_scenarios: Option<Vec<String>>,
    pub key_inspect: String,
    pub market_mechanism: String,
    pub name: String,
    pub tags: Option<Vec<String>>,
    pub why_effective: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningFunnelStage {
    pub eliminated: i64,
    pub remaining: i64,
    pub stage: String,
    pub step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningEliminationItem {
    pub count: i64,
    /// "retained" | "eliminated"（宽松建模为 String）。
    pub kind: String,
    pub reason: String,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningProblemGroup {
    pub count: i64,
    pub label: String,
    pub prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningKpi {
    pub eliminated: i64,
    pub evaluate_method: String,
    pub problem_count: i64,
    pub retain_rate: f64,
    pub retained: i64,
    pub total_candidates: i64,
    pub total_evaluations: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningOverview {
    #[serde(default)]
    pub elimination_breakdown: Vec<MiningEliminationItem>,
    #[serde(default)]
    pub funnel: Vec<MiningFunnelStage>,
    pub kpi: MiningKpi,
    #[serde(default)]
    pub problem_groups: Vec<MiningProblemGroup>,
    pub route: MiningRoute,
    pub run_dir: String,
    pub run_id: String,
}

/* ---------------- GET /research/mining/{run_id}/factors（这次挖出的因子） --------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningFactorAgg {
    pub best_problem: Option<String>,
    pub best_sharpe: Option<f64>,
    pub mean_sharpe: Option<f64>,
    pub median_sharpe: Option<f64>,
    pub pos_sharpe_ratio: Option<f64>,
    pub problem_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningFactorItem {
    pub agg: MiningFactorAgg,
    pub compute_engine: String,
    pub create_time: String,
    pub description: String,
    pub eval_count: i64,
    pub factor_code: String,
    pub factor_name: String,
    /// 指标 map（动态键）→ Value。
    #[serde(default)]
    pub metrics: Value,
    pub problem_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningFactorList {
    #[serde(default)]
    pub items: Vec<MiningFactorItem>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}
