//! CLI OAuth：PKCE + loopback 回调。凭据保存完成前绝不展示成功页。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::credentials::{self, IdentityInfo, OAuthCredential, WritePolicy};
use crate::error::Error;

const CLIENT_ID: &str = "skz-cli";
const SCOPES: &str = "market:read research:mining:write strategy:write";

#[derive(Deserialize)]
struct Discovery {
    authorization_endpoint: String,
    token_endpoint: String,
}

// 不派生 Debug；错误也不透传响应正文，防止异常服务把凭据反射到日志。
#[derive(Deserialize)]
pub(crate) struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    token_type: String,
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn auth_error(message: &str) -> Error {
    Error::Api {
        status: 401,
        code: "OAUTH_LOGIN_REQUIRED".into(),
        title: message.into(),
        retry_after_ms: None,
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(crate::http::tls_config())
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

fn checked_url(value: &str) -> Result<Url, Error> {
    let url = Url::parse(value).map_err(|_| Error::Args("OAuth 地址无效".into()))?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && local)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Args(
            "OAuth 地址须为 HTTPS（本机允许 HTTP），且不能含凭据、query 或 fragment".into(),
        ));
    }
    Ok(url)
}

fn api_origin(base_url: &str) -> Result<Url, Error> {
    let url = checked_url(base_url)?;
    url.join("/")
        .map_err(|_| Error::Args("OAuth API 地址无效".into()))
}

fn token_endpoint(value: &str, base_url: &str) -> Result<Url, Error> {
    let url = checked_url(value)?;
    if url.origin() != api_origin(base_url)?.origin() {
        return Err(Error::Args("OAuth token 地址必须与 API 同源".into()));
    }
    Ok(url)
}

fn authorization_endpoint(value: &str, base_url: &str) -> Result<Url, Error> {
    let url = checked_url(value)?;
    // 生产发现配置仍指向官网旧地址；量化 SPA 已部署到 quant 子域。
    // 只兼容这个已知旧地址，其他环境与自定义部署继续使用发现结果。
    if api_origin(base_url)?.origin() == api_origin(crate::config::DEFAULT_BASE_URL)?.origin()
        && url.as_str() == "https://shengkezhi.com/oauth/authorize"
    {
        return checked_url(crate::config::DEFAULT_OAUTH_AUTHORIZATION_URL);
    }
    Ok(url)
}

fn discover(base_url: &str) -> Result<Discovery, Error> {
    let origin = api_origin(base_url)?;
    let url = origin.join(".well-known/openid-configuration").unwrap();
    let mut response = agent()
        .get(url.as_str())
        .call()
        .map_err(|_| Error::Network("读取 OAuth 配置失败".into()))?;
    if response.status().as_u16() != 200 {
        return Err(Error::Args("服务器未提供 OAuth 配置发现接口".into()));
    }
    let body = response
        .body_mut()
        .with_config()
        .limit(65536)
        .read_to_string()
        .map_err(|_| Error::Internal("读取 OAuth 配置响应失败".into()))?;
    let mut discovery: Discovery =
        serde_json::from_str(&body).map_err(|_| Error::Internal("OAuth 配置响应无效".into()))?;
    discovery.authorization_endpoint =
        authorization_endpoint(&discovery.authorization_endpoint, base_url)?.to_string();
    // 网关后的 Identity 可能在发现文档里公布容器地址。第一方换码接口固定为
    // /oauth/token，始终走用户配置的公网 API origin，避免把凭据发给发现文档中的其他主机。
    discovery.token_endpoint = origin.join("oauth/token").unwrap().to_string();
    Ok(discovery)
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn random_secret() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Internal("生成 OAuth 随机数失败".into()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn exchange(endpoint: &str, fields: &[(&str, &str)]) -> Result<Tokens, Error> {
    // 授权码和 refresh token 都是一次性的，禁止自动重试及重定向。
    let mut response = agent()
        .post(endpoint)
        .send_form(fields.iter().copied())
        .map_err(|_| auth_error("OAuth 换取或刷新凭据结果不确定，请重新登录"))?;
    if response.status().as_u16() != 200 {
        return Err(auth_error("OAuth 换取或刷新凭据失败，请重新登录"));
    }
    let body = response
        .body_mut()
        .with_config()
        .limit(65536)
        .read_to_string()
        .map_err(|_| auth_error("OAuth token 响应读取失败，请重新登录"))?;
    let tokens: Tokens =
        serde_json::from_str(&body).map_err(|_| auth_error("OAuth token 响应无效，请重新登录"))?;
    if !tokens.token_type.eq_ignore_ascii_case("bearer")
        || tokens.expires_in == 0
        || tokens.access_token.trim().is_empty()
        || tokens.refresh_token.trim().is_empty()
    {
        return Err(auth_error("OAuth token 响应不完整，请重新登录"));
    }
    Ok(tokens)
}

pub(crate) fn refresh(
    endpoint: &str,
    refresh_token: &str,
    base_url: &str,
) -> Result<Tokens, Error> {
    token_endpoint(endpoint, base_url)?;
    exchange(
        endpoint,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
        ],
    )
}

pub struct LoginOptions<'a> {
    pub identity: &'a str,
    pub account: Option<&'a str>,
    pub write_policy: WritePolicy,
    pub replace: bool,
    pub no_browser: bool,
    pub timeout_secs: u64,
    pub base_url: &'a str,
}

pub fn login(options: LoginOptions<'_>) -> Result<IdentityInfo, Error> {
    credentials::check_login(options.identity, options.account, options.replace)?;
    if !(1..=600).contains(&options.timeout_secs) {
        return Err(Error::Args("OAuth 等待时间须为 1～600 秒".into()));
    }
    let discovery = discover(options.base_url)?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|_| Error::Internal("无法监听 OAuth 本机回调".into()))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| Error::Internal("设置 OAuth 监听失败".into()))?;
    let redirect = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr().unwrap().port()
    );
    let verifier = random_secret()?;
    let state = random_secret()?;
    let challenge = pkce_challenge(&verifier);
    let mut authorize = checked_url(&discovery.authorization_endpoint)?;
    authorize.query_pairs_mut().extend_pairs([
        ("client_id", CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", &redirect),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("scope", SCOPES),
        ("state", &state),
    ]);
    eprintln!("请在运行 CLI 的电脑上打开链接并完成授权：\n{authorize}");
    if !options.no_browser {
        open_browser(authorize.as_str());
    }
    let deadline = Instant::now() + Duration::from_secs(options.timeout_secs);
    let (mut stream, code) = wait_callback(&listener, &state, deadline)?;
    let result = (|| {
        let tokens = exchange(
            &discovery.token_endpoint,
            &[
                ("grant_type", "authorization_code"),
                ("client_id", CLIENT_ID),
                ("code", &code),
                ("redirect_uri", &redirect),
                ("code_verifier", &verifier),
            ],
        )?;
        let expires_at = now().saturating_add(tokens.expires_in);
        credentials::save_oauth(
            options.identity,
            options.account,
            options.write_policy,
            options.replace,
            tokens.access_token,
            OAuthCredential {
                refresh_token: tokens.refresh_token,
                expires_at,
                token_endpoint: discovery.token_endpoint,
                base_url: options.base_url.into(),
                refresh_pending: false,
            },
        )
    })();
    match result {
        Ok(info) => {
            let success = checked_url(&discovery.authorization_endpoint)?
                .join("success")
                .unwrap();
            // 浏览器已经完成回调；断开连接不应把已保存的登录报告为失败。
            let _ = write!(
                stream,
                "HTTP/1.1 303 See Other\r\nLocation: {success}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            Ok(info)
        }
        Err(error) => {
            reply(
                &mut stream,
                "400 Bad Request",
                "Login failed. Return to terminal.",
            );
            Err(error)
        }
    }
}

fn reply(stream: &mut TcpStream, status: &str, text: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    );
}

fn wait_callback(
    listener: &TcpListener,
    state: &str,
    deadline: Instant,
) -> Result<(TcpStream, String), Error> {
    while Instant::now() < deadline {
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(_) => return Err(Error::Internal("接收 OAuth 回调失败".into())),
        };
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let mut header = Vec::new();
        // 限制请求大小和绝对读取时间，避免无关连接让授权永久挂起。
        while header.len() < 8192 && !header.ends_with(b"\r\n\r\n") {
            let remaining = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(2));
            if remaining.is_zero() {
                break;
            }
            let _ = stream.set_read_timeout(Some(remaining));
            let mut byte = [0];
            match stream.read(&mut byte) {
                Ok(1) => header.push(byte[0]),
                _ => break,
            }
        }
        if !header.ends_with(b"\r\n\r\n") {
            reply(&mut stream, "400 Bad Request", "Invalid request.");
            continue;
        }
        let text = String::from_utf8_lossy(&header);
        let mut parts = text.lines().next().unwrap_or("").split_whitespace();
        let method = parts.next();
        let target = parts.next().unwrap_or("");
        let url = Url::parse(&format!("http://127.0.0.1{target}"));
        let Ok(url) = url else {
            reply(&mut stream, "400 Bad Request", "Invalid request.");
            continue;
        };
        if method != Some("GET") || url.path() != "/callback" {
            reply(&mut stream, "404 Not Found", "Not found.");
            continue;
        }
        let params: Vec<_> = url.query_pairs().collect();
        let states: Vec<_> = params.iter().filter(|(k, _)| k == "state").collect();
        if states.len() != 1 || states[0].1 != state {
            reply(&mut stream, "400 Bad Request", "Invalid state.");
            return Err(auth_error("OAuth state 校验失败，请重新登录"));
        }
        if params.iter().any(|(k, _)| k == "error") {
            reply(&mut stream, "400 Bad Request", "Authorization cancelled.");
            return Err(auth_error("OAuth 授权被取消或拒绝"));
        }
        let codes: Vec<_> = params.iter().filter(|(k, _)| k == "code").collect();
        if codes.len() != 1 || codes[0].1.is_empty() {
            reply(&mut stream, "400 Bad Request", "Missing code.");
            return Err(auth_error("OAuth 回调缺少唯一授权码"));
        }
        return Ok((stream, codes[0].1.to_string()));
    }
    Err(auth_error("等待 OAuth 授权超时，请重新运行 skz auth login"))
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let result = Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Ok(mut child) = result {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_s256_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn production_legacy_authorization_url_uses_quant_frontend() {
        assert_eq!(
            authorization_endpoint(
                "https://shengkezhi.com/oauth/authorize",
                crate::config::DEFAULT_BASE_URL
            )
            .unwrap()
            .as_str(),
            "https://quant.shengkezhi.com/oauth/authorize"
        );
        assert_eq!(
            authorization_endpoint(
                "https://quant.shengkezhi.com/oauth/authorize",
                crate::config::DEFAULT_BASE_URL
            )
            .unwrap()
            .join("success")
            .unwrap()
            .as_str(),
            "https://quant.shengkezhi.com/oauth/success"
        );
        for (value, base) in [
            (
                "http://127.0.0.1:1234/oauth/authorize",
                "http://127.0.0.1:5678/open/v1",
            ),
            (
                "https://tquant.shengkezhi.cn/oauth/authorize",
                "https://tapi.shengkezhi.cn/open/v1",
            ),
            (
                "https://custom.example.com/a/oauth/authorize",
                crate::config::DEFAULT_BASE_URL,
            ),
        ] {
            assert_eq!(authorization_endpoint(value, base).unwrap().as_str(), value);
        }
    }

    #[test]
    fn oauth_rejects_insecure_or_cross_origin_token_endpoints() {
        for value in [
            "http://example.com/oauth/token",
            "https://user:pass@example.com/oauth/token",
            "https://example.com/oauth/token?secret=x",
            "https://example.com/oauth/token#fragment",
        ] {
            assert!(checked_url(value).is_err());
        }
        assert!(checked_url("http://127.0.0.1:1234/oauth/authorize").is_ok());
        assert!(
            token_endpoint(
                "https://other.example.com/oauth/token",
                "https://api.example.com/open/v1"
            )
            .is_err()
        );
        assert!(
            token_endpoint(
                "https://api.example.com/oauth/token",
                "https://api.example.com/open/v1"
            )
            .is_ok()
        );
    }
}
