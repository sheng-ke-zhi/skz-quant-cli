use std::net::TcpListener;
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};

use super::*;

pub(super) fn force_install(adapter: &Adapter, enabled: bool) -> Result<(), Error> {
    if adapter.installed()?.is_some_and(|p| p.enabled) {
        adapter.run(&["plugin", "disable", ID, "--scope", "user"])?;
    }
    let result = install_through_gateway(adapter);
    // 维护结束后恢复启用意图；失败也要恢复，pending 保存重试时的原意图。
    if let Some(current) = adapter.installed()? {
        if current.enabled != enabled {
            adapter.run(&[
                "plugin",
                if enabled { "enable" } else { "disable" },
                ID,
                "--scope",
                "user",
            ])?;
        }
        if adapter.installed()?.is_none_or(|p| p.enabled != enabled) {
            return Err(argument(
                "插件维护后未能恢复原有启用状态；请重新运行安装包恢复",
            ));
        }
    }
    result
}

fn install_through_gateway(adapter: &Adapter) -> Result<(), Error> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).map_err(|_| argument("无法分配本机插件维护端口"))?;
    let port = listener
        .local_addr()
        .map_err(|e| argument(e.to_string()))?
        .port();
    let mut random = [0; 32];
    getrandom::fill(&mut random).map_err(|_| fail("无法生成本机插件维护凭证"))?;
    let nonce = STANDARD.encode(random);
    let authorization = format!("Bearer {nonce}");
    let base = format!("http://127.0.0.1:{port}/api/v1");
    drop(listener);
    // 仅在修复时启动回环接口；口令只注入本次子进程，不关闭认证或工具权限。
    let mut server = adapter
        .command()?
        .args([
            "--serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--auth",
            "password",
            "--base-path",
            "/",
            "--no-session-persistence",
            "--strict-mcp-config",
            "--mcp-config",
            "{\"mcpServers\":{}}",
        ])
        .env("CODEBUDDY_GATEWAY_AUTH", "password")
        .env("CODEBUDDY_GATEWAY_PASSWORD", nonce)
        .env_remove("CODEBUDDY_GATEWAY_BASE_PATH")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| argument("无法启动 WorkBuddy 原生插件维护接口"))?;
    let result = (|| {
        let probe: ureq::Agent = ureq::Agent::config_builder()
            .max_redirects(0)
            .timeout_global(Some(Duration::from_millis(750)))
            .build()
            .into();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if server
                .try_wait()
                .map_err(|e| fail(e.to_string()))?
                .is_some()
            {
                return Err(argument(
                    "WorkBuddy 插件维护接口启动失败，请更新 WorkBuddy 后重试",
                ));
            }
            if let Ok(mut response) = probe
                .get(format!("{base}/auth/status"))
                .header("Authorization", &authorization)
                .call()
                && let Ok(raw) = response
                    .body_mut()
                    .with_config()
                    .limit(8192)
                    .read_to_string()
                && let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
            {
                let auth = value.get("data").unwrap_or(&value);
                if auth.get("authEnabled").and_then(|v| v.as_bool()) != Some(true)
                    || auth.get("authenticated").and_then(|v| v.as_bool()) != Some(true)
                {
                    return Err(argument("本机插件维护接口认证检查失败，未继续安装"));
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err(argument("WorkBuddy 插件维护接口准备超时"));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        agent.post(format!("{base}/plugins"))
            .header("Authorization", &authorization).header("x-codebuddy-request", "1")
            .header("Content-Type", "application/json")
            .send(r#"{"plugin":"skz@skz","options":{"scope":"user","force":true,"autoEnable":false,"persistEnabledState":false}}"#)
            .map_err(|_| argument("WorkBuddy 原生接口未完成技能修复；保留备份供恢复"))?;
        Ok(())
    })();
    // 不保留后台服务、端口或口令；只结束本次创建的进程。
    if server
        .try_wait()
        .map_err(|e| fail(e.to_string()))?
        .is_none()
    {
        server
            .kill()
            .map_err(|_| argument("本次插件维护进程无法退出，请重新运行安装包恢复"))?;
    }
    let _ = server.wait();
    result
}
