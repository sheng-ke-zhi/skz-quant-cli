//! 研究问题读侧（研究后端，经 `/research/problems*`）响应类型。
//! 照 skz-client `problem/types.ts` 移植：snake_case。`type` 是 Rust 关键字，字段用 `type_` + serde rename。
//! （创建 `problem create` 走 `/strategy/problems` 面，其类型在 models/strategy.rs。）

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 研究问题时间分段（列表无，详情/meta 有）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSegment {
    pub edt: String,
    pub name: String,
    pub sdt: String,
}

/// 研究问题视图：列表项 = 详情去掉 time_segments，故合并建模、time_segments 可缺省。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemView {
    pub code: String,
    pub dataset: String,
    pub description: String,
    /// 截面问题可能带 domains；时序无 → Option 缺省。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
    pub editable: bool,
    pub freq: String,
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub symbols: Vec<String>,
    /// 后端发 `problem_type`（非 `type`），列表/详情一致。
    pub problem_type: String,
    pub type_label: String,
    /// 详情（GET /research/problems/{code}）才有；列表无。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_segments: Option<Vec<TimeSegment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemList {
    #[serde(default)]
    pub items: Vec<ProblemView>,
    pub total: i64,
}

/// `DELETE /research/problems/{code}` 的 CLI 回执。
///
/// 后端成功信封的 data 为 null；CLI 回显目标 code，避免成功输出只有 null。
#[derive(Debug, Clone, Serialize)]
pub struct ProblemDeleted {
    pub code: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemTypeOption {
    /// "symbols" | "domains"
    pub id_field: String,
    pub id_hint: String,
    pub id_label: String,
    pub label: String,
    pub value: String,
}

/// `GET /research/problems/meta`：建 problem 前发现合法枚举（type/dataset/freq/默认分段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemMeta {
    /// code 生成规则（后端自动生成、前端仅预览）：pattern/prefix_rule/dataset_letter/… 松散透传。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub code_gen: Value,
    #[serde(default)]
    pub dataset_options: Vec<LabeledOption>,
    #[serde(default)]
    pub default_time_segments: Vec<TimeSegment>,
    #[serde(default)]
    pub freq_options: Vec<LabeledOption>,
    /// 创建时所有分段起止日期的服务端上限；旧后端未返回时保持兼容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_time_segment_date: Option<String>,
    #[serde(default)]
    pub problem_types: Vec<ProblemTypeOption>,
    #[serde(default)]
    pub required_segments: Vec<String>,
}
