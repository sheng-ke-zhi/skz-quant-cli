//! 与开放平台一致的响应类型。响应默认忽略未知字段（向前兼容）；
//! 时间字段以 `String` 原样承载、不解析（不解析我们不计算的数据）。

pub mod common;
pub mod experiment;
pub mod factor;
pub mod live;
pub mod market;
pub mod mining;
pub mod portfolio;
pub mod problem;
pub mod research;
pub mod strategy;
