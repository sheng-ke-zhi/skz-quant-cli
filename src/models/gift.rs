//! 策略赠予（`/research/gifts`）响应类型：A 发码打包实盘策略，B 凭码在自己库里拿到副本。
//!
//! 赠予的语义是**复制不是转移**：B 领走的副本独立存在，A 事后删除或废弃都不影响它；
//! 反过来在 B 领取之前，A 删/废弃任意一条就整码不可领（后端只在 Redis 存引用、不存快照）。
//!
//! `created_at`/`expires_at` 是事件时刻（后端 `to_rfc3339()` 发 UTC），用 [`Timestamp`]
//! 换算成东八区输出；`gift_code` 是纯 hex 串，不是时间。

use serde::{Deserialize, Serialize};

use super::Timestamp;

/* ---------------- POST /research/gifts、GET /research/gifts ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftView {
    /// 32 位小写十六进制。**它本身就是策略的访问凭证**——谁拿到谁就能领走这些策略的
    /// 完整定义，别把它贴进公开渠道或日志。
    pub gift_code: String,
    /// 码内打包的策略编号（赠予方侧编号）。
    #[serde(default)]
    pub strategy_codes: Vec<String>,
    pub max_claims: u32,
    pub claimed: u32,
    pub ttl_days: u8,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    /// 已失效（赠予方已删除或已废弃）的策略编号；**非空即整码不可领**。
    #[serde(default)]
    pub unavailable_strategy_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftList {
    #[serde(default)]
    pub items: Vec<GiftView>,
}

/* ---------------- DELETE /research/gifts/{gift_code} ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftRevoked {
    pub revoked: bool,
}

/* ---------------- GET /research/gifts/{gift_code}/preview ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPreview {
    pub from_user_id: String,
    #[serde(default)]
    pub items: Vec<GiftPreviewItem>,
    pub remaining_claims: u32,
    pub expires_at: Timestamp,
    /// 整体能不能领：全部策略可用 + 名额未尽 + 不是自己发的码 + 自己还没领过。
    pub claimable: bool,
    /// 已领过时再 `claim` 会原样回放上次结果，不重复拷贝、不再占名额。
    pub already_claimed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPreviewItem {
    pub strategy_code: String,
    #[serde(default)]
    pub description: String,
    pub available: bool,
    /// 不可领取的原因；可领时为 null。
    #[serde(default)]
    pub reason: Option<String>,
}

/* ---------------- POST /research/gifts/{gift_code}/claim ---------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftClaimed {
    pub from_user_id: String,
    #[serde(default)]
    pub items: Vec<GiftClaimItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftClaimItem {
    /// 赠予方侧的原编号。
    pub origin_strategy_code: String,
    /// 落到自己实盘库里的编号；与原编号撞名且内容不同时带 `_G{n}` 后缀。
    /// **后续所有 `skz strategy *` 都要用这个编号**，不是 `origin_strategy_code`。
    pub strategy_code: String,
    /// 本次是否新写入；本库已有内容一致的同名策略时为 false（幂等，不重复建）。
    pub inserted: bool,
    /// 是否因撞名而改了编号。
    pub renamed: bool,
}
