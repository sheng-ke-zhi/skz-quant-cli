//! 与开放平台一致的响应类型。响应默认忽略未知字段（向前兼容）。
//!
//! **时间字段分两类，别混**：
//! - **事件发生时刻**（`create_time`/`created_at`/`started_at`/`update_time`/…）用
//!   [`Timestamp`]，后端发的 UTC 在**输出侧**换算成东八区（见 `timestamp.rs`）。
//! - **日期与区间边界**（`cal_date`/`dates`/`sdt`/`edt`/`dt`/`latest_weight_date`/
//!   `outsample_sdt`/…）仍是 `String` 原样承载。它们是交易日语义，±8h 会整体跨日，
//!   把「7月24日的持仓」读成 25 日。
//!
//! `serde_json::Value` 透传块（`metrics`/`trades`/`kline`/`definition`/`verdict`/…）
//! 一律不做时间换算：`trades` 里的 `kline_key` 内嵌时间，却是要原样回传给
//! `strategy kline` 的路径参数，改写它那根 K 线就再也查不到了。

pub mod common;
pub mod experiment;
pub mod factor;
pub mod gift;
pub mod live;
pub mod market;
pub mod mining;
pub mod portfolio;
pub mod problem;
pub mod research;
pub mod strategy;
pub mod timestamp;
pub mod wallet;

pub use timestamp::Timestamp;
