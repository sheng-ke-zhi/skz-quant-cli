use serde::{Deserialize, Serialize};

use super::Timestamp;

/// 一条“运行”的摘要，用于两个列表端点的 item：
/// `GET /strategy/miner/runs`、`GET /strategy/explore/runs`。
/// 两者字段一致，故共用一个类型。nullable 字段建模为 `Option`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub fc_run_id: String,
    pub route_code: String,
    pub status: String,
    pub status_text: String,
    pub done: bool,
    pub ok: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub result_path: Option<String>,
    pub created_at: Timestamp,
    pub finished_at: Option<Timestamp>,
}

/// 一条“运行”的实时进度，用于 `GET /strategy/explore/{fcRunId}` 与两个 poll 端点
/// （`POST /strategy/miner/poll`、`POST /strategy/explore/poll`）的 item。
///
/// 这些端点较新、各家字段不完全一致（explore-get 无 routeCode、explore-poll 多 problemCode
/// 与两个 list 相比多出 percent/step/message），故除 `fcRunId`/`status` 外一律 `Option`：
/// 宁可把在场的字段原样透传，也别因某端点缺一个字段而整条 exit 6。
/// `percent` 用 `serde_json::Number` 承载，不预设整型或浮点。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProgress {
    pub fc_run_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<Timestamp>,
}

// ── 研究面（/research/*）：snake_case，不加 `rename_all`（后端本就 snake_case）──

/// `GET /research/whoami` 开放平台身份自检（网关注入的 user_id）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoAmI {
    pub user_id: String,
}
