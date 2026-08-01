//! 常量配置 + `SKZ_BASE_URL` 服务器覆盖。

use crate::error::Error;

pub const DEFAULT_BASE_URL: &str = "https://api.shengkezhi.com/open/v1";
pub const BASE_URL_ENV: &str = "SKZ_BASE_URL";
pub const TIMEOUT_SECONDS: u64 = 30;
pub const USER_AGENT: &str = concat!("skz/", env!("CARGO_PKG_VERSION"));

pub struct Config {
    pub base_url: String,
    pub timeout_secs: u64,
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
        })
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
