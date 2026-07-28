//! 只读请求的有限重试：最多 3 次尝试；`Retry-After` 优先，否则带抖动指数退避。
//! 重试判据 = `action == RetryLater`（统一覆盖 RATE_LIMITED / 5xx / 临时网络）。

use std::thread::sleep;
use std::time::Duration;

use crate::error::{Action, Error};

const MAX_ATTEMPTS: u32 = 3;

pub fn with_retry<T, F>(mut f: F) -> Result<T, Error>
where
    F: FnMut() -> Result<T, Error>,
{
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                let body = e.to_body();
                // `WriteNetwork` 的 action 也是 RetryLater（agent 确实该稍后再处理），
                // 但它的结果**未知**——重发可能重复扣费/建重复资源。写命令本就不套
                // with_retry（不变量 1），这里再挡一道：将来谁误套上去也不会重发。
                let retryable = body.action == Action::RetryLater && body.retryable != Some(false);
                if !retryable || attempt >= MAX_ATTEMPTS {
                    return Err(e);
                }
                let delay_ms = match body.retry_after_ms {
                    Some(ms) => ms,
                    None => {
                        let base = 200u64.saturating_mul(1u64 << (attempt - 1));
                        base + fastrand::u64(0..=base / 2)
                    }
                };
                sleep(Duration::from_millis(delay_ms));
            }
        }
    }
}
