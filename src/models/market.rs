use serde::{Deserialize, Serialize};

use super::Timestamp;

/// `GET /market/markets` 的元素。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub market: String,
    pub count: u64,
}

/// `GET /market/symbols` 的 item。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub id: u64,
    pub name: String,
    pub symbol: String,
    pub market: String,
    pub update_at: Timestamp,
}

/// `GET /market/trading-calendar` 的元素。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    pub exchange: String,
    pub cal_date: String,
    pub is_open: bool,
    pub pretrade_date: String,
}

/// `POST /research/market-data/future-contracts/resolve` 的单项合约。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureContractItem {
    pub contract: String,
    pub exchange: String,
    pub index_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureContractResolution {
    pub symbol: String,
    #[serde(default)]
    pub contracts: Vec<FutureContractItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureContractsResolved {
    #[serde(default)]
    pub items: Vec<FutureContractResolution>,
}
