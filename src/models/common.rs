use serde::{Deserialize, Serialize};

/// 分页响应外壳：`{page, size, total, items}`。翻页由 agent 驱动。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub page: u32,
    pub size: u32,
    pub total: u64,
    pub items: Vec<T>,
}
