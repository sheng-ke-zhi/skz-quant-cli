use serde::{Deserialize, Serialize};

/// `POST /strategy/routes`（创建研究方向）的响应：`{ routeCode }`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCreated {
    pub route_code: String,
}

/// `GET /strategy/routes/adopted`（已采用路线列表）的元素：`{ routeCode, name }`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedRoute {
    pub route_code: String,
    pub name: String,
}

/// 触发类端点的确认回执：
/// `POST /strategy/miner/runs` → `{ fcRunId, status, routeCode }`；
/// `POST /strategy/explore`    → `{ fcRunId, status }`（无 routeCode，故 `Option`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerAck {
    pub fc_run_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_code: Option<String>,
}

/// `POST /strategy/problems`（创建研究问题）的响应信封：外部研究后端原样透传的
/// `{ code, msg, data }`，成功时 `code==0` 且 `data.code` 就位。
/// 注意：这不是平台统一错误结构；HTTP 200 也可能 `code!=0`，须由调用方判 code。
#[derive(Debug, Clone, Deserialize)]
pub struct ProblemEnvelope {
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub msg: String,
    #[serde(default)]
    pub data: Option<ProblemData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProblemData {
    /// 后端发的字段名是 `code`（不是 `problemCode`）；研究后端 `CreatedProblem{code}`，
    /// C# `/strategy/problems` 原样透传。读错字段会把成功当失败（exit 6），故锁死为 `code`。
    #[serde(default)]
    pub code: Option<String>,
}

/// 从信封提取出、回给用户的干净结果：`{ problemCode }`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemCreated {
    pub problem_code: String,
}
