//! 常量配置 + `SKZ_BASE_URL` 服务器覆盖 + `SKZ_READ_ONLY` 只读闸。
//!
//! 这里是**行为开关**的唯一来源。凭据仍然只来自凭据文件、绝不读 env（见
//! `credentials.rs`）——两者别混为一谈：读 env 拿 token 会把密钥摊进进程表和
//! 各种 dump，而读 env 拿一个布尔开关没有这个问题。

use crate::error::Error;

pub const DEFAULT_BASE_URL: &str = "https://api.shengkezhi.com/open/v1";
pub const BASE_URL_ENV: &str = "SKZ_BASE_URL";
pub const READ_ONLY_ENV: &str = "SKZ_READ_ONLY";
pub const TIMEOUT_SECONDS: u64 = 30;
pub const USER_AGENT: &str = concat!("skz/", env!("CARGO_PKG_VERSION"));

pub struct Config {
    pub base_url: String,
    pub timeout_secs: u64,
    /// 只读模式：禁掉一切写/触发，请求根本不发出。见 `client::Client::ensure_writable`。
    pub read_only: bool,
}

impl Config {
    /// 未设置 `SKZ_BASE_URL` 时使用生产地址；显式设置后接受任意 HTTP(S) API 根地址。
    pub fn new() -> Result<Self, Error> {
        let value = std::env::var_os(BASE_URL_ENV)
            .map(|raw| {
                raw.into_string()
                    .map_err(|_| Error::Args(format!("{BASE_URL_ENV} 必须是有效的 Unicode URL")))
            })
            .transpose()?;
        let base_url = resolve_base_url(value.as_deref())?;
        Ok(Config {
            base_url,
            timeout_secs: TIMEOUT_SECONDS,
            read_only: read_only_from_env()?,
        })
    }
}

/// 读 `SKZ_READ_ONLY`。**唯一的关闭方式是把变量拿掉**，见 `resolve_read_only`。
pub fn read_only_from_env() -> Result<bool, Error> {
    let value = std::env::var_os(READ_ONLY_ENV)
        .map(|raw| {
            raw.into_string()
                .map_err(|_| Error::Args(format!("{READ_ONLY_ENV} 必须是有效的 Unicode 值")))
        })
        .transpose()?;
    resolve_read_only(value.as_deref())
}

/// `1`/`true`/`on`（大小写不敏感）= 开；unset 或空 = 关；**其余一律报错，包括 `0`/`false`**。
///
/// 不把 `0`/`false` 当"关闭"是有意的。只读闸防的是健忘的 agent，而 agent 撞到报错后的
/// 典型反应就是"改个环境变量再试一次"——`SKZ_READ_ONLY=0 skz promote start ...` 一行就
/// 把闸绕了，且这不需要它起坏心。只认 unset 为关闭，绕过就必须是 `env -u` 这种明显是
/// 蓄意的动作，而不是顺手。
///
/// 顺带这也堵掉了 env 开关最危险的失效模式：值写错（`ture`）不会静默退化成"关闭"。
/// **变量名写错仍然是静默失效**，那个只能靠 `skz auth status` 的 `readOnly` 字段亲眼确认。
fn resolve_read_only(value: Option<&str>) -> Result<bool, Error> {
    let Some(value) = value else {
        return Ok(false);
    };
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => Ok(false),
        "1" | "true" | "on" => Ok(true),
        _ => Err(Error::Args(format!(
            "{READ_ONLY_ENV} 只接受 1/true/on（开启只读）；\
             关闭只读的唯一方式是 unset 掉这个变量，不要设成 {value:?}"
        ))),
    }
}

fn resolve_base_url(value: Option<&str>) -> Result<String, Error> {
    let Some(value) = value else {
        return Ok(DEFAULT_BASE_URL.to_string());
    };
    if value.is_empty() {
        return Err(Error::Args(format!("{BASE_URL_ENV} 不能为空")));
    }
    if value.trim() != value {
        return Err(Error::Args(format!("{BASE_URL_ENV} 首尾不能包含空白字符")));
    }

    let parsed = url::Url::parse(value)
        .map_err(|e| Error::Args(format!("{BASE_URL_ENV} 不是有效 URL：{e}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::Args(format!(
            "{BASE_URL_ENV} 只接受 http 或 https URL"
        )));
    }
    let has_explicit_authority = value.split_once("://").is_some_and(|(scheme, rest)| {
        scheme.eq_ignore_ascii_case(parsed.scheme()) && !rest.starts_with('/')
    });
    if !has_explicit_authority || parsed.host().is_none() {
        return Err(Error::Args(format!("{BASE_URL_ENV} URL 必须包含主机")));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(Error::Args(format!(
            "{BASE_URL_ENV} URL 不能包含用户名或密码"
        )));
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(Error::Args(format!(
            "{BASE_URL_ENV} URL 不能包含 query 或 fragment"
        )));
    }

    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_truthy_and_off_values() {
        for v in ["1", "true", "TRUE", "On", " true "] {
            assert!(resolve_read_only(Some(v)).unwrap(), "{v:?} 应当开启");
        }
        assert!(!resolve_read_only(None).unwrap());
        assert!(!resolve_read_only(Some("")).unwrap());
        assert!(!resolve_read_only(Some("  ")).unwrap());
    }

    /// 关闭只读的唯一方式是 unset。`0`/`false` 报错而不是静默关闭——
    /// 否则 agent 一行 `SKZ_READ_ONLY=0 skz ...` 就把闸绕了。
    #[test]
    fn read_only_falsey_values_are_rejected_not_treated_as_off() {
        for v in ["0", "false", "off", "no", "ture", "yes"] {
            assert!(resolve_read_only(Some(v)).is_err(), "{v:?} 应当报错");
        }
    }

    #[test]
    fn unset_uses_default() {
        assert_eq!(resolve_base_url(None).unwrap(), DEFAULT_BASE_URL);
    }

    #[test]
    fn remote_http_and_https_urls_are_accepted_and_normalized() {
        for (input, expected) in [
            (
                "http://dev.example.com:8080/open/v2",
                "http://dev.example.com:8080/open/v2",
            ),
            (
                "https://api.example.com/open/v2/",
                "https://api.example.com/open/v2",
            ),
            ("http://[::1]:8080///", "http://[::1]:8080"),
        ] {
            assert_eq!(resolve_base_url(Some(input)).unwrap(), expected);
        }
    }

    #[test]
    fn invalid_urls_are_rejected() {
        for value in [
            "",
            " https://api.example.com",
            "api.example.com",
            "ftp://api.example.com",
            "http:///open/v1",
            "https://user@example.com/open/v1",
            "https://api.example.com/open/v1?debug=1",
            "https://api.example.com/open/v1#section",
        ] {
            assert!(
                resolve_base_url(Some(value)).is_err(),
                "should reject {value:?}"
            );
        }
    }
}
