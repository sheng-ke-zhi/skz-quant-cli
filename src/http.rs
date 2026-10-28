//! 业务响应读取与所有联网入口共用的 TLS 信任策略。

use crate::error::Error;

// shortcut: 暂定 64 MiB，合法业务响应仍超限时根据实际大小调整。
const RESPONSE_LIMIT: u64 = 64 * 1024 * 1024;

pub(crate) fn tls_config() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build()
}

pub(crate) fn read_body(body: &mut ureq::Body) -> Result<String, Error> {
    read_body_with_limit(body, RESPONSE_LIMIT)
}

fn read_body_with_limit(body: &mut ureq::Body, limit: u64) -> Result<String, Error> {
    // ureq 在恰好读满 limit 时也报超限，多读一字节才能接受边界值。
    let bytes = body
        .with_config()
        .limit(limit + 1)
        .read_to_vec()
        .map_err(|e| match e {
            ureq::Error::BodyExceedsLimit(_) => response_too_large(limit),
            other => Error::Network(format!("读取响应失败: {other}")),
        })?;
    if bytes.len() as u64 > limit {
        return Err(response_too_large(limit));
    }
    Ok(String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

fn response_too_large(limit: u64) -> Error {
    Error::ResponseTooLarge(format!(
        "响应超过客户端读取上限 {limit} 字节；重复请求不会解决，请联系维护者调整上限"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, retry::with_retry};
    use httpmock::prelude::*;

    #[test]
    fn response_limit_boundaries_and_retry_policy() {
        let server = MockServer::start();
        for size in [31, 32, 33, 34] {
            let mock = server.mock(|when, then| {
                when.path(format!("/{size}"));
                then.body("x".repeat(size));
            });
            let result = with_retry(|| {
                let mut response = ureq::get(server.url(format!("/{size}"))).call().unwrap();
                read_body_with_limit(response.body_mut(), 32)
            });
            mock.assert_calls(1);
            if size <= 32 {
                assert_eq!(result.unwrap().len(), size);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.to_body().action, Action::Internal);
                assert!(error.to_body().message.contains("32 字节"));
                assert_eq!(
                    error
                        .into_write_unknown("skz portfolio list")
                        .to_body()
                        .action,
                    Action::CheckExisting
                );
            }
        }
    }
}
