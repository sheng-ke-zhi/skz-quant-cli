//! 贯穿 library 与 binary 的强类型错误 + 动作导向退出码 + JSON 错误体。
//!
//! 退出码按 agent 的**下一步动作**分类（粗），JSON body 承载精确来源（细）。
//! `action` 与退出码 1:1；agent 的控制流应只 branch 在 `action`。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Args,
    Config,
    Api,
    Network,
    Internal,
    /// `update` shell 出的 pipx/uv 升级命令失败。跟 `Network` 分开:那个专指 ureq 的
    /// HTTP 传输层,这里可能是"pipx 不在 PATH 上""venv 坏了"之类本地问题，跟网络无关，
    /// 即便粗分类（Action）恰好同样落在 RetryLater。
    Subprocess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    FixParams,
    FixAuth,
    GiveUp,
    RetryLater,
    Internal,
    /// 该任务已在运行（HTTP 409）：别重复触发，去 `*-runs --status active`
    /// 找到在跑的 run 并轮询它。语义既非 give_up（当天弃）也非 retry_later（盲重试），
    /// 故独立一码，让 agent 只看退出码就能分流。
    CheckExisting,
}

impl Action {
    /// action 与退出码 1:1。
    pub fn exit_code(self) -> i32 {
        match self {
            Action::FixParams => 2,
            Action::FixAuth => 3,
            Action::GiveUp => 4,
            Action::RetryLater => 5,
            Action::Internal => 6,
            Action::CheckExisting => 7,
        }
    }
}

/// 序列化进 `{"error": <body>}` 的错误对象。
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub kind: Kind,
    pub action: Action,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    #[serde(rename = "retryAfterMs", skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 本地参数校验失败（fix_params / exit 2）。
    #[error("{0}")]
    Args(String),
    /// 无 credentials 文件（fix_auth / exit 3，带 remediation）。
    #[error("缺少 credentials")]
    MissingCredentials,
    /// 平台返回的结构化错误（action 由 status + code 分类）。
    #[error("api error {status}")]
    Api {
        status: u16,
        code: String,
        title: String,
        retry_after_ms: Option<u64>,
    },
    /// 研究后端业务信封错误 `{code,msg,data:null}`（骑非 2xx，`HTTP == code 前三位`）。
    /// action 由 code 家族 + `is_read` 分类：42201 读=净值未算完(retry)、写=参数非法(fix_params)。
    #[error("research error {code}")]
    Research {
        http_status: u16,
        code: i64,
        msg: String,
        is_read: bool,
    },
    /// 传输层错误：连接失败 / 超时 / 重置（retry_later / exit 5）。
    #[error("network error: {0}")]
    Network(String),
    /// **写**命令的传输层错误：结果未知（check_existing / **exit 7**）。
    ///
    /// 读超时重来一次即可；写超时**结果未知**——请求可能已经落库（真机实测过
    /// `route create` 超时后确实没写进去，但那是查回来才知道的，不能假设）。
    /// 盲重试会重复扣费/建重复资源，故归 `CheckExisting`（"别重触发，先查现有状态"），
    /// 与 409 同构，并带 `verify_with` 指明用哪条读命令查证。
    #[error("network error (write, outcome unknown): {msg}")]
    WriteNetwork {
        msg: String,
        /// 用来查证这次写到底成没成的读命令，如 `skz factor-routes list`。
        verify_with: &'static str,
    },
    /// 内部 / 协议错误：无法解析、未知码、非预期状态（internal / exit 6）。
    #[error("internal error: {0}")]
    Internal(String),
    /// `update` shell 出的 pipx/uv 升级命令失败（retry_later / exit 5）：进程非零退出，
    /// 或压根起不来。不复用 `WriteNetwork`——那个的 `verify_with`/"结果未知"语义专门对应
    /// 业务写的幂等顾虑，重跑 `skz update` 没有这个顾虑，盲重试完全安全。
    #[error("upgrade via {channel} failed: {message}")]
    UpgradeFailed {
        channel: &'static str,
        command: &'static str,
        message: String,
    },
}

impl Error {
    /// 把写命令的传输层错误升级成「结果未知」语义（带查证指引）。
    /// 只转 `Network`——已经拿到 HTTP 响应的错误（`Api`/`Research`）结果是确定的，不必查证。
    pub fn into_write_unknown(self, verify_with: &'static str) -> Self {
        match self {
            Error::Network(msg) => Error::WriteNetwork { msg, verify_with },
            other => other,
        }
    }

    pub fn to_body(&self) -> ErrorBody {
        match self {
            Error::Args(msg) => ErrorBody {
                kind: Kind::Args,
                action: Action::FixParams,
                message: msg.clone(),
                status: None,
                code: None,
                retryable: None,
                retry_after_ms: None,
                remediation: None,
            },
            Error::MissingCredentials => ErrorBody {
                kind: Kind::Config,
                action: Action::FixAuth,
                message: "缺少 credentials".to_string(),
                status: None,
                code: None,
                retryable: None,
                retry_after_ms: None,
                remediation: Some(missing_credentials_remediation()),
            },
            Error::Api {
                status,
                code,
                title,
                retry_after_ms,
            } => {
                let action = classify_api(*status, code);
                ErrorBody {
                    kind: Kind::Api,
                    action,
                    message: title.clone(),
                    status: Some(*status),
                    code: if code.is_empty() {
                        None
                    } else {
                        Some(code.clone())
                    },
                    retryable: Some(action == Action::RetryLater),
                    retry_after_ms: *retry_after_ms,
                    // 402 余额不足：give_up，但补一条指向充值的 remediation，
                    // 免得 agent 把它当“配额超限、当天别试”误传给用户。
                    remediation: if *status == 402 {
                        Some(insufficient_balance_remediation())
                    } else {
                        None
                    },
                }
            }
            Error::Research {
                http_status,
                code,
                msg,
                is_read,
            } => {
                let action = classify_research_code(*code, *is_read);
                ErrorBody {
                    kind: Kind::Api,
                    action,
                    message: msg.clone(),
                    status: Some(*http_status),
                    code: Some(code.to_string()),
                    retryable: Some(action == Action::RetryLater),
                    retry_after_ms: None,
                    remediation: None,
                }
            }
            Error::Network(msg) => ErrorBody {
                kind: Kind::Network,
                action: Action::RetryLater,
                message: msg.clone(),
                status: None,
                code: None,
                retryable: Some(true),
                retry_after_ms: None,
                remediation: None,
            },
            Error::WriteNetwork { msg, verify_with } => ErrorBody {
                kind: Kind::Network,
                // 用 CheckExisting(exit 7) 而不是 RetryLater(exit 5)：契约要求 agent
                // 「照 action 分支」，而 retry_later 的字面意思就是重发——写超时恰恰不能重发。
                // CheckExisting 的既有语义正是「别重触发，先去查现有状态」，与这里完全同构
                // （409 是"已经在跑了"，这里是"可能已经写进去了"，下一步动作相同）。
                // 真机评测暴露过：写超时若给 retry_later + retryable:false，两个机读字段
                // 自相矛盾，只按 action 分支的 agent 会盲重试、重复扣费。
                action: Action::CheckExisting,
                message: msg.clone(),
                status: None,
                code: None,
                retryable: Some(false),
                retry_after_ms: None,
                remediation: Some(serde_json::json!({
                    "howTo": format!(
                        "写请求结果未知（可能已落库）。先跑 `{verify_with}` 查证：\
                         确认没写进去才可重来一次；已写进去就往下走，别重复触发。"
                    ),
                    "verifyWith": verify_with,
                })),
            },
            Error::Internal(msg) => ErrorBody {
                kind: Kind::Internal,
                action: Action::Internal,
                message: msg.clone(),
                status: None,
                code: None,
                retryable: None,
                retry_after_ms: None,
                remediation: None,
            },
            Error::UpgradeFailed {
                channel,
                command,
                message,
            } => ErrorBody {
                kind: Kind::Subprocess,
                action: Action::RetryLater,
                message: message.clone(),
                status: None,
                code: None,
                retryable: Some(true),
                retry_after_ms: None,
                remediation: Some(upgrade_failed_remediation(channel, command)),
            },
        }
    }

    pub fn exit_code(&self) -> i32 {
        self.to_body().action.exit_code()
    }
}

/// 分类优先看 errorCode（两个 429 语义相反，只能按 code 分），无命中再按 HTTP status。
///
/// status 分支要能兜住**没有命名 errorCode 的响应**：一是策略业务触发端点的
/// 402/409/503（平台确未给 errorCode，见错误码页）；二是边缘网关/代理在 token 过期或
/// 限流时可能回的裸 401/403/429（HTML 或空体，无 errorCode）。按 status 归到正确动作，
/// 别让它们掉进 `internal`(exit 6) 让 agent 误当代码 bug：
/// 400/404/422→fix_params、401/403→fix_auth、402→give_up、409→check_existing、
/// 429/5xx(含 503)→retry_later。
fn classify_api(status: u16, code: &str) -> Action {
    match code {
        "RATE_LIMITED" => Action::RetryLater,
        "QUOTA_EXCEEDED" | "TOO_MANY_IPS" => Action::GiveUp,
        "MISSING_API_KEY" | "INVALID_API_KEY" | "INSUFFICIENT_SCOPE" | "IP_NOT_ALLOWED" => {
            Action::FixAuth
        }
        _ => match status {
            // 404/422 同归参数错：平台面(`/strategy/*`)的 404 是坏 id/路径；422 是后端业务校验
            // 不过（如 problem create 缺必需时间段——C# 把 Rust 的 `{code:42201}` 原样透传上来，
            // 没有 errorCode 字段，只能靠 status 认）。少了这两臂会误判成 internal/exit 6，
            // agent 会以为是内部故障而放弃，实际上改参数就能过。
            // 413 同归参数错：请求体超上限（如策略 TOML > 1 MiB）是"把输入改小就能过"，
            // 落到下面的 _ => Internal 会让 agent 当成内部故障放弃。本地预检通常先拦一道，
            // 这里是防御性兜底——上限值只有后端知道，本地那份是抄来的、可能过期。
            400 | 404 | 413 | 422 => Action::FixParams,
            401 | 403 => Action::FixAuth,
            402 => Action::GiveUp,
            409 => Action::CheckExisting,
            429 | 500..=599 => Action::RetryLater,
            _ => Action::Internal,
        },
    }
}

/// 研究信封数值 code 分类。code 前三位 == HTTP status（后端不变量），故按家族分流。
/// 422 是唯一需要读/写分叉的：读→NotReady(净值未算完)=retry_later；写→参数非法=fix_params
/// （42201 被后端重载:数据未就绪 vs 非法配置，仅 msg 可辨,故按命令读写属性兜底）。
/// 40400(库未生成/详情不存在)：LIST 已被后端软化成 200-空,到这里的是 detail 坏 id → fix_params。
fn classify_research_code(code: i64, is_read: bool) -> Action {
    match code / 100 {
        // 413：请求体超上限，改小输入就能过（同 classify_api 的理由）。
        400 | 404 | 413 => Action::FixParams,
        401 | 403 => Action::FixAuth,
        402 => Action::GiveUp,
        409 => Action::CheckExisting,
        422 => {
            if is_read {
                Action::RetryLater
            } else {
                Action::FixParams
            }
        }
        429 | 500..=599 => Action::RetryLater,
        _ => Action::Internal,
    }
}

fn missing_credentials_remediation() -> serde_json::Value {
    serde_json::json!({
        "howTo": "没 key 就去胜可知开放平台一键生成，然后把 key 经 stdin 交给 `skz auth set`",
        "notes": [
            "一把 Key/账户，刷新即失效",
            "勿多设备共享；每日 5 个 IP 上限"
        ]
    })
}

fn insufficient_balance_remediation() -> serde_json::Value {
    serde_json::json!({
        "howTo": "账户余额不足或扣费被拒：去胜可知开放平台充值后再重试触发",
        "notes": [
            "这不是配额超限（非 0 点重置）；充值即可恢复",
            "别自动重试触发——触发即扣费"
        ]
    })
}

fn upgrade_failed_remediation(channel: &str, command: &str) -> serde_json::Value {
    serde_json::json!({
        "howTo": format!(
            "`skz update` 执行的 {channel} 命令没成功。重跑一次通常安全\
             （这步没有幂等顾虑，不是业务写）；若持续失败，直接手动跑 `{command}`\
             看完整报错，或参考 README 重装。"
        )
    })
}
