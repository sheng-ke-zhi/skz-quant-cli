//! 通用投研资产赠予（`/research/gifts`）响应类型。

use super::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiftAssetType {
    Problem,
    FactorRoute,
    Strategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiftClaimStatus {
    New,
    Pending,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftClaimItem {
    pub origin_code: String,
    pub target_code: String,
    pub inserted: bool,
    pub renamed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftClaimRecord {
    pub gift_code: String,
    pub recipient_user_id: String,
    pub from_user_id: String,
    pub asset_type: GiftAssetType,
    pub claimed_at: Timestamp,
    #[serde(default)]
    pub items: Vec<GiftClaimItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftView {
    pub gift_code: String,
    pub asset_type: GiftAssetType,
    #[serde(default)]
    pub asset_codes: Vec<String>,
    pub max_claims: u32,
    pub claimed: u32,
    pub ttl_days: u8,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub status: String,
    #[serde(default)]
    pub unavailable_asset_codes: Vec<String>,
    #[serde(default)]
    pub claim_records: Vec<GiftClaimRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftList {
    #[serde(default)]
    pub items: Vec<GiftView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedGiftList {
    #[serde(default)]
    pub items: Vec<GiftClaimRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftRevoked {
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPreview {
    pub asset_type: GiftAssetType,
    pub from_user_id: String,
    #[serde(default)]
    pub items: Vec<GiftPreviewItem>,
    pub remaining_claims: u32,
    pub expires_at: Option<Timestamp>,
    pub claimable: bool,
    pub already_claimed: bool,
    pub claim_status: GiftClaimStatus,
    pub resumable: bool,
    #[serde(default)]
    pub claim_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPreviewItem {
    pub origin_code: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub available: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftClaimed {
    pub asset_type: GiftAssetType,
    pub from_user_id: String,
    #[serde(default)]
    pub items: Vec<GiftClaimItem>,
}
