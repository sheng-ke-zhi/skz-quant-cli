//! C# `/strategy/portfolios/builds`：与前端 Agent 组合构建一致的请求及运行状态。
use super::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// 不接受后端为读取历史保留、但新建时会清空的旧表单字段，避免约束静默失效。
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortfolioBuildRequest {
    pub market: String,
    pub goal: String,
    #[serde(default)]
    pub problem_codes: Vec<String>,
    #[serde(default)]
    pub strategy_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_trials: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_explorations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_concurrency: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<serde_json::Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_partial: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_constraints: Option<BTreeMap<String, serde_json::Number>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<PortfolioBuildLlm>,
}

// 不派生 Debug，防止 BYOK 明文 API key 被调试日志输出。
#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum PortfolioBuildLlm {
    Platform {},
    Byok {
        #[serde(rename = "configId", default, skip_serializing_if = "Option::is_none")]
        config_id: Option<String>,
        #[serde(rename = "apiKey", default, skip_serializing_if = "Option::is_none")]
        api_key: Option<String>,
        #[serde(rename = "baseUrl", default, skip_serializing_if = "Option::is_none")]
        base_url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(rename = "saveAs", default, skip_serializing_if = "Option::is_none")]
        save_as: Option<String>,
    },
}

impl PortfolioBuildRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.market.trim().is_empty() || self.goal.trim().is_empty() {
            return Err("market 和 goal 不能为空；方法、调仓规则和品种要求请写入 goal".into());
        }
        // 动态候选、预算及约束值由后端在扣费前校验，不在 CLI 复制服务端目录与值域。
        if let Some(PortfolioBuildLlm::Byok {
            config_id,
            api_key,
            base_url,
            model,
            save_as,
        }) = &self.llm
        {
            if config_id.is_some() {
                if config_id.as_ref().is_some_and(|id| id.trim().is_empty())
                    || api_key.is_some()
                    || base_url.is_some()
                    || model.is_some()
                    || save_as.is_some()
                {
                    return Err("BYOK configId 不得为空或混用明文模型参数".into());
                }
            } else if [api_key, base_url, model]
                .iter()
                .any(|v| v.as_ref().is_none_or(|v| v.trim().is_empty()))
            {
                return Err("BYOK 需提供 configId，或完整 apiKey/baseUrl/model".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioBuildRun {
    pub run_id: String,
    pub status: String,
    #[serde(default)]
    pub started_at: Option<Timestamp>,
    #[serde(default)]
    pub finished_at: Option<Timestamp>,
    #[serde(default)]
    pub read_at: Option<Timestamp>,
    /// 动态结果及后端新增字段原样输出，包括 request、result 和 requirementMismatches。
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}
