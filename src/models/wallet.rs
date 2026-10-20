//! 钱包概览与 CLI 固定价目。

use std::collections::BTreeMap;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletCash {
    pub account_id: i64,
    pub currency: String,
    pub balance_cent: i64,
    pub frozen_cent: i64,
    pub overdraft_limit_cent: i64,
    pub available_cent: i64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummary {
    pub user_id: i64,
    pub cash: WalletCash,
    #[serde(default)]
    pub purses: Vec<Value>,
    pub total_available_cent: i64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum PaidOperation {
    Mine,
    Explore,
    Refresh,
    #[serde(rename = "portfolio-build")]
    PortfolioBuild,
    #[serde(rename = "portfolio-build-byok")]
    PortfolioBuildByok,
}

impl PaidOperation {
    pub const ALL: [Self; 5] = [
        Self::Mine,
        Self::Explore,
        Self::Refresh,
        Self::PortfolioBuild,
        Self::PortfolioBuildByok,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Mine => "因子挖掘",
            Self::Explore => "策略探索",
            Self::Refresh => "实盘更新",
            Self::PortfolioBuild => "组合 Agent 构建（平台模型）",
            Self::PortfolioBuildByok => "组合 Agent 构建（自有模型，模型服务商费用另计）",
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Self::Mine | Self::Explore | Self::PortfolioBuild | Self::PortfolioBuildByok => "run",
            Self::Refresh => "strategy",
        }
    }

    pub const fn unit_price_cent(self) -> i64 {
        match self {
            Self::Mine => 3_000,
            Self::Explore => 600,
            Self::Refresh => 200,
            // 支付迁移中的初始价；与其它条目一样是 CLI 版本价目，不冒充实时服务端报价。
            Self::PortfolioBuild => 2000,
            Self::PortfolioBuildByok => 1000,
        }
    }

    pub const fn commands(self) -> &'static [&'static str] {
        match self {
            Self::Mine => &["skz mine start"],
            Self::Explore => &["skz explore start"],
            Self::Refresh => &["skz strategy refresh"],
            Self::PortfolioBuild | Self::PortfolioBuildByok => &["skz portfolio build start"],
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPrice {
    pub operation: PaidOperation,
    pub label: &'static str,
    pub unit: &'static str,
    pub unit_price_cent: i64,
    pub commands: &'static [&'static str],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletCosts {
    pub currency: &'static str,
    pub pricing_source: &'static str,
    pub items: Vec<WalletPrice>,
}

impl WalletCosts {
    pub fn current() -> Self {
        Self {
            currency: "CNY",
            pricing_source: "cli",
            items: PaidOperation::ALL
                .into_iter()
                .map(|operation| WalletPrice {
                    operation,
                    label: operation.label(),
                    unit: operation.unit(),
                    unit_price_cent: operation.unit_price_cent(),
                    commands: operation.commands(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletCheck {
    pub operation: PaidOperation,
    pub qty: u32,
    pub unit_price_cent: i64,
    pub required_cent: i64,
    pub available_cent: i64,
    pub affordable: bool,
    pub shortfall_cent: i64,
    pub currency: String,
    pub pricing_source: &'static str,
}
