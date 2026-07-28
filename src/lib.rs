//! skz — 胜可知开放平台 client library。
//!
//! 覆盖市场数据只读查询与策略业务写链路（创建研究方向/问题、触发因子挖掘/策略探索）。
//! CLI（`bin/skz.rs`）只是这个 library 的一个调用入口；未来的 MCP server
//! 或其他 Rust 入口可以直接复用它，不依赖 CLI。

pub mod client;
pub mod config;
pub mod credentials;
pub mod error;
pub mod models;
pub mod retry;
pub mod skill;
pub mod token;
pub mod update;

pub use error::{Action, Error, ErrorBody, Kind};
