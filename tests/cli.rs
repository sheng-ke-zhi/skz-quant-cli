//! 端到端测试：spawn `skz`，用 httpmock 当平台，断言 stdout / stderr / exit code。
//! 网络全部通过 `SKZ_BASE_URL` 走本地 mock，不访问真实平台。

use assert_cmd::Command;
use httpmock::Mock;
use httpmock::prelude::*;
use tempfile::TempDir;

/// credentials 文件路径：与 `credentials::credentials_path()` 的解析逻辑一致。
/// Linux 测试把 XDG_CONFIG_HOME 直接指向 base（它本身就是「.config」那一层）
/// → `base/skz/credentials`；macOS 测试把 HOME 指向 base，代码手动拼 `.config`
/// → `base/.config/skz/credentials`。有了它测试才能在开发机（macOS）与 CI（Linux）都跑通。
fn creds_file(base: &std::path::Path) -> std::path::PathBuf {
    let dir = if cfg!(target_os = "macos") {
        base.join(".config").join("skz")
    } else {
        base.join("skz")
    };
    dir.join("credentials")
}

/// 建一个带 credentials 的临时配置目录（token 唯一来源）。
fn config_with_token(token: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let creds = creds_file(dir.path());
    std::fs::create_dir_all(creds.parent().unwrap()).unwrap();
    std::fs::write(&creds, token).unwrap();
    dir
}

/// 一个隔离到临时配置目录的 `skz` 命令（XDG_CONFIG_HOME + HOME 都指向它）。
fn skz(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("skz").unwrap();
    cmd.env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path());
    cmd
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).expect("output was not JSON")
}

fn mock_factor_routes<'a>(server: &'a MockServer, codes: &[&str]) -> Mock<'a> {
    let items: Vec<_> = codes
        .iter()
        .map(|code| {
            serde_json::json!({
                "code": code,
                "name": code,
                "compute_engine": "alpha158",
                "key_inspect": "x",
                "economic_logic": "x",
                "why_effective": "x",
                "market_mechanism": "x",
                "failure_scenarios": [],
                "description": "",
                "tags": [],
                "creator": null,
                "create_time": "2026-01-01T00:00:00Z"
            })
        })
        .collect();
    let body = serde_json::json!({
        "code": 0,
        "msg": "ok",
        "data": {"total": items.len(), "items": items}
    })
    .to_string();
    server.mock(move |when, then| {
        when.method(GET).path("/research/factor-routes");
        then.status(200).body(body);
    })
}

fn mock_problem<'a>(server: &'a MockServer, code: &str) -> Mock<'a> {
    let body = serde_json::json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "code": code,
            "dataset": "stock",
            "description": "",
            "editable": true,
            "freq": "日线",
            "name": code,
            "source": "user",
            "symbols": ["000001.SZ"],
            "problem_type": "TimeSeriesProblem",
            "type_label": "时序",
            "time_segments": []
        }
    })
    .to_string();
    let path = format!("/research/problems/{code}");
    server.mock(move |when, then| {
        when.method(GET).path(path);
        then.status(200).body(body);
    })
}

fn mock_portfolios<'a>(server: &'a MockServer, codes: &[&str]) -> Mock<'a> {
    let items: Vec<_> = codes
        .iter()
        .map(|code| {
            serde_json::json!({
                "code": code,
                "status": "实盘",
                "base_market": "stock",
                "base_freq": "1d",
                "symbol_count": 1,
                "strategy_count": 1
            })
        })
        .collect();
    let body = serde_json::json!({"code": 0, "msg": "ok", "data": {"items": items}}).to_string();
    server.mock(move |when, then| {
        when.method(GET).path("/research/portfolios");
        then.status(200).body(body);
    })
}

fn mock_live_strategies<'a>(server: &'a MockServer, codes: &[&str]) -> Mock<'a> {
    let items: Vec<_> = codes
        .iter()
        .map(|code| {
            serde_json::json!({
                "base_freq": "1d",
                "code": code,
                "description": "",
                "last_heartbeat": null,
                "latest_weight_date": null,
                "outsample_sdt": null,
                "status": "实盘",
                "tags": [],
                "weight_type": "long_short"
            })
        })
        .collect();
    let body = serde_json::json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "items": items,
            "page": 1,
            "page_size": 1000,
            "total": items.len(),
            "status_counts": {}
        }
    })
    .to_string();
    server.mock(move |when, then| {
        when.method(GET)
            .path("/research/strategies")
            .query_param("status", "实盘")
            .query_param("page", "1")
            .query_param("page_size", "1000");
        then.status(200).body(body);
    })
}

fn mock_mining_overview<'a>(server: &'a MockServer, run_id: &str, groups: &[&str]) -> Mock<'a> {
    let problem_groups: Vec<_> = groups
        .iter()
        .map(|prefix| serde_json::json!({"count": 1, "label": prefix, "prefix": prefix}))
        .collect();
    let body = serde_json::json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "elimination_breakdown": [],
            "funnel": [],
            "kpi": {"eliminated": 0, "evaluate_methods": ["x"], "problem_count": 1,
                    "retain_rate": 1.0, "retained": 1, "total_candidates": 1,
                    "total_evaluations": 1},
            "problem_groups": problem_groups,
            "route": {"code": "RT_1", "compute_engine": "x",
                      "create_time": "2026-01-01T00:00:00Z", "creator": null,
                      "economic_logic": "x", "failure_scenarios": [], "key_inspect": "x",
                      "market_mechanism": "x", "name": "x", "tags": [],
                      "why_effective": "x"},
            "run_dir": "run",
            "run_id": run_id
        }
    })
    .to_string();
    let path = format!("/research/mining/{run_id}/overview");
    server.mock(move |when, then| {
        when.method(GET).path(path);
        then.status(200).body(body);
    })
}

/// 把编译好的测试二进制拷到 `<tmp>/<rel_bin_dir>/skz`——`rel_bin_dir` 传
/// `.local/pipx/venvs/skz-quant-cli/bin` 或 `.local/share/uv/tools/skz-quant-cli/bin`，
/// 跟各安装工具的真实目录结构对齐（见 CLAUDE.md「自更新」一节），
/// 这样从这个路径起的进程 `current_exe()` 天然落在对应渠道分支，不用造假路径。
/// `update` 测试专用：普通 `Command::cargo_bin` 起的进程永远落在 target/debug 下，
/// 测不到任何受支持渠道分支。
#[cfg(unix)]
fn fake_tool_install(tmp: &std::path::Path, rel_bin_dir: &str) -> std::path::PathBuf {
    fake_tool_install_named(tmp, rel_bin_dir, "skz")
}

#[cfg(unix)]
fn fake_tool_install_named(
    tmp: &std::path::Path,
    rel_bin_dir: &str,
    exe_name: &str,
) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let bin_dir = tmp.join(rel_bin_dir);
    std::fs::create_dir_all(&bin_dir).unwrap();
    let dest = bin_dir.join(exe_name);
    std::fs::copy(env!("CARGO_BIN_EXE_skz"), &dest).unwrap();
    let mut perms = std::fs::metadata(&dest).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dest, perms).unwrap();
    dest
}

/// 在新建的目录下写一个可执行的假包管理器 shell 脚本，返回该目录
/// （用来整个替换或前置到 PATH）。
#[cfg(unix)]
fn fake_tool_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let script = dir.join(name);
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    dir.to_path_buf()
}

#[test]
fn markets_ok_compact_json_and_auth_header() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/market/markets")
            .header("authorization", "Bearer sk_test");
        then.status(200)
            .body(r#"[{"market":"stock","count":5464},{"market":"etf","count":1532}]"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    // 紧凑 JSON + 结尾换行，字段名/结构对齐平台
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "[{\"market\":\"stock\",\"count\":5464},{\"market\":\"etf\",\"count\":1532}]\n"
    );
}

#[test]
fn symbols_search_encodes_query_and_returns() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/market/symbols")
            .query_param("market", "stock")
            .query_param("keyword", "平安")
            .query_param("page", "1")
            .query_param("size", "5");
        then.status(200).body(
            r#"{"page":1,"size":5,"total":1,"items":[{"id":1,"name":"平安银行","symbol":"000001.SZ","market":"stock","updateAt":"2026-07-08T12:31:22.989173+00:00"}]}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "symbols",
            "--market",
            "stock",
            "--keyword",
            "平安",
            "--page",
            "1",
            "--size",
            "5",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["total"], 1);
    assert_eq!(v["items"][0]["symbol"], "000001.SZ");
    // 后端发的是 UTC（+00:00），输出统一换算成东八区
    assert_eq!(
        v["items"][0]["updateAt"],
        "2026-07-08T20:31:22.989173+08:00"
    );
}

#[test]
fn calendar_only_open_maps_to_onlyopen_true() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/market/trading-calendar")
            .query_param("exchange", "SSE")
            .query_param("onlyOpen", "true");
        then.status(200)
            .body(r#"[{"exchange":"SSE","calDate":"2026-01-05","isOpen":true,"pretradeDate":"2025-12-31"}]"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["calendar", "SSE", "--only-open"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v[0]["isOpen"], true);
    assert_eq!(v[0]["calDate"], "2026-01-05");
}

#[test]
fn empty_result_is_success_not_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/market/symbols");
        then.status(200)
            .body(r#"{"page":1,"size":20,"total":0,"items":[]}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["symbols", "--keyword", "zzzznomatch"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success()); // exit 0
    assert_eq!(json(&out.stdout)["total"], 0);
}

#[test]
fn invalid_api_key_is_exit_3_fix_auth() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET);
        then.status(401)
            .body(r#"{"status":401,"title":"Key 无效","errorCode":"INVALID_API_KEY"}"#);
    });
    let cfg = config_with_token("sk_bad");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_auth");
    assert_eq!(v["error"]["code"], "INVALID_API_KEY");
    assert_eq!(v["error"]["status"], 401);
}

#[test]
fn quota_exceeded_is_exit_4_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET);
        then.status(429)
            .body(r#"{"status":429,"title":"配额超限","errorCode":"QUOTA_EXCEEDED"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    m.assert_calls(1); // 不可重试：只调一次
    assert_eq!(json(&out.stderr)["error"]["action"], "give_up");
}

#[test]
fn rate_limited_retries_three_times_then_exit_5() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET);
        then.status(429)
            .body(r#"{"status":429,"title":"限流","errorCode":"RATE_LIMITED"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(3); // 1 原始 + 2 重试
    assert_eq!(json(&out.stderr)["error"]["action"], "retry_later");
}

#[test]
fn retry_after_header_is_parsed_into_retry_after_ms() {
    // ureq 3.x 迁移改写了 header 读取路径（http::HeaderMap 取代 2.x 的
    // resp.header()）——这条之前完全没测过，属于"改哪测哪"。
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET);
        then.status(429)
            .header("Retry-After", "0")
            .body(r#"{"status":429,"title":"限流","errorCode":"RATE_LIMITED"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(3);
    assert_eq!(json(&out.stderr)["error"]["retryAfterMs"], 0);
}

#[test]
fn no_credentials_is_exit_3_with_remediation_and_no_leak() {
    let dir = TempDir::new().unwrap(); // 空目录，无 credentials
    let out = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .arg("markets")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_auth");
    assert!(v["error"]["remediation"]["howTo"].is_string());
}

#[test]
fn bad_size_is_exit_2_before_network() {
    let dir = config_with_token("sk_test");
    // 没有 mock：校验必须在发网络前失败
    let out = skz(&dir)
        .args(["symbols", "--size", "9999"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["kind"], "args");
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn bad_date_range_is_exit_2() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir)
        .args([
            "calendar",
            "SSE",
            "--start",
            "2026-02-01",
            "--end",
            "2026-01-01",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn unknown_subcommand_is_exit_2_json() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir).arg("frobnicate").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["kind"], "args");
}

#[test]
fn version_is_json_exit_0() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir).arg("--version").output().unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert!(v["cli"].is_string());
    assert_eq!(v["contract"], "2.14"); // 契约版本被 agent 编程校验，锁死值别只判类型
}

#[test]
fn help_is_exit_0() {
    let dir = config_with_token("sk_test");
    skz(&dir).arg("--help").assert().success();
}

#[test]
fn auth_set_trims_newline_and_roundtrips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    // set：stdin 带尾部换行，必须被裁掉
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", &path)
        .env("HOME", &path)
        .args(["auth", "set"])
        .write_stdin("sk_trimme\n")
        .assert()
        .success();
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(creds_file(&path)).unwrap()).unwrap();
    assert_eq!(stored["version"], 1);
    assert_eq!(stored["active"], "default");
    assert_eq!(stored["identities"]["default"]["token"], "sk_trimme");
    assert_eq!(stored["identities"]["default"]["writePolicy"], "allow");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(creds_file(&path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    // status → present:true
    let out = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", &path)
        .env("HOME", &path)
        .args(["auth", "status"])
        .output()
        .unwrap();
    assert_eq!(json(&out.stdout)["present"], true);
    assert_eq!(json(&out.stdout)["active"], "default");
    // status 不打印 token
    assert!(!String::from_utf8_lossy(&out.stdout).contains("sk_trimme"));

    // unset → present:false
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", &path)
        .env("HOME", &path)
        .args(["auth", "unset"])
        .assert()
        .success();
    let out = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", &path)
        .env("HOME", &path)
        .args(["auth", "status"])
        .output()
        .unwrap();
    assert_eq!(json(&out.stdout)["present"], false);
}

#[test]
fn auth_named_identities_require_explicit_default_and_never_print_tokens() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    let alice = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "add", "alice", "--read-only"])
        .write_stdin("sk_alice\n")
        .output()
        .unwrap();
    assert!(alice.status.success());
    let alice_body = json(&alice.stdout);
    assert_eq!(alice_body["name"], "alice");
    assert_eq!(alice_body["account"], "alice");
    assert_eq!(alice_body["writePolicy"], "deny");
    assert_eq!(alice_body["active"], false);

    let bob = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args([
            "auth",
            "add",
            "bob-write",
            "--account",
            "bob",
            "--allow-write",
        ])
        .write_stdin("sk_bob\n")
        .output()
        .unwrap();
    assert!(bob.status.success());

    let listed = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "list"])
        .output()
        .unwrap();
    let rendered = String::from_utf8(listed.stdout.clone()).unwrap();
    assert!(!rendered.contains("sk_alice"));
    assert!(!rendered.contains("sk_bob"));
    let list = json(&listed.stdout);
    assert!(list["active"].is_null());
    assert_eq!(list["identities"][0]["name"], "alice");
    assert_eq!(list["identities"][1]["name"], "bob-write");

    let missing_default = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .arg("markets")
        .output()
        .unwrap();
    assert_eq!(missing_default.status.code(), Some(3));
    let error = &json(&missing_default.stderr)["error"];
    assert_eq!(error["code"], "IDENTITY_REQUIRED");
    assert_eq!(error["remediation"]["requiresUserChoice"], true);
    assert_eq!(error["remediation"]["identities"][0], "alice");
}

#[test]
fn auth_use_persists_identity_policy_and_remove_clears_active() {
    let dir = TempDir::new().unwrap();
    let path = dir.path();
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "add", "alice", "--read-only"])
        .write_stdin("sk_alice")
        .assert()
        .success();

    let selected = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "use", "alice"])
        .output()
        .unwrap();
    let selected = json(&selected.stdout);
    assert_eq!(selected["active"], "alice");
    assert_eq!(selected["writePolicy"], "deny");
    assert_eq!(selected["persistent"], true);

    let status = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "status"])
        .output()
        .unwrap();
    let status = json(&status.stdout);
    assert_eq!(status["present"], true);
    assert_eq!(status["active"], "alice");
    assert_eq!(status["account"], "alice");
    assert_eq!(status["writePolicy"], "deny");
    assert_eq!(status["globalReadOnly"], false);
    assert_eq!(status["readOnly"], true);

    let removed = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "remove", "alice"])
        .output()
        .unwrap();
    assert!(removed.status.success());
    assert_eq!(json(&removed.stdout)["active"], serde_json::Value::Null);
    let status = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", path)
        .env("HOME", path)
        .args(["auth", "status"])
        .output()
        .unwrap();
    assert_eq!(json(&status.stdout)["present"], false);
}

#[test]
fn auth_add_migrates_legacy_plain_token_without_changing_default() {
    let dir = config_with_token("sk_legacy");
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "alice", "--read-only"])
        .write_stdin("sk_alice")
        .assert()
        .success();

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(creds_file(dir.path())).unwrap()).unwrap();
    assert_eq!(stored["active"], "default");
    assert_eq!(stored["identities"]["default"]["token"], "sk_legacy");
    assert_eq!(stored["identities"]["alice"]["token"], "sk_alice");
}

#[test]
fn auth_duplicate_requires_replace_and_validates_names() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "alice", "--read-only"])
        .write_stdin("sk_one")
        .assert()
        .success();

    let duplicate = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "alice", "--allow-write"])
        .write_stdin("sk_two")
        .output()
        .unwrap();
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(
        json(&duplicate.stderr)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--replace")
    );

    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "alice", "--allow-write", "--replace"])
        .write_stdin("sk_two")
        .assert()
        .success();
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(creds_file(dir.path())).unwrap()).unwrap();
    assert_eq!(stored["identities"]["alice"]["token"], "sk_two");
    assert_eq!(stored["identities"]["alice"]["writePolicy"], "allow");

    let invalid = Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "Alice", "--read-only"])
        .write_stdin("sk_bad")
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn named_read_only_identity_blocks_write_before_http() {
    let dir = TempDir::new().unwrap();
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "add", "alice", "--read-only"])
        .write_stdin("sk_alice")
        .assert()
        .success();
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "use", "alice"])
        .assert()
        .success();

    // 不启动 mock server：exit 8 本身证明请求没有进入网络层。
    let out = skz(&dir)
        .args(["route", "create"])
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(8));
    let rendered = String::from_utf8(out.stderr).unwrap();
    assert_eq!(
        json(rendered.as_bytes())["error"]["action"],
        "not_permitted"
    );
    assert!(!rendered.contains("sk_alice"));
}

#[test]
fn auth_use_changes_token_used_by_network_commands() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/market/markets")
            .header("authorization", "Bearer sk_bob");
        then.status(200).body(r#"[{"market":"stock","count":1}]"#);
    });
    let dir = TempDir::new().unwrap();
    for (name, token) in [("alice", "sk_alice"), ("bob", "sk_bob")] {
        Command::cargo_bin("skz")
            .unwrap()
            .env("XDG_CONFIG_HOME", dir.path())
            .env("HOME", dir.path())
            .args(["auth", "add", name, "--allow-write"])
            .write_stdin(token)
            .assert()
            .success();
    }
    Command::cargo_bin("skz")
        .unwrap()
        .env("XDG_CONFIG_HOME", dir.path())
        .env("HOME", dir.path())
        .args(["auth", "use", "bob"])
        .assert()
        .success();

    let out = skz(&dir)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
}

#[test]
fn base_url_flag_is_rejected() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir)
        .args(["markets", "--base-url", "http://127.0.0.1:8080"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["kind"], "args");
}

#[test]
fn invalid_base_url_env_is_fix_params() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir)
        .arg("markets")
        .env("SKZ_BASE_URL", "ftp://api.example.com/open/v1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let error = &json(&out.stderr)["error"];
    assert_eq!(error["kind"], "args");
    assert_eq!(error["action"], "fix_params");
    assert!(error["message"].as_str().unwrap().contains("SKZ_BASE_URL"));
}

#[test]
fn invalid_base_url_env_does_not_break_offline_commands() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir)
        .arg("--version")
        .env("SKZ_BASE_URL", "not a URL")
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn skill_show_outputs_books_and_rejects_unknown() {
    let dir = config_with_token("sk_test");
    // 无名 → 共享前言（auth / 退出码 / HITL 底表），它同时是索引
    let out = skz(&dir).args(["skills", "show"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("auth add"));
    assert!(s.contains("auth use"));
    assert!(s.contains("check_existing"));
    assert!(s.contains("HITL"));

    for (name, needle) in [
        ("guide", "market_mechanism"),
        ("factor", "mining factors"),
        ("strategy", "experiment delete"),
        ("portfolio", "portfolio create"),
    ] {
        let out = skz(&dir).args(["skills", "show", name]).output().unwrap();
        assert!(out.status.success(), "skill show {name} failed");
        let s = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(s.contains(needle), "skill show {name} missing {needle}");
        // 各册自带 frontmatter（description 决定 harness 里的触发）
        assert!(
            s.starts_with(&format!("---\nname: skz-{name}\n")),
            "skill show {name} missing frontmatter"
        );
        // 共享前言就地展开，占位符不该漏出去
        assert!(s.contains("HITL 底表"), "skill show {name} missing common");
        assert!(!s.contains("<!-- COMMON -->"), "{name} 占位符未替换");
    }

    let out = skz(&dir).args(["skills", "show", "nope"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn skill_install_status_uninstall_lifecycle() {
    let dir = config_with_token("sk_test");
    let root = dir.path().join(".claude").join("skills");

    // 装之前：needs_install
    let out = skz(&dir).args(["skills", "status"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["needs_install"], true);

    let out = skz(&dir).args(["skills", "install"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["installed"].as_array().unwrap().len(), 4);

    // 只写自己的技能目录：settings.json / CLAUDE.md 一概不碰
    for d in ["skz-factor", "skz-strategy", "skz-guide", "skz-portfolio"] {
        assert!(root.join(d).join("SKILL.md").is_file(), "{d} 未写入");
        assert!(
            root.join(d).join(".skz-install.json").is_file(),
            "{d} 无标记"
        );
    }
    assert!(!dir.path().join(".claude").join("settings.json").exists());
    assert!(!dir.path().join("CLAUDE.md").exists());

    let out = skz(&dir).args(["skills", "status"]).output().unwrap();
    assert_eq!(json(&out.stdout)["needs_install"], false);

    let out = skz(&dir).args(["skills", "uninstall"]).output().unwrap();
    assert!(out.status.success());
    assert!(!root.join("skz-factor").exists());
}

#[test]
fn skill_install_refuses_foreign_dir_and_uninstall_spares_it() {
    let dir = config_with_token("sk_test");
    // 同名目录但没有归属标记 = 别人的技能：不覆盖、不删除
    let foreign = dir.path().join(".claude").join("skills").join("skz-factor");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("SKILL.md"), "别人的技能").unwrap();

    let out = skz(&dir).args(["skills", "install"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
    // 整体拒绝：一个都不该装，避免半新半旧
    assert_eq!(
        std::fs::read_to_string(foreign.join("SKILL.md")).unwrap(),
        "别人的技能"
    );
    assert!(!foreign.parent().unwrap().join("skz-strategy").exists());
    assert!(!foreign.parent().unwrap().join("skz-portfolio").exists());

    let out = skz(&dir).args(["skills", "status"]).output().unwrap();
    let books = json(&out.stdout);
    assert_eq!(books["books"][0]["foreign"], true);

    // uninstall 只删带标记的目录，外来目录必须幸存
    let out = skz(&dir).args(["skills", "uninstall"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["books"][0]["skipped"], "foreign");
    assert!(foreign.join("SKILL.md").is_file());
}

#[test]
fn skill_status_flags_stale_install() {
    let dir = config_with_token("sk_test");
    skz(&dir).args(["skills", "install"]).output().unwrap();
    // 伪造旧版本戳：升级了二进制但忘了重装 → status 必须报出来
    let marker = dir
        .path()
        .join(".claude/skills/skz-factor/.skz-install.json");
    std::fs::write(
        &marker,
        r#"{"cli":"0.0.9","contract":"1.0","book":"factor"}"#,
    )
    .unwrap();

    let out = skz(&dir).args(["skills", "status"]).output().unwrap();
    let v = json(&out.stdout);
    assert_eq!(v["needs_install"], true);
    assert_eq!(v["books"][0]["stale"], true);
    assert_eq!(v["books"][0]["installed_cli"], "0.0.9");
}

#[test]
fn skill_permissions_lists_hitl_writes_only() {
    let dir = config_with_token("sk_test");
    let out = skz(&dir).args(["skills", "permissions"]).output().unwrap();
    assert!(out.status.success());
    let rules = json(&out.stdout)["ask"].as_array().unwrap().clone();
    let joined = rules
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(" ");
    // 命中 HITL 底表的写命令都要在；读命令不该被拦
    for needle in [
        "mine start",
        "explore start",
        "promote start",
        "factor delete",
        "experiment delete",
        "strategy status",
        "portfolio create",
    ] {
        assert!(joined.contains(needle), "缺少 ask 规则 {needle}");
    }
    assert!(!joined.contains("factor list"));
}

#[test]
fn skill_installs_to_every_harness_target() {
    let dir = config_with_token("sk_test");
    // 四家的技能约定一致：<root>/skills/<name>/SKILL.md（claude/codex 本机实证，
    // openclaw/hermes 依官方文档），所以 adapter 只是换根目录。
    for (t, cfg) in [
        ("claude", ".claude"),
        ("codex", ".codex"),
        ("openclaw", ".openclaw"),
        ("hermes", ".hermes"),
    ] {
        let out = skz(&dir)
            .args(["skills", "install", "--target", t])
            .output()
            .unwrap();
        assert!(out.status.success(), "target {t} 安装失败");
        let v = json(&out.stdout);
        assert_eq!(v["target"], t);
        // 单 target 仍出对象（不是数组），老脚本不破
        assert!(v.is_object(), "单 target 不该出数组");
        assert!(
            dir.path()
                .join(cfg)
                .join("skills/skz-factor/SKILL.md")
                .is_file()
        );
    }

    let out = skz(&dir)
        .args(["skills", "install", "--target", "nope"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn skill_target_all_covers_present_harnesses_only() {
    let dir = config_with_token("sk_test");
    // 只给 codex / openclaw 造配置目录 → all 应只命中这两家，
    // 不给不存在的 harness 造目录（那既没用又是噪音）。
    std::fs::create_dir_all(dir.path().join(".codex")).unwrap();
    std::fs::create_dir_all(dir.path().join(".openclaw")).unwrap();

    let out = skz(&dir)
        .args(["skills", "install", "--target", "all"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    let arr = v.as_array().expect("多 target 应出数组");
    let hit: Vec<_> = arr.iter().map(|r| r["target"].as_str().unwrap()).collect();
    assert_eq!(hit, vec!["codex", "openclaw"], "只应命中本机存在的");
    assert!(
        !dir.path().join(".hermes").exists(),
        "不该给不存在的 harness 造目录"
    );

    // status / uninstall 同样支持 all
    let out = skz(&dir)
        .args(["skills", "status", "--target", "all"])
        .output()
        .unwrap();
    let arr = json(&out.stdout);
    assert_eq!(arr.as_array().unwrap().len(), 2);
    assert_eq!(arr[0]["needs_install"], false);

    let out = skz(&dir)
        .args(["skills", "uninstall", "--target", "all"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!dir.path().join(".codex/skills/skz-factor").exists());
}

#[test]
fn skill_target_all_errors_when_no_harness_present() {
    // 一家都没有时给出可操作的错误，而不是静默装 0 家
    let dir = config_with_token("sk_test");
    let out = skz(&dir)
        .args(["skills", "install", "--target", "all"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

// ── 自更新（`update`）────────────────────────────────────────────────
// `update` 零 HTTP 调用，测试都不需要 `config_with_token`。

#[test]
fn update_unknown_channel_reports_public_install_remediation() {
    // 普通 cargo 测试二进制天然落在 target/debug 下，不在任何受支持安装目录里，
    // 不用特殊布置就能测到"识别不出渠道"这条默认分支。
    let dir = TempDir::new().unwrap();
    let out = skz(&dir).arg("update").output().unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "unknown");
    assert_eq!(v["attempted"], false);
    assert_eq!(v["updated"], false);
    assert!(v.get("cli_after").is_none(), "cli_after 不该出现");

    let remediation = v["remediation"].to_string();
    assert!(remediation.contains("brew install sheng-ke-zhi/tap/skz"));
    assert!(remediation.contains("scoop bucket add skz"));
    assert!(remediation.contains("scoop install skz"));
    assert!(remediation.contains("pipx install skz-quant-cli"));
    assert!(remediation.contains("uv tool install skz-quant-cli"));
    assert!(!remediation.contains("--index-url"));
    assert!(
        !remediation.to_lowercase().contains("release"),
        "uv/pipx 的修复指引不该绕到 GitHub Release：{remediation}"
    );
}

#[cfg(unix)]
#[test]
fn update_brew_channel_passes_expected_argv_and_confirms_unchanged_version() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), "Cellar/skz/0.1.9/bin");
    fake_tool_install(tmp.path(), "opt/skz/bin");
    let recorded = tmp.path().join("recorded-args.txt");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "brew",
        &format!("echo \"$@\" > \"{}\"\n", recorded.display()),
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "brew");
    assert_eq!(v["attempted"], true);
    assert_eq!(v["updated"], false);
    assert_eq!(
        std::fs::read_to_string(&recorded).unwrap().trim(),
        "upgrade skz"
    );
}

#[cfg(unix)]
#[test]
fn update_brew_channel_uses_opt_version_for_staleness() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), "Cellar/skz/0.1.9/bin");
    let stable = tmp.path().join("opt/skz/bin/skz");
    std::fs::create_dir_all(stable.parent().unwrap()).unwrap();
    let script_body = format!(
        "cat > \"{}\" <<'EOF'\n#!/bin/sh\necho '{{\"cli\":\"9.9.9\",\"contract\":\"9.9\"}}'\nEOF\nchmod +x \"{}\"\n",
        stable.display(),
        stable.display()
    );
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "brew", &script_body);
    let path_with_real_tools = format!(
        "{}:{}",
        scripts_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let marker_dir = tmp.path().join(".claude/skills/skz-factor");
    std::fs::create_dir_all(&marker_dir).unwrap();
    std::fs::write(
        marker_dir.join(".skz-install.json"),
        format!(
            r#"{{"cli":"{}","contract":"9.9","book":"factor"}}"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", path_with_real_tools)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "brew");
    assert_eq!(v["updated"], true);
    assert_eq!(v["cli_after"], "9.9.9");
    let stale = v["skills"]["stale"].as_array().unwrap();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0]["installed_cli"], env!("CARGO_PKG_VERSION"));
}

#[cfg(unix)]
#[test]
fn update_brew_channel_skips_skills_when_opt_path_is_unavailable() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), "Cellar/skz/0.1.9/bin");
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "brew", "exit 0");

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "brew");
    assert!(v["updated"].is_null());
    assert_eq!(v["skills"]["evaluated"], false);
    assert!(v["skills"]["skip_reason"].is_string());
}

#[cfg(unix)]
#[test]
fn update_brew_channel_nonzero_exit_is_retry_later() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), "Cellar/skz/0.1.9/bin");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "brew",
        "echo brew-failed >&2; exit 1",
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "subprocess");
    assert_eq!(v["error"]["action"], "retry_later");
    assert!(
        v["error"]["remediation"]["howTo"]
            .as_str()
            .unwrap()
            .contains("brew upgrade skz")
    );
}

#[cfg(unix)]
#[test]
fn update_scoop_channel_passes_expected_argv_and_confirms_unchanged_version() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install_named(tmp.path(), "scoop/apps/skz/0.1.9", "skz.exe");
    fake_tool_install_named(tmp.path(), "scoop/apps/skz/current", "skz.exe");
    let recorded = tmp.path().join("recorded-args.txt");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "scoop",
        &format!("echo \"$@\" > \"{}\"\n", recorded.display()),
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "scoop");
    assert_eq!(v["attempted"], true);
    assert_eq!(v["updated"], false);
    assert_eq!(
        std::fs::read_to_string(&recorded).unwrap().trim(),
        "update skz"
    );
}

#[cfg(unix)]
#[test]
fn update_scoop_channel_uses_current_path_after_version_change() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install_named(tmp.path(), "scoop/apps/skz/0.1.9", "skz.exe");
    let stable = tmp.path().join("scoop/apps/skz/current/skz.exe");
    std::fs::create_dir_all(stable.parent().unwrap()).unwrap();
    let script_body = format!(
        "cat > \"{}\" <<'EOF'\n#!/bin/sh\necho '{{\"cli\":\"8.8.8\",\"contract\":\"8.8\"}}'\nEOF\nchmod +x \"{}\"\n",
        stable.display(),
        stable.display()
    );
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "scoop", &script_body);
    let path_with_real_tools = format!(
        "{}:{}",
        scripts_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", path_with_real_tools)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "scoop");
    assert_eq!(v["updated"], true);
    assert_eq!(v["cli_after"], "8.8.8");
}

#[cfg(unix)]
#[test]
fn update_scoop_channel_skips_skills_when_current_path_is_unavailable() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install_named(tmp.path(), "scoop/apps/skz/0.1.9", "skz.exe");
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "scoop", "exit 0");

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "scoop");
    assert!(v["updated"].is_null());
    assert_eq!(v["skills"]["evaluated"], false);
}

#[cfg(unix)]
#[test]
fn update_scoop_channel_nonzero_exit_is_retry_later() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install_named(tmp.path(), "scoop/apps/skz/0.1.9", "skz.exe");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "scoop",
        "echo scoop-failed >&2; exit 1",
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "subprocess");
    assert_eq!(v["error"]["action"], "retry_later");
    assert!(
        v["error"]["remediation"]["howTo"]
            .as_str()
            .unwrap()
            .contains("scoop update skz")
    );
}

#[test]
fn update_reports_no_stale_skills_when_none_installed() {
    let dir = TempDir::new().unwrap();
    let out = skz(&dir).arg("update").output().unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["skills"]["evaluated"], true);
    assert_eq!(v["skills"]["stale"].as_array().unwrap().len(), 0);
}

#[test]
fn update_reports_stale_skills_without_acting_when_noninteractive() {
    let dir = TempDir::new().unwrap();
    skz(&dir).args(["skills", "install"]).output().unwrap();
    // 伪造旧版本戳，仿照 `skill_status_flags_stale_install` 的手法。
    let marker = dir
        .path()
        .join(".claude/skills/skz-factor/.skz-install.json");
    let stale_marker = r#"{"cli":"0.0.9","contract":"1.0","book":"factor"}"#;
    std::fs::write(&marker, stale_marker).unwrap();

    let out = skz(&dir).arg("update").output().unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    let stale = v["skills"]["stale"].as_array().unwrap();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0]["target"], "claude");
    assert_eq!(stale[0]["book"], "factor");
    assert_eq!(stale[0]["installed_cli"], "0.0.9");

    // assert_cmd 默认管道所有 stdio，天然非 tty → 只报告、不问、不动手。
    assert_eq!(v["skills"]["refresh_offered"], false);
    assert!(v["skills"]["refresh_accepted"].is_null());
    assert!(v["skills"]["refreshed"].is_null());

    // 落地证据：标记文件字节级未变——"只报告不动手"真的没有副作用。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), stale_marker);
}

#[test]
fn update_skills_check_ignores_project_scope() {
    let home = TempDir::new().unwrap();
    // 只需要 ~/.claude 存在（让 present_targets 判定 "claude" 在本机出现过），
    // 不在里面装任何东西——重点是它跟下面 cwd 下的过期标记是两个不同的目录。
    std::fs::create_dir_all(home.path().join(".claude")).unwrap();

    let cwd = TempDir::new().unwrap();
    let marker = cwd
        .path()
        .join(".claude/skills/skz-factor/.skz-install.json");
    std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
    std::fs::write(
        &marker,
        r#"{"cli":"0.0.9","contract":"1.0","book":"factor"}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("skz").unwrap();
    cmd.env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .current_dir(cwd.path())
        .arg("update");
    let out = cmd.output().unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert!(
        v["skills"]["checked_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "claude"),
        "claude 应该因为 ~/.claude 存在而被检查到"
    );
    assert_eq!(
        v["skills"]["stale"].as_array().unwrap().len(),
        0,
        "project scope 下的标记不该被 update 的技能核对看到（只查 Scope::User）"
    );
}

#[cfg(unix)]
#[test]
fn update_pipx_channel_confirms_unchanged_version() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/pipx/venvs/skz-quant-cli/bin");
    // 假 pipx：exit 0、不碰文件，模拟"已是最新，无需升级"。
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "pipx", "exit 0");

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "pipx");
    assert_eq!(v["attempted"], true);
    // 重新探测到的版本跟探测前一样（还是同一个文件）→ 确认没变，不是"不知道"。
    assert_eq!(v["updated"], false);
}

#[cfg(unix)]
#[test]
fn update_pipx_channel_detects_version_change_and_uses_it_for_staleness() {
    // 这是验证"staleness 比对基准"那个坑是否真的修好的关键用例：升级发生后，
    // 过期与否必须拿新版本号算，不能拿测试二进制自己编译时的旧版本号算。
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/pipx/venvs/skz-quant-cli/bin");
    let exe_str = exe.display().to_string();
    // 假 pipx：先写到同目录下的临时文件，再 rename 覆盖过去——模拟真实升级的
    // 安全自替换手法（write-new-then-rename）。直接 `cat > 正在跑的文件` 会截断
    // 当前进程自己映射着的可执行镜像，有实际崩溃风险，不能图省事直接覆盖。
    let script_body = format!(
        "cat > \"{exe_str}.new\" <<'EOF'\n#!/bin/sh\necho '{{\"cli\":\"9.9.9\",\"contract\":\"2.2\"}}'\nEOF\nchmod +x \"{exe_str}.new\"\nmv \"{exe_str}.new\" \"{exe_str}\"\n"
    );
    let scripts_dir = fake_tool_script(&tmp.path().join("fakebin"), "pipx", &script_body);
    // 这个假 pipx 脚本要用到 cat/chmod/mv 这些外部命令（不像另外两个假脚本只用
    // shell 内建的 echo/exit）——PATH 得前置而不是整个替换掉，否则脚本自己都跑不起来。
    let path_with_real_tools = format!(
        "{}:{}",
        scripts_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    // 沙箱里预先埋一条标记：cli 等于当前（升级前）编译版本——升级前不算过期，
    // 升级后应该被拿 "9.9.9" 重新判定为过期。
    let marker_dir = tmp.path().join(".claude/skills/skz-factor");
    std::fs::create_dir_all(&marker_dir).unwrap();
    std::fs::write(
        marker_dir.join(".skz-install.json"),
        format!(
            r#"{{"cli":"{}","contract":"2.2","book":"factor"}}"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", path_with_real_tools)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = json(&out.stdout);
    assert_eq!(v["channel"], "pipx");
    assert_eq!(v["updated"], true);
    assert_eq!(v["cli_after"], "9.9.9");

    let stale = v["skills"]["stale"].as_array().unwrap();
    assert_eq!(stale.len(), 1, "旧版本戳应该被判定为落后于 9.9.9");
    assert_eq!(stale[0]["installed_cli"], env!("CARGO_PKG_VERSION"));
}

#[cfg(unix)]
#[test]
fn update_pipx_channel_nonzero_exit_is_retry_later() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/pipx/venvs/skz-quant-cli/bin");
    let scripts_dir =
        fake_tool_script(&tmp.path().join("fakebin"), "pipx", "echo boom >&2; exit 1");

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "subprocess");
    assert_eq!(v["error"]["action"], "retry_later");
}

#[cfg(unix)]
#[test]
fn update_pipx_channel_spawn_failure_is_retry_later() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/pipx/venvs/skz-quant-cli/bin");
    // PATH 指向一个空目录，排除宿主机上可能真实存在的 pipx——不依赖开发机/CI
    // 是否装了真 pipx，模拟"起不来"。
    let empty_path = tmp.path().join("empty-path");
    std::fs::create_dir_all(&empty_path).unwrap();

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &empty_path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "subprocess");
    assert_eq!(v["error"]["action"], "retry_later");
}

/// 上面几条 pipx 用例只断言退出码/JSON 形状，从没验证过实际传给子进程的 argv——
/// 一个把 args 拼错的实现（比如漏了 upgrade、传错包名）照样能让假脚本按预期
/// 退出。这条录下 `$@`，钉住 `pipx upgrade skz-quant-cli` 这个具体调用。
#[cfg(unix)]
#[test]
fn update_pipx_channel_passes_expected_upgrade_argv() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/pipx/venvs/skz-quant-cli/bin");
    let recorded = tmp.path().join("recorded-args.txt");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "pipx",
        &format!("echo \"$@\" > \"{}\"\nexit 1\n", recorded.display()),
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(
        std::fs::read_to_string(&recorded).unwrap().trim(),
        "upgrade skz-quant-cli"
    );
}

/// uv 渠道在此之前完全没有端到端覆盖——只有 `detect_channel_uv_path` 这条纯路径
/// 嗅探的单测，`("uv", &["tool", "upgrade", PACKAGE_NAME])` 这三个参数的拼接
/// 从没被真的跑过一次。跟上面 pipx 那条一样录 argv，顺带覆盖 uv 渠道的错误路径
/// 形状（跟 pipx 应该完全一致，因为两者共用 `upgrade()` 里同一段失败处理逻辑）。
#[cfg(unix)]
#[test]
fn update_uv_channel_passes_expected_upgrade_argv_and_reports_retry_later() {
    let tmp = TempDir::new().unwrap();
    let exe = fake_tool_install(tmp.path(), ".local/share/uv/tools/skz-quant-cli/bin");
    let recorded = tmp.path().join("recorded-args.txt");
    let scripts_dir = fake_tool_script(
        &tmp.path().join("fakebin"),
        "uv",
        &format!("echo \"$@\" > \"{}\"\nexit 1\n", recorded.display()),
    );

    let out = std::process::Command::new(&exe)
        .arg("update")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("PATH", &scripts_dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "subprocess");
    assert_eq!(v["error"]["action"], "retry_later");
    assert_eq!(
        std::fs::read_to_string(&recorded).unwrap().trim(),
        "tool upgrade skz-quant-cli"
    );
    assert!(
        v["error"]["remediation"]["howTo"]
            .as_str()
            .unwrap()
            .contains("uv tool upgrade skz-quant-cli")
    );
}

// 交互式 TTY 问答分支（真终端场景下人真的按 y/Enter）没有自动化覆盖——这仓库没有
// PTY 测试设施，assert_cmd/std::process::Command 默认管道所有 stdio，搭不出真终端。
// 仅人工验证，不假装有覆盖。

// ── 策略业务：写 + 读 ────────────────────────────────────────────────

#[test]
fn route_create_reads_stdin_json_and_returns_routecode() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/routes")
            .header("authorization", "Bearer sk_test")
            .json_body(serde_json::json!({"name": "放量突破", "market_mechanism": "趋势跟踪"}));
        then.status(200).body(r#"{"routeCode":"RT_1"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["route", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"放量突破","market_mechanism":"趋势跟踪"}"#)
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "{\"routeCode\":\"RT_1\"}\n"
    );
}

#[test]
fn route_create_empty_stdin_is_exit_2_before_network() {
    let cfg = config_with_token("sk_test");
    // 无 mock：空 stdin 必须在发网络前失败
    let out = skz(&cfg)
        .args(["route", "create"])
        .write_stdin("")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn route_create_malformed_json_is_exit_2() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["route", "create"])
        .write_stdin("not json at all")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn problem_create_unwraps_envelope() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200)
            .body(r#"{"code":0,"msg":"success","data":{"code":"PRB_1"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"银行股短期动量"}"#)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "{\"problemCode\":\"PRB_1\"}\n"
    );
}

#[test]
fn problem_delete_sends_delete_without_body_and_returns_ack() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE).path("/research/problems/PRB_1");
        then.status(200)
            .body(r#"{"code":0,"msg":"删除成功","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "delete", "PRB_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(out.status.success());
    m.assert_calls(1);
    assert_eq!(
        json(&out.stdout),
        serde_json::json!({
            "code": "PRB_1",
            "deleted": true
        })
    );
}

#[test]
fn problem_delete_surfaces_forbidden_and_not_found() {
    for (status, response_code, expected_exit, expected_action) in
        [(403, 40301, 3, "fix_auth"), (404, 40400, 2, "fix_params")]
    {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(DELETE).path("/research/problems/PRB_1");
            then.status(status).body(format!(
                r#"{{"code":{response_code},"msg":"cannot delete","data":null}}"#
            ));
        });
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["problem", "delete", "PRB_1"])
            .env("SKZ_BASE_URL", server.base_url())
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(expected_exit));
        m.assert_calls(1);
        let error = json(&out.stderr);
        assert_eq!(error["error"]["action"], expected_action);
        assert_eq!(error["error"]["code"], response_code.to_string());
    }
}

#[test]
fn problem_delete_write_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE).path("/research/problems/PRB_1");
        then.status(503)
            .body(r#"{"code":50301,"msg":"上游不可用","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "delete", "PRB_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1);
}

#[test]
fn problem_delete_network_error_verifies_problem_detail() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "delete", "PRB_1"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59924")
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(7));
    let error = json(&out.stderr);
    assert_eq!(error["error"]["action"], "check_existing");
    assert_eq!(error["error"]["retryable"], false);
    assert_eq!(
        error["error"]["remediation"]["verifyWith"],
        "skz problem get <code>"
    );
}

#[test]
fn problem_create_rejects_symbols_without_market_suffix_before_request() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200)
            .body(r#"{"code":0,"msg":"success","data":{"code":"PRB_1"}}"#);
    });
    let cfg = config_with_token("sk_test");
    for dataset in ["stock", "etf", "future"] {
        let body = serde_json::json!({
            "dataset": dataset,
            "name": "银行股短期动量",
            "symbols": ["000001", "600000.SH"]
        });
        let out = skz(&cfg)
            .args(["problem", "create"])
            .env("SKZ_BASE_URL", server.base_url())
            .write_stdin(body.to_string())
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(2), "dataset={dataset}");
        let err = json(&out.stderr);
        assert_eq!(err["error"]["action"], "fix_params");
        assert!(err["error"]["message"].as_str().unwrap().contains("000001"));
        assert!(
            err["error"]["message"]
                .as_str()
                .unwrap()
                .contains("skz symbols --keyword")
        );
    }
    m.assert_calls(0);
}

#[test]
fn problem_create_skips_symbol_suffix_validation_for_other_or_missing_dataset() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200)
            .body(r#"{"code":0,"msg":"success","data":{"code":"PRB_1"}}"#);
    });
    let cfg = config_with_token("sk_test");
    for body in [
        serde_json::json!({"dataset": "index", "symbols": ["000001"]}),
        serde_json::json!({"symbols": ["000001"]}),
    ] {
        let out = skz(&cfg)
            .args(["problem", "create"])
            .env("SKZ_BASE_URL", server.base_url())
            .write_stdin(body.to_string())
            .output()
            .unwrap();

        assert!(out.status.success());
    }
    m.assert_calls(2);
}

#[test]
fn problem_create_accepts_qualified_symbols() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/problems")
            .json_body(serde_json::json!({
                "dataset": "stock",
                "name": "银行股短期动量",
                "symbols": ["000001.SZ", "600000.SH"]
            }));
        then.status(200)
            .body(r#"{"code":0,"msg":"success","data":{"code":"PRB_1"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(
            r#"{"dataset":"stock","name":"银行股短期动量","symbols":["000001.SZ","600000.SH"]}"#,
        )
        .output()
        .unwrap();

    assert!(out.status.success());
    m.assert_calls(1);
}

#[test]
fn problem_create_bad_envelope_is_exit_6() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200)
            .body(r#"{"code":1,"msg":"字段缺失","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert_eq!(json(&out.stderr)["error"]["action"], "internal");
}

#[test]
fn portfolio_list_returns_items() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/research/portfolios")
            .header("authorization", "Bearer sk_test");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[
                {"code":"PF_1","description":"","status":"实盘","base_market":"stock",
                 "base_freq":"1d","symbol_count":3,"strategy_count":2,
                 "sdt":"2024-01-01","edt":"2026-01-01",
                 "annual_return":0.12,"sharpe":1.1,"max_drawdown":-0.1,"abs_return":0.3,
                 "job_status":null,"job_error":null}
            ]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "list"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["items"][0]["code"], "PF_1");
    assert_eq!(v["items"][0]["job_status"], serde_json::Value::Null);
}

#[test]
fn portfolio_get_returns_detail() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/research/portfolios/PF_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{
                "meta":{"code":"PF_1","status":"实盘","base_market":"stock","base_freq":"1d",
                        "price_field":"close","rebalance_method":"equal_weight",
                        "lookback_days":60,"fee_bp":0.0,"digits":2,"symbol_count":3,
                        "sdt":"2024-01-01","edt":"2026-01-01"},
                "compare":{},"nav":{},"positions":{}
            }}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "get", "PF_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["meta"]["code"], "PF_1");
}

#[test]
fn portfolio_get_pending_is_exit_2_fix_params_not_a_transient_failure() {
    // 组合生成中/生成失败时,detail 端点同样 404——跟"code 打错了"经同一条 classify 路径,
    // 落在 fix_params(exit 2)。这条锁住"别用 get 轮询建组合进度"这个行为，书里的警告才站得住。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/portfolios/PF_PENDING");
        then.status(404)
            .body(r#"{"code":40400,"msg":"portfolio not found: PF_PENDING","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "get", "PF_PENDING"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn portfolio_create_reads_stdin_and_returns_ack() {
    let server = MockServer::start();
    let portfolios = mock_portfolios(&server, &[]);
    let strategies = mock_live_strategies(&server, &["STS_1"]);
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/research/portfolios")
            .header("authorization", "Bearer sk_test")
            .json_body(serde_json::json!({
                "portfolio_code": "PF_1",
                "candidate_strategies": ["STS_1"],
                "rebalance_dates": ["2025-01-01"],
                "base_market": "stock"
            }));
        then.status(202).body(
            r#"{"code":0,"msg":"已受理，正在后台生成","data":{"portfolio_code":"PF_1","status":"pending"}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(
            r#"{"portfolio_code":"PF_1","candidate_strategies":["STS_1"],"rebalance_dates":["2025-01-01"],"base_market":"stock"}"#,
        )
        .output()
        .unwrap();
    m.assert();
    portfolios.assert();
    strategies.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["portfolio_code"], "PF_1");
    assert_eq!(v["status"], "pending");
}

#[test]
fn portfolio_create_rejects_existing_code_before_paid_request() {
    let server = MockServer::start();
    mock_portfolios(&server, &["PF_1"]);
    let post = server.mock(|when, then| {
        when.method(POST).path("/research/portfolios");
        then.status(202);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(
            r#"{"portfolio_code":"PF_1","candidate_strategies":["STS_1"],"rebalance_dates":["2025-01-01"],"base_market":"stock"}"#,
        )
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert!(
        json(&out.stderr)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("已存在")
    );
    post.assert_calls(0);
}

#[test]
fn portfolio_create_rejects_non_live_candidate_before_paid_request() {
    let server = MockServer::start();
    mock_portfolios(&server, &[]);
    mock_live_strategies(&server, &["STS_LIVE"]);
    let post = server.mock(|when, then| {
        when.method(POST).path("/research/portfolios");
        then.status(202);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(
            r#"{"portfolio_code":"PF_2","candidate_strategies":["STS_PAUSED"],"rebalance_dates":["2025-01-01"],"base_market":"stock"}"#,
        )
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert!(
        json(&out.stderr)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("STS_PAUSED")
    );
    post.assert_calls(0);
}

#[test]
fn mine_start_triggers_and_returns_ack() {
    let server = MockServer::start();
    let routes = mock_factor_routes(&server, &["RT_1"]);
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/miner/runs")
            .json_body(serde_json::json!({"routeCode": "RT_1"}));
        then.status(200)
            .body(r#"{"fcRunId":"fc1","status":"running","routeCode":"RT_1"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "start", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    routes.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["fcRunId"], "fc1");
    assert_eq!(v["status"], "running");
    assert_eq!(v["routeCode"], "RT_1"); // miner 触发独有 routeCode，透传别丢
}

#[test]
fn mine_start_rejects_unknown_route_before_paid_request() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_VALID"]);
    let post = server.mock(|when, then| {
        when.method(POST).path("/strategy/miner/runs");
        then.status(200);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "start", "--route", "RT_BAD"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
    post.assert_calls(0);
}

#[test]
fn mine_start_409_is_exit_7_check_existing_and_not_retried() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/miner/runs");
        then.status(409)
            .body(r#"{"status":409,"title":"该路线正在执行中"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "start", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    m.assert_calls(1); // 写不自动重试：只调一次
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "check_existing");
    assert_eq!(v["error"]["status"], 409);
}

#[test]
fn explore_start_402_is_exit_4_giveup_with_topup_remediation() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    mock_problem(&server, "PRB_1");
    server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/explore")
            .json_body(serde_json::json!({"problemCode": "PRB_1", "routeCode": "RT_1"}));
        then.status(402)
            .body(r#"{"status":402,"title":"余额不足"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "start", "--problem", "PRB_1", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "give_up");
    assert_eq!(v["error"]["status"], 402);
    assert!(v["error"]["remediation"]["howTo"].is_string());
}

#[test]
fn explore_start_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    mock_problem(&server, "PRB_1");
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/explore");
        then.status(503)
            .body(r#"{"status":503,"title":"后端未就绪"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "start", "--problem", "PRB_1", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    // 关键：503 虽 retry_later，但写路径不进 with_retry → 只调一次
    m.assert_calls(1);
    assert_eq!(json(&out.stderr)["error"]["action"], "retry_later");
}

#[test]
fn explore_start_rejects_unknown_problem_before_paid_request() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    server.mock(|when, then| {
        when.method(GET).path("/research/problems/PRB_BAD");
        then.status(404)
            .body(r#"{"code":40400,"msg":"problem not found","data":null}"#);
    });
    let post = server.mock(|when, then| {
        when.method(POST).path("/strategy/explore");
        then.status(200);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "explore",
            "start",
            "--problem",
            "PRB_BAD",
            "--route",
            "RT_1",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
    post.assert_calls(0);
}

#[test]
fn explore_poll_rate_limited_retries_three_times() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/explore/poll");
        then.status(429)
            .body(r#"{"status":429,"title":"限流","errorCode":"RATE_LIMITED"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "poll", "fc1", "fc2"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    // 关键：poll 是读、幂等，进 with_retry → 1 原始 + 2 重试（与写路径对照）
    m.assert_calls(3);
}

#[test]
fn explore_poll_sends_runids_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/explore/poll")
            .json_body(serde_json::json!({"runIds": ["fc1", "fc2"]}));
        then.status(200).body(
            r#"[{"fcRunId":"fc1","status":"running","percent":42,"step":"mining","done":false,"ok":false}]"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "poll", "fc1", "fc2"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v[0]["fcRunId"], "fc1");
    assert_eq!(v[0]["percent"], 42);
    assert_eq!(v[0]["step"], "mining");
}

#[test]
fn explore_get_progress_passthrough() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/strategy/explore/fc1");
        then.status(200).body(
            r#"{"fcRunId":"fc1","status":"running","statusText":"执行中","done":false,"ok":false,"percent":42,"step":"mining","message":"跑第2步","createdAt":"2026-07-20T11:24:52+00:00"}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "get", "fc1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["fcRunId"], "fc1");
    assert_eq!(v["percent"], 42);
    // explore-get 无 routeCode：不应凭空造出 null 字段
    assert!(v.get("routeCode").is_none());
}

#[test]
fn explore_runs_pagination() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/strategy/explore/runs")
            .query_param("status", "active")
            .query_param("page", "1")
            .query_param("size", "5");
        then.status(200).body(
            r#"{"page":1,"size":5,"total":1,"items":[{"fcRunId":"fc1","routeCode":"RT_1","status":"running","statusText":"执行中","done":false,"ok":false,"errorCode":null,"errorMessage":null,"resultPath":null,"createdAt":"2026-07-20T11:24:52+00:00","finishedAt":null}]}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["explore", "runs", "--status", "active"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["total"], 1);
    assert_eq!(v["items"][0]["fcRunId"], "fc1");
}

#[test]
fn adopted_routes_returns_array() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/strategy/routes/adopted");
        then.status(200)
            .body(r#"[{"routeCode":"RT_1","name":"放量突破后的短期动量"}]"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["route", "adopted"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v[0]["routeCode"], "RT_1");
    assert_eq!(v[0]["name"], "放量突破后的短期动量");
}

#[test]
fn mine_poll_over_100_ids_is_exit_2_before_network() {
    let cfg = config_with_token("sk_test");
    let mut args: Vec<String> = vec!["mine".into(), "poll".into()];
    args.extend((0..101).map(|i| format!("fc{i}")));
    let out = skz(&cfg).args(&args).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

// ── 写路径不重试 ─────────────────────────────────────────────────
// 关键不变量：所有触发/创建（扣费、非幂等）即使遇 retryable 503 也**只调一次**。
// 用 503（retry_later）而非 409（check_existing 本就不重试）才真正证明“绕开 with_retry”。

#[test]
fn mine_start_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/miner/runs");
        then.status(503)
            .body(r#"{"status":503,"title":"后端未就绪"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "start", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1); // 触发绝不自动重试
    assert_eq!(json(&out.stderr)["error"]["action"], "retry_later");
}

#[test]
fn route_create_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/routes");
        then.status(503)
            .body(r#"{"status":503,"title":"后端未就绪"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["route", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1); // 创建非幂等，绝不自动重试
}

#[test]
fn problem_create_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(503)
            .body(r#"{"status":503,"title":"后端未就绪"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1);
}

#[test]
fn write_transport_error_is_outcome_unknown_with_verify_hint() {
    // 真机实测：/strategy/* 的落库写会 30s 超时。写超时**结果未知**（可能已落库），
    // 盲重试会重复扣费/建重复资源 → 必须标 retryable:false 并给出查证命令。
    // 打一个没人监听的 loopback 端口 → 连接被拒，走 Error::Network 分支。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["route", "create"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59917")
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    // exit 7 check_existing，不是 5——契约要求 agent「照 action 分支」，
    // 而 retry_later 的字面意思就是重发；写超时必须导向「先查证」。
    assert_eq!(out.status.code(), Some(7));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["kind"], "network");
    assert_eq!(v["error"]["action"], "check_existing");
    // action 与 retryable 必须一致，不能自相矛盾（真机评测暴露过）
    assert_eq!(v["error"]["retryable"], false);
    assert_eq!(
        v["error"]["remediation"]["verifyWith"],
        "skz factor-routes list"
    );
}

#[test]
fn read_transport_error_stays_retryable() {
    // 对照：读超时结果是确定的（没拿到数据），重来一次即可 → 仍标 retryable:true。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", "http://127.0.0.1:59918")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(json(&out.stderr)["error"]["retryable"], true);
}

#[test]
fn portfolio_create_preflight_network_error_is_retryable() {
    // 预检读都没成功时，CLI 可以确定付费 POST 尚未发生，因此仍可安全重试整条命令。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["portfolio", "create"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59919")
        .write_stdin(r#"{"portfolio_code":"PF_1","candidate_strategies":["STS_1"],"rebalance_dates":["2025-01-01"],"base_market":"stock"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "retry_later");
    assert_eq!(v["error"]["retryable"], true);
}

#[test]
fn experiment_delete_write_network_error_verifies_candidate_list() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["experiment", "delete", "E1", "TS_1"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59920")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "check_existing");
    assert_eq!(v["error"]["retryable"], false);
    assert_eq!(
        v["error"]["remediation"]["verifyWith"],
        "skz experiment strategies <id>"
    );
}

#[test]
fn platform_research_errors_are_structured_exit_2_fix_params() {
    // 真机暴露：problem create 缺必需时间段，C# 把 Rust 的 {code:42201} 原样透传，
    // 既要按数值 code 分类成 fix_params，也要保留 code/msg，不能退化成一整段原始 JSON。
    for (status, body, expected_code, expected_message) in [
        (
            422u16,
            r#"{"code":42201,"msg":"validation failed: 缺少必需时间段: 训练集A段","data":null}"#,
            "42201",
            "validation failed: 缺少必需时间段: 训练集A段",
        ),
        (
            404,
            r#"{"code":40400,"msg":"数据不存在或尚未生成","data":null}"#,
            "40400",
            "数据不存在或尚未生成",
        ),
    ] {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST).path("/strategy/problems");
            then.status(status).body(body);
        });
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["problem", "create"])
            .env("SKZ_BASE_URL", server.base_url())
            .write_stdin(r#"{"name":"x"}"#)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "status {status} 应为 exit 2");
        let error = json(&out.stderr);
        assert_eq!(error["error"]["action"], "fix_params");
        assert_eq!(error["error"]["code"], expected_code);
        assert_eq!(error["error"]["message"], expected_message);
        m.assert_calls(1); // 写不重试
    }
}

// ── 读路径（含 POST poll）会重试；mine poll 与 explore poll 对称 ─────────────

#[test]
fn mine_poll_rate_limited_retries_three_times() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST).path("/strategy/miner/poll");
        then.status(429)
            .body(r#"{"status":429,"title":"限流","errorCode":"RATE_LIMITED"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "poll", "fc1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(3); // poll 是读、幂等，进 with_retry
}

#[test]
fn mine_poll_sends_runids_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/miner/poll")
            .json_body(serde_json::json!({"runIds": ["fc1", "fc2"]}));
        then.status(200)
            .body(r#"[{"fcRunId":"fc1","status":"running","done":false,"ok":false}]"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mine", "poll", "fc1", "fc2"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)[0]["fcRunId"], "fc1");
}

// ── 写端点的 scope 不足 → fix_auth ────────────────────────────────

#[test]
fn mine_start_insufficient_scope_is_exit_3_fix_auth() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    server.mock(|when, then| {
        when.method(POST).path("/strategy/miner/runs");
        then.status(403)
            .body(r#"{"status":403,"title":"缺少 scope","errorCode":"INSUFFICIENT_SCOPE"}"#);
    });
    let cfg = config_with_token("sk_readonly");
    let out = skz(&cfg)
        .args(["mine", "start", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_auth");
    assert_eq!(v["error"]["code"], "INSUFFICIENT_SCOPE");
}

// ── explore start 可选会话字段透传 ────────────────────────────────

#[test]
fn explore_start_passes_conversation_and_tool_call_ids() {
    let server = MockServer::start();
    mock_factor_routes(&server, &["RT_1"]);
    mock_problem(&server, "PRB_1");
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/explore")
            .json_body(serde_json::json!({
                "problemCode": "PRB_1",
                "routeCode": "RT_1",
                "conversationId": "conv1",
                "toolCallId": "tc1"
            }));
        then.status(200)
            .body(r#"{"fcRunId":"fc1","status":"running"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "explore",
            "start",
            "--problem",
            "PRB_1",
            "--route",
            "RT_1",
            "--conversation-id",
            "conv1",
            "--tool-call-id",
            "tc1",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert(); // json_body 精确匹配：字段名/值错了这里就挂
    assert!(out.status.success());
}

// ── problem 信封：code==0 但 problemCode 缺失/空 → 非成功包 exit 6 ─────────

#[test]
fn problem_create_code0_but_missing_problemcode_is_exit_6() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200).body(r#"{"code":0,"msg":"ok","data":{}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert_eq!(json(&out.stderr)["error"]["action"], "internal");
}

#[test]
fn problem_create_blank_problemcode_is_exit_6() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/strategy/problems");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":""}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "create"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(r#"{"name":"x"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
}

// ── 无 errorCode 的裸 status（网关/代理）也要归到正确动作，不掉 internal ───────

#[test]
fn bare_401_without_errorcode_is_exit_3_fix_auth() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET);
        then.status(401).body("Unauthorized"); // 非 JSON、无 errorCode
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["markets"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    m.assert_calls(1); // fix_auth 不重试
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_auth");
}

#[test]
fn bare_429_without_errorcode_is_exit_5_retry_later() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET);
        then.status(429).body(r#"{"message":"slow down"}"#); // 无 errorCode
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["markets"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(3); // 归 retry_later → 有限重试
}

// ── token 不泄露：真 token 在场时，API 错误的 stderr 里不得出现它 ─────────────

#[test]
fn token_not_leaked_on_api_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET);
        then.status(401)
            .body(r#"{"status":401,"title":"Key 无效","errorCode":"INVALID_API_KEY"}"#);
    });
    let cfg = config_with_token("sk_super_secret_123");
    let out = skz(&cfg)
        .args(["markets"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stderr.contains("sk_super_secret_123"),
        "token leaked to stderr"
    );
    assert!(
        !stdout.contains("sk_super_secret_123"),
        "token leaked to stdout"
    );
}

// ── research 信封（/research/*，{code,msg,data}，业务错骑非 2xx）──────────────

#[test]
fn whoami_unwraps_research_envelope() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/whoami");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"user_id":"u1"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["whoami"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    // stdout 是拆封后的 data，不含 code/msg
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "{\"user_id\":\"u1\"}\n"
    );
}

#[test]
fn research_read_notready_42201_retries_then_exit_5() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/research/whoami");
        then.status(422)
            .body(r#"{"code":42201,"msg":"数据尚未就绪，请稍后重试"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["whoami"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(json(&out.stderr)["error"]["action"], "retry_later");
    assert_eq!(json(&out.stderr)["error"]["code"], "42201");
    m.assert_calls(3); // 读命令 42201=NotReady → 重试满 3 次
}

#[test]
fn research_read_404_40400_is_exit_2_fix_params() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/research/whoami");
        then.status(404)
            .body(r#"{"code":40400,"msg":"资源不存在"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["whoami"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
    m.assert_calls(1); // 非 retry，不重试
}

#[test]
fn research_409_40901_is_exit_7_check_existing() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/whoami");
        then.status(409).body(r#"{"code":40901,"msg":"已在进行"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["whoami"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    assert_eq!(json(&out.stderr)["error"]["action"], "check_existing");
}

#[test]
fn research_read_40909_is_exit_5_and_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/research/strategies");
        then.status(409)
            .body(r#"{"code":40909,"msg":"工作区正在初始化，请稍后重试","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "list"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let body = json(&out.stderr);
    assert_eq!(body["error"]["action"], "retry_later");
    assert_eq!(body["error"]["retryable"], true);
    assert_eq!(body["error"]["code"], "40909");
    m.assert_calls(3);
}

#[test]
fn research_insufficient_scope_falls_back_to_platform_exit_3() {
    // /research/* 上的 scope 错误来自 C# 网关的平台信封（非 research 信封），须回落归 fix_auth。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/whoami");
        then.status(403)
            .body(r#"{"status":403,"title":"缺少 scope","errorCode":"INSUFFICIENT_SCOPE"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["whoami"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_auth");
    assert_eq!(json(&out.stderr)["error"]["code"], "INSUFFICIENT_SCOPE");
}

// ── 因子管理册（/research/factors*, /research/mining*）────────────────────────

#[test]
fn factor_summary_unwraps_research_envelope() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/factors/summary");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"total_routes":2,"total_factors":37,"deleted_factors":1,"total_evaluations":100,"engine_distribution":[],"route_distribution":[],"tag_distribution":[],"generated_at":null}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "summary"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["total_factors"], 37);
    assert!(v.get("code").is_none()); // 已拆封
}

#[test]
fn factor_summary_uses_latest_top_factor_shape() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/factors/summary");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"total_routes":1,"total_factors":1,"deleted_factors":0,"total_evaluations":1,"engine_distribution":[],"route_distribution":[{"route_code":"RT_1","route_name":"路线","engine":"TSA","factor_count":1,"total":1,"avg_sharpe":1.2,"top_factors":[{"factor_name":"TSA_1","sharpe":1.2}]}],"tag_distribution":[],"generated_at":null}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "summary"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let top = &json(&out.stdout)["route_distribution"][0]["top_factors"][0];
    assert_eq!(top["sharpe"], 1.2);
    assert!(top.get("annual_return").is_none());
}

#[test]
fn factor_get_uses_compact_evaluations() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/factors/TSA_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"factor_name":"TSA_1","factor_code":"x","compute_engine":"TSA","engine_full":"TimeSeriesAstEngine","description":"d","creator":null,"create_time":"2026-08-11T00:00:00Z","route":"RT_1","route_name":"路线","is_deleted":false,"delete_reason":null,"tags":[],"evaluations":[{"problem":"P1","method":"M1","status":"ok","sharpe":1.1,"calmar":0.7}]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "get", "TSA_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let evaluation = &json(&out.stdout)["evaluations"][0];
    assert_eq!(evaluation["calmar"], 0.7);
    assert!(evaluation.get("segments").is_none());
}

#[test]
fn factor_list_sends_query_params() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/research/factors")
            .query_param("page", "2")
            .query_param("page_size", "10")
            .query_param("route", "RT_1")
            .query_param("include_deleted", "true");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[],"total":0,"page":2,"page_size":10,"sampled":0}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "factor",
            "list",
            "--route",
            "RT_1",
            "--include-deleted",
            "--page",
            "2",
            "--page-size",
            "10",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["page"], 2);
}

#[test]
fn mining_runs_maps_route_to_route_code_query() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET)
            .path("/research/mining/runs")
            .query_param("route_code", "RT_1");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"items":[],"total":0}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mining", "runs", "--route", "RT_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
}

#[test]
fn mining_overview_uses_evaluate_methods_array() {
    let server = MockServer::start();
    mock_mining_overview(&server, "RUN_1", &[]);
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mining", "overview", "RUN_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let kpi = &json(&out.stdout)["kpi"];
    assert_eq!(kpi["evaluate_methods"], serde_json::json!(["x"]));
    assert!(kpi.get("evaluate_method").is_none());
}

#[test]
fn mining_factors_preserves_median_calmar() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/mining/RUN_1/factors");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[{"agg":{"best_problem":"P1","best_sharpe":1.2,"mean_sharpe":0.8,"median_sharpe":0.7,"median_calmar":0.6,"pos_sharpe_ratio":0.75,"problem_count":4},"compute_engine":"TSA","create_time":"2026-08-11T00:00:00Z","description":"d","eval_count":8,"factor_code":"x","factor_name":"TSA_1","metrics":{"夏普比率":0.8,"卡玛比率":0.6},"problem_count":4}],"page":1,"page_size":5,"total":1}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mining", "factors", "RUN_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["items"][0]["agg"]["median_calmar"], 0.6);
}

#[test]
fn research_retry_after_header_is_preserved() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/research/factors/summary");
        then.status(429)
            .header("Retry-After", "0")
            .body(r#"{"code":42901,"msg":"请求过多"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "summary"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(3);
    assert_eq!(json(&out.stderr)["error"]["retryAfterMs"], 0);
}

#[test]
fn factor_delete_sends_reason_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/factors/TSA_1")
            .json_body(serde_json::json!({"reason":"逻辑不成立"}));
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"factor_name":"TSA_1","is_deleted":true}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "delete", "TSA_1", "--reason", "逻辑不成立"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["is_deleted"], true);
}

#[test]
fn factor_delete_write_503_is_exit_5_and_not_retried() {
    // 写命令即便 action=retry_later 也不重试（不套 with_retry）。
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE).path("/research/factors/TSA_1");
        then.status(503)
            .body(r#"{"code":50301,"msg":"上游不可用"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor", "delete", "TSA_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1);
}

// ── 策略管理册（实盘读 + 状态/标签/promote 写；problem）──────────────

#[test]
fn strategy_get_converts_naive_timestamp_but_leaves_dates_alone() {
    // update_time 是后端唯一见过的「无时区标记」形状（UTC），要换算成东八区；
    // 同一条响应里的 latest_weight_date / outsample_sdt 是交易日语义，移一天就错。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies/TS_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"base_freq":"1D","code":"TS_1","death_time":null,"description":"d","outsample_sdt":"2024-01-01","recent_update":{"last_heartbeat":"2026-07-25T23:00:00Z","latest_weight_date":"2026-07-24"},"status":"暂停","tags":[],"update_time":"2026-07-25 17:20:41","weight_type":"w"}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "get", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["update_time"], "2026-07-26T01:20:41+08:00");
    assert_eq!(
        v["recent_update"]["last_heartbeat"],
        "2026-07-26T07:00:00+08:00"
    );
    // 日期字段原样
    assert_eq!(v["recent_update"]["latest_weight_date"], "2026-07-24");
    assert_eq!(v["outsample_sdt"], "2024-01-01");
    assert!(v["death_time"].is_null());
}

#[test]
fn strategy_trades_kline_key_is_not_rewritten() {
    // kline_key 内嵌时间，但它是要原样回传给 `strategy kline` 的路径参数——
    // 一旦被时区换算改写，那根 K 线就再也查不到。Value 透传块一律不碰。
    let key = "601688.SH|2016-09-28T16:00:00|2016-11-11T16:00:00";
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies/TS_1/trades");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[{"kline_key":"601688.SH|2016-09-28T16:00:00|2016-11-11T16:00:00","entry_time":"2016-09-28T16:00:00"}]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "trades", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v = json(&out.stdout);
    assert_eq!(v["items"][0]["kline_key"], key);
    assert_eq!(v["items"][0]["entry_time"], "2016-09-28T16:00:00");
}

#[test]
fn strategy_list_invalid_status_is_exit_2_before_network() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "list", "--status", "运行中"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn strategy_trades_invalid_kind_is_exit_2_before_network() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "trades", "TS_1", "--kind", "profit"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn mining_factors_rejects_invalid_group_from_run_overview() {
    let server = MockServer::start();
    mock_mining_overview(&server, "RUN_1", &["FTS", "STS"]);
    let factors = server.mock(|when, then| {
        when.method(GET).path("/research/mining/RUN_1/factors");
        then.status(200);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mining", "factors", "RUN_1", "--group", "problem"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    let message = json(&out.stderr)["error"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("FTS"));
    assert!(message.contains("STS"));
    factors.assert_calls(0);
}

#[test]
fn mining_factors_accepts_group_from_run_overview() {
    let server = MockServer::start();
    mock_mining_overview(&server, "RUN_1", &["FTS", "STS"]);
    let factors = server.mock(|when, then| {
        when.method(GET)
            .path("/research/mining/RUN_1/factors")
            .query_param("group", "FTS")
            .query_param("page_size", "100");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[],"page":1,"page_size":100,"total":0}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "mining",
            "factors",
            "RUN_1",
            "--group",
            "FTS",
            "--page-size",
            "100",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(out.status.success());
    factors.assert();
}

#[test]
fn mining_factors_page_size_over_backend_limit_is_exit_2_before_network() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["mining", "factors", "RUN_1", "--page-size", "101"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn strategy_status_patch_sends_body_and_unwraps() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(httpmock::Method::PATCH)
            .path("/strategy/realtime/strategies/TS_1/status")
            .json_body(serde_json::json!({"status":"实盘"}));
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":"TS_1","status":"实盘"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "status", "TS_1", "--status", "实盘"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["status"], "实盘");
}

#[test]
fn strategy_status_invalid_enum_is_exit_2_before_network() {
    // 非法枚举本地失败，不发网络（无 mock）。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "status", "TS_1", "--status", "涨停"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["kind"], "args");
}

#[test]
fn strategy_status_write_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(httpmock::Method::PATCH)
            .path("/strategy/realtime/strategies/TS_1/status");
        then.status(503)
            .body(r#"{"code":50301,"msg":"上游不可用"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "status", "TS_1", "--status", "暂停"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1); // 写不重试
}

#[test]
fn strategy_tag_rm_delete_no_body() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/strategies/TS_1/tags/momentum");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":"TS_1","tag":"momentum"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "tag-rm", "TS_1", "momentum"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["tag"], "momentum");
}

#[test]
fn experiment_delete_strategy_sends_delete_and_unwraps() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/experiments/E1/strategies/TS_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"experiment_id":"E1","strategy_code":"TS_1","deleted":true}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["experiment", "delete", "E1", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    let body = json(&out.stdout);
    assert_eq!(body["experiment_id"], "E1");
    assert_eq!(body["strategy_code"], "TS_1");
    assert_eq!(body["deleted"], true);
}

#[test]
fn experiment_delete_strategy_write_503_is_exit_5_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/experiments/E1/strategies/TS_1");
        then.status(503)
            .body(r#"{"code":50301,"msg":"上游不可用"}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["experiment", "delete", "E1", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    m.assert_calls(1);
}

#[test]
fn promote_start_posts_empty_body_and_not_retried() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/research/experiments/E1/strategies/TS_1/promote")
            // 不传 --memo 时请求体必须逐字还是 `{}`，不能发 `{"memo":null}`——
            // 后端加 memo 字段前后的请求体要完全一致，否则老部署会拿到不认识的键。
            .json_body(serde_json::json!({}));
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"promotion_id":"P1","experiment_id":"E1","strategy_code":"TS_1","status":"running","phase":"realtime_running","registered":true,"lifecycle":null,"realtime":null,"error":null,"created_at":"2026-07-24T00:00:00Z","updated_at":"2026-07-24T00:00:00Z"}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["promote", "start", "E1", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["status"], "running");
}

#[test]
fn promote_start_sends_memo_when_given() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/research/experiments/E1/strategies/TS_1/promote")
            .json_body(serde_json::json!({"memo":"入库理由：样本外夏普 1.8"}));
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"promotion_id":"P1","experiment_id":"E1","strategy_code":"TS_1","status":"running","phase":"realtime_running","registered":true,"lifecycle":null,"realtime":null,"error":null,"created_at":"2026-07-24T00:00:00Z","updated_at":"2026-07-24T00:00:00Z"}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "promote",
            "start",
            "E1",
            "TS_1",
            "--memo",
            "入库理由：样本外夏普 1.8",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
}

#[test]
fn promote_start_rejects_blank_memo_before_network() {
    // 空白 --memo 发上去后端会当没传，agent 却以为写成功了——静默无操作比报错难查，
    // 所以本地就拦掉（且不发网络：没有 mock 也不该有请求）。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["promote", "start", "E1", "TS_1", "--memo", "   "])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59931")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

/// 一份最小但完整的策略定义（七个必需字段齐全），JSON 形态。
/// `problem.suffix` 故意是 null——`strategy definition` 的真实输出里就有，
/// 它是 TOML 表示不了的那个值，必须被剥掉才转得过去。
fn minimal_definition_json() -> serde_json::Value {
    serde_json::json!({
        "factors": [{"factor_name":"F1","factor_code":"$close","compute_engine":"E","route":"r1"}],
        "model_config": {"model":"TS003","name":"TS003","kwargs":{}},
        "post_process": "WEIGHT",
        "problem": {"code":"P1","freq":"1D","problem_type":"ts","suffix": null},
        "route": "r1",
        "runtime": {"update_mode":"auto"},
        "strategy": "STS_1D_NEW"
    })
}

/// httpmock 0.8 没有「取回已收请求」的接口，只能靠 `is_true` 匹配器在服务端线程里
/// 顺手把 body 抄一份出来（匹配器恒真，断言留到测试线程做，失败信息才有用）。
fn capture_body() -> (
    std::sync::Arc<std::sync::Mutex<Option<String>>>,
    impl Fn(&httpmock::prelude::HttpMockRequest) -> bool + Send + Sync + 'static,
) {
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let w = slot.clone();
    (slot, move |req: &httpmock::prelude::HttpMockRequest| {
        *w.lock().unwrap() = Some(req.body_string());
        true
    })
}

#[test]
fn strategy_register_converts_json_to_toml() {
    let server = MockServer::start();
    let (body, cap) = capture_body();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/research/strategy-imports")
            .is_true(cap);
        then.status(200).body(r#"{"code":0,"msg":"ok","data":{"total":1,"inserted":1,"existing":0,"items":[{"strategy_code":"STS_1D_NEW","inserted":true,"lifecycle":"暂停","toml_sha256":"abc"}]}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(minimal_definition_json().to_string())
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    let result = json(&out.stdout);
    assert_eq!(result["total"], 1);
    assert_eq!(result["inserted"], 1);
    assert_eq!(result["existing"], 0);
    assert_eq!(result["items"][0]["strategy_code"], "STS_1D_NEW");

    // 发出去的必须是 TOML 文本（不是 JSON 透传），且 null 已被剥掉——
    // 不剥的话 toml::to_string 会直接报 unsupported unit type。
    let sent: serde_json::Value =
        serde_json::from_str(body.lock().unwrap().as_ref().unwrap()).unwrap();
    let toml_text = sent["tomls"][0].as_str().expect("tomls[0] 必须是字符串");
    assert!(sent.get("toml").is_none());
    assert!(sent.get("run_realtime").is_none());
    assert!(
        !toml_text.contains("suffix"),
        "null 字段应被剥掉：{toml_text}"
    );
    let back: toml::Value = toml::from_str(toml_text).expect("发出去的必须是合法 TOML");
    let t = back.as_table().unwrap();
    for k in [
        "strategy",
        "problem",
        "runtime",
        "model_config",
        "post_process",
        "route",
        "factors",
    ] {
        assert!(t.contains_key(k), "转换后丢了必需字段 {k}");
    }
}

#[test]
fn strategy_register_passes_raw_toml_through() {
    let server = MockServer::start();
    let (body, cap) = capture_body();
    server.mock(|when, then| {
        when.method(POST)
            .path("/research/strategy-imports")
            .is_true(cap);
        then.status(200).body(r#"{"code":0,"msg":"ok","data":{"total":1,"inserted":1,"existing":0,"items":[{"strategy_code":"S1","inserted":true,"lifecycle":"暂停","toml_sha256":"abc"}]}}"#);
    });
    let raw = "strategy = \"S1\"\npost_process = \"WEIGHT\"\nroute = \"r1\"\nfactors = []\n\
               \n[problem]\nfreq = \"1D\"\nproblem_type = \"ts\"\n\
               \n[runtime]\nupdate_mode = \"auto\"\n\
               \n[model_config]\nmodel = \"TS003\"\n";
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin(raw)
        .output()
        .unwrap();
    assert!(out.status.success());
    // 裸 TOML 必须原样上传，不做任何往返改写。
    let sent: serde_json::Value =
        serde_json::from_str(body.lock().unwrap().as_ref().unwrap()).unwrap();
    assert_eq!(sent["tomls"][0], raw);
}

#[test]
fn strategy_register_batches_files_in_one_request() {
    let server = MockServer::start();
    let (body, cap) = capture_body();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/research/strategy-imports")
            .is_true(cap);
        then.status(200).body(r#"{"code":0,"msg":"ok","data":{"total":2,"inserted":1,"existing":1,"items":[{"strategy_code":"STS_1D_NEW","inserted":false,"lifecycle":"实盘","toml_sha256":"aaa"},{"strategy_code":"STS_1D_SECOND","inserted":true,"lifecycle":"暂停","toml_sha256":"bbb"}]}}"#);
    });

    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first.json");
    let second = tmp.path().join("second.json");
    std::fs::write(&first, minimal_definition_json().to_string()).unwrap();
    let mut second_definition = minimal_definition_json();
    second_definition["strategy"] = serde_json::json!("STS_1D_SECOND");
    std::fs::write(&second, second_definition.to_string()).unwrap();

    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("strategy")
        .arg("register")
        .arg(&first)
        .arg(&second)
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    mock.assert();

    let sent: serde_json::Value =
        serde_json::from_str(body.lock().unwrap().as_ref().unwrap()).unwrap();
    assert_eq!(sent["tomls"].as_array().unwrap().len(), 2);
    let first_toml: toml::Value = toml::from_str(sent["tomls"][0].as_str().unwrap()).unwrap();
    let second_toml: toml::Value = toml::from_str(sent["tomls"][1].as_str().unwrap()).unwrap();
    assert_eq!(first_toml["strategy"].as_str(), Some("STS_1D_NEW"));
    assert_eq!(second_toml["strategy"].as_str(), Some("STS_1D_SECOND"));

    let v = json(&out.stdout);
    assert_eq!(v["total"], 2);
    assert_eq!(v["inserted"], 1);
    assert_eq!(v["existing"], 1);
    assert_eq!(v["items"][0]["lifecycle"], "实盘");
    assert_eq!(v["items"][1]["strategy_code"], "STS_1D_SECOND");
}

#[test]
fn strategy_register_rejects_more_than_100_files_before_reading() {
    let cfg = config_with_token("sk_test");
    let mut cmd = skz(&cfg);
    cmd.arg("strategy").arg("register");
    for index in 0..101 {
        cmd.arg(format!("missing-{index}.toml"));
    }
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_params");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("最多读取 100")
    );
}

#[test]
fn strategy_register_rejects_removed_realtime_flag() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register", "--realtime"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_params");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--realtime")
    );
}

#[test]
fn strategy_register_rejects_missing_fields_before_network() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59935")
        .write_stdin(r#"{"strategy":"X","route":"r1"}"#)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let msg = json(&out.stderr)["error"]["message"].to_string();
    // 报「缺哪几个」，比后端笼统的 42201 可操作
    for k in [
        "problem",
        "runtime",
        "model_config",
        "post_process",
        "factors",
    ] {
        assert!(msg.contains(k), "报错该点名缺失的 {k}：{msg}");
    }
}

#[test]
fn strategy_register_rejects_unparseable_stdin() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59936")
        .write_stdin("这既不是 JSON 也不是 TOML: {{{")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn strategy_register_transport_error_is_check_existing() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "register"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59937")
        .write_stdin(minimal_definition_json().to_string())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "check_existing");
    assert_eq!(v["error"]["remediation"]["verifyWith"], "skz strategy list");
}

#[test]
fn strategy_memo_writes_from_stdin() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(PATCH)
            // memo 住在 research 面，不是 status 那个 /strategy/realtime/* 包装口
            // （那是另一个下游服务，打过去必然 404）。
            .path("/research/strategies/TS_1/memo")
            // 首尾空白由本地 trim 掉后再发，跟后端归一化同序。
            .json_body(serde_json::json!({"memo":"近 20 日回撤 -18%\n等下周复盘"}));
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":"TS_1","memo":"近 20 日回撤 -18%\n等下周复盘"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "memo", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin("  近 20 日回撤 -18%\n等下周复盘\n  ")
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["memo"], "近 20 日回撤 -18%\n等下周复盘");
}

#[test]
fn strategy_memo_clear_sends_empty_string() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(PATCH)
            .path("/research/strategies/TS_1/memo")
            .json_body(serde_json::json!({"memo":""}));
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":"TS_1","memo":""}}"#);
    });
    let cfg = config_with_token("sk_test");
    // --clear 时不读 stdin：这里故意喂一段正文，证明它被忽略而不是拼进请求。
    let out = skz(&cfg)
        .args(["strategy", "memo", "TS_1", "--clear"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin("这段不该被发出去")
        .output()
        .unwrap();
    assert!(out.status.success());
    m.assert();
    assert_eq!(json(&out.stdout)["memo"], "");
}

#[test]
fn strategy_memo_empty_stdin_is_not_a_clear() {
    // 关键防呆：空管道（上游命令没输出 / < /dev/null）绝不能被当成「清除」，
    // 那是不可恢复的覆盖写。必须 exit 2 并把人指向显式 --clear。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "memo", "TS_1"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59932")
        .write_stdin("   \n  ")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "fix_params");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("--clear"),
        "空 stdin 的报错必须指向 --clear，否则用户不知道怎么清除：{v}"
    );
}

#[test]
fn strategy_memo_rejects_overlong_by_chars_not_bytes() {
    // 上限按 Unicode 字符计。10000 个中文 = 30000 字节，用 len() 判会误拦；
    // 这里正好 10000 个字符必须放行，10001 个才拦。
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(PATCH).path("/research/strategies/TS_1/memo");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"code":"TS_1","memo":"x"}}"#);
    });
    let cfg = config_with_token("sk_test");
    let ok = skz(&cfg)
        .args(["strategy", "memo", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .write_stdin("测".repeat(10_000))
        .output()
        .unwrap();
    assert!(ok.status.success(), "10000 个中文字符应放行");
    m.assert();

    // 超限走本地拦截，不发网络（指向没人监听的端口，有请求就会变成 exit 7）。
    let over = skz(&cfg)
        .args(["strategy", "memo", "TS_1"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59933")
        .write_stdin("测".repeat(10_001))
        .output()
        .unwrap();
    assert_eq!(over.status.code(), Some(2));
    assert_eq!(json(&over.stderr)["error"]["action"], "fix_params");
}

#[test]
fn strategy_memo_transport_error_is_check_existing() {
    // memo 是幂等覆盖写、重发无害，但仍归 check_existing 而不是 retry_later——
    // 「写一律不重试」是零例外规则，见 CLAUDE.md 不变量 1。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "memo", "TS_1"])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59934")
        .write_stdin("笔记")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    let v = json(&out.stderr);
    assert_eq!(v["error"]["action"], "check_existing");
    assert_eq!(v["error"]["retryable"], false);
    assert_eq!(
        v["error"]["remediation"]["verifyWith"],
        "skz strategy get <code>"
    );
}

#[test]
fn strategy_get_surfaces_memo() {
    // 回归：models 里漏掉 memo 字段时 serde 会静默丢弃（项目没有 deny_unknown_fields），
    // 命令照样 exit 0，只是笔记凭空消失——只有断言字段本身能抓到。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies/TS_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"code":"TS_1","base_freq":"日线","status":"暂停","description":"d","memo":"回撤超限，观察中","weight_type":"ts","outsample_sdt":"2025-01-01 00:00:00","update_time":"2026-07-31 12:00:00","tags":[],"recent_update":{"last_heartbeat":null,"latest_weight_date":null}}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "get", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["memo"], "回撤超限，观察中");
}

#[test]
fn strategy_list_surfaces_memo() {
    // 列表侧的 memo 是后端后补的（先有详情、后进列表），同样会被漏掉的 model 静默丢弃。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[{"code":"TS_1","base_freq":"日线","status":"暂停","description":"d","memo":"观察中","factor_route":"ROUTE_A","weight_type":"ts","outsample_sdt":null,"last_heartbeat":null,"latest_weight_date":null,"tags":[]}],"page":1,"page_size":20,"total":1,"status_counts":{}}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "list"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["items"][0]["memo"], "观察中");
    assert_eq!(json(&out.stdout)["items"][0]["factor_route"], "ROUTE_A");
}

#[test]
fn strategy_latest_positions_selects_view_and_converts_update_time() {
    let server = MockServer::start();
    let ts = server.mock(|when, then| {
        when.method(GET)
            .path("/research/strategies/positions/latest")
            .query_param("weight_type", "ts");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"items":[{"dt":"2026-08-10 00:00:00","symbol":"AAA","weight":0.0,"strategy":"TS_1","update_time":"2026-08-10 08:30:00"},{"dt":"2026-08-09 00:00:00","symbol":"BBB","weight":-0.25,"strategy":"TS_1","update_time":null}]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "latest-positions", "--weight-type", "ts"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    let body = json(&out.stdout);
    assert_eq!(body["items"][0]["dt"], "2026-08-10 00:00:00");
    assert_eq!(body["items"][0]["weight"], 0.0);
    assert_eq!(body["items"][0]["update_time"], "2026-08-10T16:30:00+08:00");
    assert!(body["items"][1]["update_time"].is_null());
    ts.assert_calls(1);

    let cs = server.mock(|when, then| {
        when.method(GET)
            .path("/research/strategies/positions/latest")
            .query_param("weight_type", "cs");
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"items":[]}}"#);
    });
    let out = skz(&cfg)
        .args(["strategy", "latest-positions", "--weight-type", "cs"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(json(&out.stdout)["items"].as_array().unwrap().is_empty());
    cs.assert_calls(1);
}

#[test]
fn strategy_latest_positions_rejects_unknown_weight_type_before_request() {
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "strategy",
            "latest-positions",
            "--weight-type",
            "long_short",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn research_write_40909_is_exit_5_without_retry() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(PATCH).path("/research/strategies/TS_1/memo");
        then.status(409)
            .body(r#"{"code":40909,"msg":"工作区正在初始化，请稍后重试","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "memo", "TS_1", "--clear"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(json(&out.stderr)["error"]["action"], "retry_later");
    m.assert_calls(1);
}

#[test]
fn strategy_get_tolerates_missing_memo() {
    // 老部署（未上 memo 的后端）不返回这个字段，不能因此解析失败 → exit 6。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies/TS_1");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"code":"TS_1","base_freq":"日线","status":"暂停","description":"d","weight_type":"ts","outsample_sdt":null,"update_time":null,"tags":[],"recent_update":{"last_heartbeat":null,"latest_weight_date":null}}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "get", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["memo"], "");
}

#[test]
fn http_413_is_fix_params_not_internal() {
    // 请求体超上限是「改小输入就能过」。落到 _ => Internal(exit 6) 会让 agent
    // 当成内部故障放弃。本地预检通常先拦一道，这里是防御性兜底。
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/strategies/TS_1");
        then.status(413)
            .body(r#"{"code":41300,"msg":"payload too large","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "get", "TS_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
}

#[test]
fn problem_meta_unwraps() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/problems/meta");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"code_rule":"x","dataset_options":[{"label":"股票","value":"stock"}],"default_time_segments":[],"freq_options":[],"max_time_segment_date":"20250701","problem_types":[],"required_segments":[]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "meta"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(json(&out.stdout)["dataset_options"][0]["value"], "stock");
    assert_eq!(json(&out.stdout)["max_time_segment_date"], "20250701");
}

#[test]
fn problem_meta_accepts_legacy_response_without_max_date() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/research/problems/meta");
        then.status(200).body(
            r#"{"code":0,"msg":"ok","data":{"dataset_options":[],"default_time_segments":[],"freq_options":[],"problem_types":[],"required_segments":[]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["problem", "meta"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(json(&out.stdout).get("max_time_segment_date").is_none());
}

// ── 只读模式（SKZ_READ_ONLY）────────────────────────────────────────
// 断言的重点是**请求没发出去**（`assert_calls(0)` / hits==0），不是退出码。
// 退出码只说明我们报了个错，唯有"零请求"能证明闸真的挡在了网络之前——
// 而"没花掉别人的钱"这件事，靠的正是后者。

/// 过得了 `strategy register` 本地必填字段校验的最小定义。
const MINIMAL_STRATEGY_TOML: &str = "\
[strategy]\n\
[problem]\n\
[runtime]\n\
[model_config]\n\
[post_process]\n\
[route]\n\
[factors]\n";

/// 过得了本地形态校验（32 位小写 hex）的赠予码。
const GIFT_CODE: &str = "0123456789abcdef0123456789abcdef";

/// 一个匹配任意路径的兜底 mock：只要 CLI 发出过任何请求，它的 hits 就非零。
fn catch_all<'a>(server: &'a MockServer) -> Mock<'a> {
    server.mock(|when, then| {
        when.any_request();
        then.status(200).body("{}");
    })
}

/// 全部写/触发命令。加新写命令时**同步加到这里**——这张表就是「新写命令有没有过闸」
/// 的唯一机械检查，漏登记等于漏掉一次可能花掉别人钱的调用。
fn write_commands() -> Vec<(&'static str, Vec<&'static str>, &'static str)> {
    vec![
        ("route create", vec!["route", "create"], r#"{"name":"x"}"#),
        (
            "problem create",
            vec!["problem", "create"],
            r#"{"name":"x"}"#,
        ),
        ("problem delete", vec!["problem", "delete", "PB_1"], ""),
        ("mine start", vec!["mine", "start", "--route", "RT_1"], ""),
        (
            "explore start",
            vec!["explore", "start", "--problem", "PB_1", "--route", "RT_1"],
            "",
        ),
        (
            "promote start",
            vec!["promote", "start", "EXP_1", "ST_1"],
            "",
        ),
        (
            "portfolio create",
            vec!["portfolio", "create"],
            r#"{"portfolio_code":"PF_1","candidate_strategies":["STS_1"],"rebalance_dates":["2025-01-01"],"base_market":"stock"}"#,
        ),
        ("factor delete", vec!["factor", "delete", "FT_1"], ""),
        (
            "experiment delete",
            vec!["experiment", "delete", "EXP_1", "ST_1"],
            "",
        ),
        (
            "experiment delete-run",
            vec!["experiment", "delete-run", "EXP_1"],
            "",
        ),
        // 不带 `--dry-run` 的形态才是写；`--dry-run` 是显式例外，
        // 由 `read_only_still_allows_factor_route_dry_run` 单独盯。
        (
            "factor-routes delete",
            vec!["factor-routes", "delete", "RT_1"],
            "",
        ),
        (
            "gift create",
            vec!["gift", "create", "--strategy", "STS_1", "--max-claims", "3"],
            "",
        ),
        ("gift revoke", vec!["gift", "revoke", GIFT_CODE], ""),
        ("gift claim", vec!["gift", "claim", GIFT_CODE], ""),
        (
            "strategy status",
            vec!["strategy", "status", "ST_1", "--status", "实盘"],
            "",
        ),
        (
            "strategy tag-add",
            vec!["strategy", "tag-add", "ST_1", "--tag", "t"],
            "",
        ),
        (
            "strategy tag-rm",
            vec!["strategy", "tag-rm", "ST_1", "t"],
            "",
        ),
        (
            "strategy memo",
            vec!["strategy", "memo", "ST_1"],
            "一行笔记",
        ),
        // register 的 stdin 要过本地必填字段校验才走到闸（见下面 `..._after_local_validation`）。
        (
            "strategy register",
            vec!["strategy", "register"],
            MINIMAL_STRATEGY_TOML,
        ),
    ]
}

#[test]
fn read_only_refuses_every_write_without_sending_a_request() {
    for (label, args, stdin) in write_commands() {
        let server = MockServer::start();
        let any = catch_all(&server);
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(&args)
            .env("SKZ_BASE_URL", server.base_url())
            .env("SKZ_READ_ONLY", "1")
            .write_stdin(stdin)
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(8), "{label} 应当是 exit 8");
        let body = json(&out.stderr);
        assert_eq!(body["error"]["action"], "not_permitted", "{label}");
        assert_eq!(body["error"]["retryable"], false, "{label}");
        any.assert_calls(0);
    }
}

#[test]
fn read_only_still_allows_reads() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/market/markets");
        then.status(200)
            .body(r#"[{"market":"stock","count":5464}]"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .arg("markets")
        .env("SKZ_BASE_URL", server.base_url())
        .env("SKZ_READ_ONLY", "1")
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
}

/// 两个 `poll` 是 POST 但语义为读，只读模式必须照常放行——否则等于把"看看我之前
/// 那个付费任务跑完没有"也一起封了，而那恰恰是只读用户最需要的能力。
#[test]
fn read_only_still_allows_post_shaped_polls() {
    for (args, path) in [
        (["mine", "poll", "fc1"], "/strategy/miner/poll"),
        (["explore", "poll", "fc1"], "/strategy/explore/poll"),
    ] {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST).path(path);
            then.status(200)
                .body(r#"[{"fcRunId":"fc1","status":"done","done":true,"ok":true}]"#);
        });
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(args)
            .env("SKZ_BASE_URL", server.base_url())
            .env("SKZ_READ_ONLY", "1")
            .output()
            .unwrap();
        m.assert();
        assert!(out.status.success(), "{path} 在只读模式下应当放行");
    }
}

/// `factor-routes delete --dry-run` 是 DELETE 但后端零修改，只读模式必须放行——
/// 只读模式的动机就是"让人看清代价再决定"，把这份代价预告一并封掉恰好封掉了它自己想要的东西。
/// 同一条命令去掉 `--dry-run` 必须仍被拦（在 `write_commands()` 表里）。
#[test]
fn read_only_still_allows_factor_route_dry_run() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/factor-routes/RT_1")
            .query_param("dry_run", "true");
        then.status(200).body(
            r#"{"code":0,"msg":"预演：未做任何修改","data":{"route_code":"RT_1","deleted":false,"dry_run":true,"mining_runs":3,"failed_mining_runs":[],"orphaned_factors":12}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor-routes", "delete", "RT_1", "--dry-run"])
        .env("SKZ_BASE_URL", server.base_url())
        .env("SKZ_READ_ONLY", "1")
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let body = json(&out.stdout);
    assert_eq!(body["deleted"], false);
    assert_eq!(body["mining_runs"], 3);
    assert_eq!(body["orphaned_factors"], 12);
}

/// 删除接口的两条软护栏（40906/40907）仍走 exit 7，但必须带 remediation 讲清"确认后带
/// --force 重发"——否则 `check_existing` 的字面意思（别重发）会把 agent 引向死角。
#[test]
fn soft_guardrail_conflict_carries_force_remediation() {
    for (code, args) in [
        (40907, vec!["factor-routes", "delete", "RT_1"]),
        (40906, vec!["experiment", "delete-run", "EXP_1"]),
    ] {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(DELETE);
            then.status(409)
                .body(format!(r#"{{"code":{code},"msg":"护栏","data":null}}"#));
        });
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(&args)
            .env("SKZ_BASE_URL", server.base_url())
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(7), "{code} 应当是 exit 7");
        let body = json(&out.stderr);
        assert_eq!(body["error"]["action"], "check_existing", "{code}");
        let howto = body["error"]["remediation"]["howTo"]
            .as_str()
            .unwrap_or_default();
        assert!(
            howto.contains("--force"),
            "{code} remediation 要点明 --force"
        );
    }
}

/// 普通 409（非软护栏码）不能被误挂上"带 --force 重发"的建议——40905「实盘更新任务正在跑」
/// 是硬拒绝，force 越不过去，给它这条建议等于教 agent 撞墙。
#[test]
fn hard_conflict_has_no_force_remediation() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(DELETE);
        then.status(409)
            .body(r#"{"code":40905,"msg":"该探索有实盘更新任务正在运行","data":null}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["experiment", "delete-run", "EXP_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(7));
    let body = json(&out.stderr);
    assert_eq!(body["error"]["remediation"], serde_json::Value::Null);
}

/// 删路线部分失败：路线行已删、个别执行目录没清掉，后端仍回 200。退出码保持 0（用户意图达成、
/// 重发即续删），所以 `failed_mining_runs` 必须原样透出——它是 agent 唯一能看见这件事的地方。
#[test]
fn factor_route_delete_surfaces_partial_failure_at_exit_zero() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/factor-routes/RT_1")
            .query_param("force", "true");
        then.status(200).body(
            r#"{"code":0,"msg":"路线已删除，部分挖掘执行未能清理，可重试","data":{"route_code":"RT_1","deleted":true,"dry_run":false,"mining_runs":2,"failed_mining_runs":["RT_1_20250101"],"orphaned_factors":0}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["factor-routes", "delete", "RT_1", "--force"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert_eq!(out.status.code(), Some(0));
    let body = json(&out.stdout);
    assert_eq!(body["deleted"], true);
    assert_eq!(body["failed_mining_runs"][0], "RT_1_20250101");
}

/// 不传 `--force` / `--dry-run` 时不发这两个 query 键（后端 `#[serde(default)]` 本就默认 false，
/// 显式发一个默认值只会让请求日志更难读）。
#[test]
fn delete_flags_absent_when_not_requested() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(DELETE)
            .path("/research/experiments/EXP_1")
            .is_true(|req| req.query_params().is_empty());
        then.status(200)
            .body(r#"{"code":0,"msg":"ok","data":{"experiment_id":"EXP_1","deleted":true}}"#);
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["experiment", "delete-run", "EXP_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
}

/// `SKZ_READ_ONLY=0` 报错而不是"关闭只读"。这条测的是防绕过：agent 撞到 exit 8 后
/// 最顺手的下一步就是 `SKZ_READ_ONLY=0 skz ...` 再试一次，这里必须拦住。
#[test]
fn read_only_falsey_values_are_errors_not_an_off_switch() {
    for value in ["0", "false", "off", "no", "ture"] {
        let server = MockServer::start();
        let any = catch_all(&server);
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["mine", "start", "--route", "RT_1"])
            .env("SKZ_BASE_URL", server.base_url())
            .env("SKZ_READ_ONLY", value)
            .write_stdin("")
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "SKZ_READ_ONLY={value:?} 应当报错"
        );
        assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
        any.assert_calls(0);
    }
}

#[test]
fn read_only_truthy_values_are_case_insensitive() {
    for value in ["1", "true", "TRUE", "On", " true "] {
        let server = MockServer::start();
        let any = catch_all(&server);
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["factor", "delete", "FT_1"])
            .env("SKZ_BASE_URL", server.base_url())
            .env("SKZ_READ_ONLY", value)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(8),
            "SKZ_READ_ONLY={value:?} 应当开启只读"
        );
        any.assert_calls(0);
    }
}

/// 变量不设 / 设成空 = 关闭。空串这条要单独钉：shell 里 `export SKZ_READ_ONLY=`
/// 是常见的"我不想要它"写法，不该变成报错。
#[test]
fn read_only_unset_or_empty_is_off() {
    for value in [None, Some("")] {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(GET).path("/market/markets");
            then.status(200).body(r#"[{"market":"stock","count":1}]"#);
        });
        let cfg = config_with_token("sk_test");
        let mut cmd = skz(&cfg);
        cmd.arg("markets").env("SKZ_BASE_URL", server.base_url());
        if let Some(v) = value {
            cmd.env("SKZ_READ_ONLY", v);
        }
        let out = cmd.output().unwrap();
        m.assert();
        assert!(out.status.success(), "SKZ_READ_ONLY={value:?} 应当是关闭态");
    }
}

/// `auth status` 是只读闸唯一的验证手段（变量名打错会静默失效），必须如实反映。
#[test]
fn auth_status_reports_read_only_mode() {
    let cfg = config_with_token("sk_test");
    let on = skz(&cfg)
        .args(["auth", "status"])
        .env("SKZ_READ_ONLY", "1")
        .output()
        .unwrap();
    assert!(on.status.success());
    assert_eq!(json(&on.stdout)["readOnly"], true);
    assert_eq!(json(&on.stdout)["present"], true);

    let off = skz(&cfg).args(["auth", "status"]).output().unwrap();
    assert_eq!(json(&off.stdout)["readOnly"], false);
}

/// remediation 是防绕过的一部分：agent 撞到工具报错的默认反应是换条路达成目标，
/// 而它手上有 shell、token 又在它读得到的文件里。少给一条线索就少一条路。
#[test]
fn read_only_remediation_leaks_no_endpoint_or_credential_hint() {
    let server = MockServer::start();
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["promote", "start", "EXP_1", "ST_1"])
        .env("SKZ_BASE_URL", server.base_url())
        .env("SKZ_READ_ONLY", "1")
        .output()
        .unwrap();
    let rendered = String::from_utf8(out.stderr).unwrap();
    for leak in [
        &server.base_url()[..],
        "credentials",
        "/research/",
        "/strategy/",
        "auth set",
        "SKZ_READ_ONLY",
    ] {
        assert!(
            !rendered.contains(leak),
            "只读 remediation 不该出现 {leak:?}：{rendered}"
        );
    }
}

/// 本地参数校验排在只读闸**前面**（闸在 client 传输层，stdin 解析/枚举校验在它上游）。
/// 所以只读机器上一条参数也错的写命令会先拿到 exit 2、改对了才拿到 exit 8。
/// 这是有意接受的：把闸提前到每条命令的入口就得维护一份命令清单，而那份清单一旦
/// 漏登记就是漏出一次真的写——比多一轮往返危险得多。钱始终没花出去，代价只是一次空转。
#[test]
fn local_validation_precedes_the_read_only_gate() {
    let server = MockServer::start();
    let any = catch_all(&server);
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["strategy", "status", "ST_1", "--status", "不存在的状态"])
        .env("SKZ_BASE_URL", server.base_url())
        .env("SKZ_READ_ONLY", "1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
    any.assert_calls(0); // 两道防线谁先响都行，唯一不能变的是请求没发出去
}

// ── 策略赠予（gift）────────────────────────────────────────────────

/// 发码：三个上限都是固定值域，本地枚举先拦一道，不为它发网络。
#[test]
fn gift_create_validates_fixed_bounds_locally() {
    // 「11 条策略」这组要每条值都不同——本地先去重再判上限，全传同一个值只会剩 1 条。
    let mut too_many: Vec<String> = vec!["gift".into(), "create".into()];
    for i in 0..11 {
        too_many.push("--strategy".into());
        too_many.push(format!("STS_{i}"));
    }
    too_many.extend(["--max-claims".into(), "1".into()]);

    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "没给策略",
            ["gift", "create", "--max-claims", "1"]
                .map(String::from)
                .to_vec(),
        ),
        ("超过 10 条", too_many),
        (
            "名额越界",
            [
                "gift",
                "create",
                "--strategy",
                "STS_1",
                "--max-claims",
                "101",
            ]
            .map(String::from)
            .to_vec(),
        ),
        (
            "ttl 不在枚举里",
            [
                "gift",
                "create",
                "--strategy",
                "STS_1",
                "--max-claims",
                "1",
                "--ttl-days",
                "2",
            ]
            .map(String::from)
            .to_vec(),
        ),
    ];

    for (label, args) in cases {
        let server = MockServer::start();
        let any = catch_all(&server);
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(&args)
            .env("SKZ_BASE_URL", server.base_url())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{label} 应当是 exit 2");
        any.assert_calls(0);
    }
}

/// 码形态本地校验：手滑贴少几位不该变成一次往返 + 一个含糊的「不存在或已过期」。
#[test]
fn gift_code_shape_is_validated_before_any_request() {
    for bad in ["abc", "0123456789ABCDEF0123456789ABCDEF"] {
        let server = MockServer::start();
        let any = catch_all(&server);
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["gift", "preview", bad])
            .env("SKZ_BASE_URL", server.base_url())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{bad}");
        assert_eq!(json(&out.stderr)["error"]["action"], "fix_params");
        any.assert_calls(0);
    }
}

#[test]
fn gift_create_posts_deduped_codes_and_unwraps_envelope() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path("/research/gifts")
            .json_body(serde_json::json!({
                "strategy_codes": ["STS_1", "STS_2"],
                "max_claims": 3,
                "ttl_days": 7
            }));
        then.status(200).body(
            r#"{"code":0,"msg":"赠予码已生成","data":{"gift_code":"0123456789abcdef0123456789abcdef","strategy_codes":["STS_1","STS_2"],"max_claims":3,"claimed":0,"ttl_days":7,"created_at":"2026-08-06T03:00:00Z","expires_at":"2026-08-13T03:00:00Z","unavailable_strategy_codes":[]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args([
            "gift",
            "create",
            "--strategy",
            "STS_1",
            "--strategy",
            "STS_2",
            // 重复项本地去重后才发出去，否则后端会按 10 条上限把它算进去
            "--strategy",
            "STS_1",
            "--max-claims",
            "3",
            "--ttl-days",
            "7",
        ])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let body = json(&out.stdout);
    assert_eq!(body["gift_code"], "0123456789abcdef0123456789abcdef");
    // 事件时刻换算成东八区（+8h）；`gift_code` 是 hex 串，不受影响。
    assert_eq!(body["created_at"], "2026-08-06T11:00:00+08:00");
    assert_eq!(body["expires_at"], "2026-08-13T11:00:00+08:00");
}

#[test]
fn gift_claim_surfaces_renamed_local_codes() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/research/gifts/{GIFT_CODE}/claim"));
        then.status(200).body(
            r#"{"code":0,"msg":"赠予策略已入库","data":{"from_user_id":"u_a","items":[{"origin_strategy_code":"STS_1","strategy_code":"STS_1_G1","inserted":true,"renamed":true}]}}"#,
        );
    });
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["gift", "claim", GIFT_CODE])
        .env("SKZ_BASE_URL", server.base_url())
        .output()
        .unwrap();
    m.assert();
    assert!(out.status.success());
    let body = json(&out.stdout);
    assert_eq!(body["items"][0]["strategy_code"], "STS_1_G1");
    assert_eq!(body["items"][0]["renamed"], true);
}

/// 领取的两个 409 与删除类命令的软护栏**数字撞车**（40907），语义却相反：
/// 那边可以 `--force` 越过，这边压根没有 force 一说。remediation 按端点挂，不按裸数字挂——
/// 挂错的代价是教 agent 去 force 一个 force 不了的东西。
#[test]
fn gift_claim_conflicts_never_suggest_force() {
    for (code, expect) in [(40907, "领完"), (40908, "退避")] {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(POST);
            then.status(409)
                .body(format!(r#"{{"code":{code},"msg":"冲突","data":null}}"#));
        });
        let cfg = config_with_token("sk_test");
        let out = skz(&cfg)
            .args(["gift", "claim", GIFT_CODE])
            .env("SKZ_BASE_URL", server.base_url())
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(7), "{code}");
        let body = json(&out.stderr);
        assert_eq!(body["error"]["action"], "check_existing", "{code}");
        let rendered = body["error"]["remediation"].to_string();
        assert!(rendered.contains(expect), "{code} remediation: {rendered}");
        assert!(
            !rendered.contains("--force"),
            "{code} 领取没有 force 一说，不该出现 --force：{rendered}"
        );
    }
}

/// 写超时 → exit 7 + `verifyWith`。领取的验证器是 `gift preview`（看 `already_claimed`），
/// 不是翻策略库——撞名时落地编号带 `_G{n}` 后缀，照原编号找会找不到。
#[test]
fn gift_claim_timeout_verifies_with_preview() {
    // 打一个没人监听的 loopback 端口 → 连接被拒，走 Error::Network 分支（同 route create 那条）。
    let cfg = config_with_token("sk_test");
    let out = skz(&cfg)
        .args(["gift", "claim", GIFT_CODE])
        .env("SKZ_BASE_URL", "http://127.0.0.1:59917")
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(7));
    let body = json(&out.stderr);
    assert_eq!(body["error"]["action"], "check_existing");
    assert_eq!(body["error"]["retryable"], false);
    assert_eq!(
        body["error"]["remediation"]["verifyWith"],
        "skz gift preview <gift_code>"
    );
}
