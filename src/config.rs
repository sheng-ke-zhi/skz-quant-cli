//! 常量配置 + 测试用 `--base-url`（loopback-only）。不读任何 SKZ_* 环境变量。

use crate::error::Error;

pub const DEFAULT_BASE_URL: &str = "https://api.shengkezhi.com/open/v1";
pub const TIMEOUT_SECONDS: u64 = 30;
pub const USER_AGENT: &str = concat!("skz/", env!("CARGO_PKG_VERSION"));

pub struct Config {
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Config {
    /// `base_url_override` 只来自测试用隐藏 flag `--base-url`，且必须是 loopback。
    /// 生产用硬编码的 HTTPS 常量。
    pub fn new(base_url_override: Option<String>) -> Result<Self, Error> {
        let base_url = match base_url_override {
            Some(u) => {
                validate_loopback_base_url(&u)?;
                u
            }
            None => DEFAULT_BASE_URL.to_string(),
        };
        Ok(Config {
            base_url,
            timeout_secs: TIMEOUT_SECONDS,
        })
    }
}

/// `--base-url` 仅接受 loopback host；loopback 允许 http（供本地 mock）。
fn validate_loopback_base_url(u: &str) -> Result<(), Error> {
    let after = u
        .strip_prefix("http://")
        .or_else(|| u.strip_prefix("https://"))
        .ok_or_else(|| Error::Args("--base-url 必须是 http(s) URL".to_string()))?;
    // IPv6 字面量带方括号（`[::1]:8080`），host 取到 `]` 为止——不能按 `:` 切，
    // 否则 `[::1]` 内部的冒号会把 host 切成 `[`，让 `[::1]` 这条分支形同虚设。
    let host = if after.starts_with('[') {
        match after.find(']') {
            Some(end) => &after[..=end],
            None => after,
        }
    } else {
        after.split(['/', ':']).next().unwrap_or("")
    };
    match host {
        "127.0.0.1" | "localhost" | "[::1]" => Ok(()),
        _ => Err(Error::Args(
            "--base-url 仅接受 loopback（127.0.0.1 / localhost / [::1]）".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_hosts_accepted_incl_ipv6() {
        for u in [
            "http://127.0.0.1:8080",
            "http://localhost:3000/x",
            "https://localhost",
            "http://[::1]:8080",
            "http://[::1]",
        ] {
            assert!(validate_loopback_base_url(u).is_ok(), "should accept {u}");
        }
    }

    #[test]
    fn non_loopback_rejected() {
        for u in [
            "http://evil.example.com",
            "https://api.shengkezhi.com/open/v1",
            "http://127.0.0.1.evil.com",
            "ftp://127.0.0.1",
        ] {
            assert!(validate_loopback_base_url(u).is_err(), "should reject {u}");
        }
    }
}
