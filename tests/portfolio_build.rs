//! 前端同款 Agent 组合构建命令；仅连接本地 mock，不触发付费计算。
use assert_cmd::Command;
use httpmock::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

fn config() -> TempDir {
    let dir = TempDir::new().unwrap();
    let credentials = if cfg!(target_os = "macos") {
        dir.path().join(".config/skz/credentials")
    } else {
        dir.path().join("skz/credentials")
    };
    std::fs::create_dir_all(credentials.parent().unwrap()).unwrap();
    std::fs::write(credentials, "sk_test").unwrap();
    dir
}

fn cli(dir: &TempDir, server: &MockServer) -> Command {
    let mut command = Command::cargo_bin("skz").unwrap();
    command
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env("SKZ_BASE_URL", server.base_url())
        .env_remove("SKZ_READ_ONLY");
    command
}

fn request() -> Value {
    json!({"market":"future","problemCodes":["PB1"],"strategyCodes":["S1"],
        "goal":"只交易 T/TL，月末调仓，风险归一后最小化半方差", "minTrials":30,
        "maxExplorations":50, "maxAttempts":45,"evaluationConcurrency":4,"timeoutSeconds":2700,
        "selectionConstraints":{"max_drawdown":0.2}, "llm":{"mode":"platform"}})
}

fn run(status: &str) -> Value {
    json!({"runId":"RUN1","userId":"U1","status":status,"userInput":"goal",
        "request":request(),"startedAt":"2026-10-20T12:00:00Z","finishedAt":null,
        "resultCode":null,"actualMethod":null,"result":{"extra":"keep"},
        "requirementMismatches":[],"futureField":"keep"})
}

#[test]
fn agent_start_sends_frontend_payload_and_preserves_run_identity() {
    let server = MockServer::start();
    let body = request();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/portfolios/builds")
            .header("authorization", "Bearer sk_test")
            .json_body(body.clone());
        then.status(202)
            .json_body(json!({"code":0,"data":run("running")}));
    });
    let dir = config();
    let output = cli(&dir, &server)
        .args(["portfolio", "build", "start"])
        .write_stdin(body.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    mock.assert_calls(1);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["runId"], "RUN1");
    assert_eq!(value["startedAt"], "2026-10-20T20:00:00+08:00");
    assert_eq!(value["futureField"], "keep");
    assert_eq!(value["result"]["extra"], "keep");
}

#[test]
fn agent_catalog_latest_notice_get_and_history_are_readable() {
    for (arguments, path, data) in [
        (
            vec!["catalog"],
            "/strategy/portfolios/builds/catalog",
            json!({"userId":"U1","markets":[],"problems":[],"strategies":[]}),
        ),
        (
            vec!["latest"],
            "/strategy/portfolios/builds/latest",
            Value::Null,
        ),
        (
            vec!["notice"],
            "/strategy/portfolios/builds/notice",
            run("needs_review"),
        ),
        (
            vec!["get", "RUN1"],
            "/strategy/portfolios/builds/RUN1",
            run("requirements_unmet"),
        ),
        (
            vec![
                "history", "--offset", "20", "--limit", "10", "--search", "T/TL", "--status",
                "failed",
            ],
            "/strategy/portfolios/builds",
            json!([run("failed")]),
        ),
    ] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            let when = when.method(GET).path(path);
            if arguments[0] == "history" {
                when.query_param("offset", "20")
                    .query_param("limit", "10")
                    .query_param("search", "T/TL")
                    .query_param("status", "failed");
            }
            then.json_body(json!({"code":0,"data":data}));
        });
        let dir = config();
        let mut command = cli(&dir, &server);
        command
            .args(["portfolio", "build"])
            .args(arguments)
            .env("SKZ_READ_ONLY", "1");
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        mock.assert();
    }
}

#[test]
fn agent_byok_configuration_is_forwarded_without_defaults_overwriting_it() {
    let server = MockServer::start();
    let mut body = request();
    body["llm"] = json!({"mode":"byok","configId":"11111111-1111-1111-1111-111111111111"});
    body["allowPartial"] = json!(false);
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/portfolios/builds")
            .json_body(body.clone());
        then.status(202)
            .json_body(json!({"code":0,"data":run("running")}));
    });
    let dir = config();
    let output = cli(&dir, &server)
        .args(["portfolio", "build", "start"])
        .write_stdin(body.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    mock.assert_calls(1);
}

#[test]
fn agent_start_rejects_ignored_legacy_fields_and_invalid_input_before_network() {
    for body in [
        json!({"market":"future","goal":"goal","requiredMethod":"skfolio"}),
        json!({"market":"future","goal":"goal","rebalanceRule":"每月最后一天"}),
        json!({"market":"future","goal":"goal","lookbackDays":120}),
        json!({"market":"future","goal":"goal","llm":{"mode":"platform","configId":"id"}}),
        json!({"market":"future","goal":" "}),
    ] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.json_body(json!({}));
        });
        let dir = config();
        let output = cli(&dir, &server)
            .args(["portfolio", "build", "start"])
            .write_stdin(body.to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{body}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        mock.assert_calls(0);
    }
}

#[test]
fn agent_start_and_read_receipt_obey_read_only_guard() {
    for arguments in [vec!["start"], vec!["read", "RUN1"]] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.json_body(json!({}));
        });
        let dir = config();
        let output = cli(&dir, &server)
            .args(["portfolio", "build"])
            .args(arguments)
            .env("SKZ_READ_ONLY", "1")
            .write_stdin(request().to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(8),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        mock.assert_calls(0);
    }
}

#[test]
fn agent_start_never_retries_http_failure_and_directs_unknown_write_to_latest() {
    for status in [409, 503] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/strategy/portfolios/builds");
            then.status(status)
                .json_body(json!({"code":status * 100,"msg":"failure","data":null}));
        });
        let dir = config();
        let output = cli(&dir, &server)
            .args(["portfolio", "build", "start"])
            .write_stdin(request().to_string())
            .output()
            .unwrap();
        assert!(!output.status.success());
        mock.assert_calls(1);
    }
}

#[test]
fn agent_wallet_check_distinguishes_platform_and_byok_prices() {
    for (operation, required) in [("portfolio-build", 2000), ("portfolio-build-byok", 1000)] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/payment/wallet/summary");
            then.json_body(json!({"userId":1,"cash":{"accountId":1,"currency":"CNY","balanceCent":5000,
                "frozenCent":0,"overdraftLimitCent":0,"availableCent":5000},"purses":[],"totalAvailableCent":5000}));
        });
        let dir = config();
        let output = cli(&dir, &server)
            .args(["wallet", "check", operation, "--qty", "1"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        mock.assert_calls(1);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["operation"], operation);
        assert_eq!(value["requiredCent"], required);
        assert_eq!(value["pricingSource"], "cli");
    }
}

#[test]
fn agent_transport_failure_is_unknown_and_does_not_suggest_resubmission() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let server = MockServer::start();
    let dir = config();
    let output = cli(&dir, &server)
        .args(["portfolio", "build", "start"])
        .env("SKZ_BASE_URL", format!("http://{address}"))
        .write_stdin(request().to_string())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["action"], "check_existing");
    assert_eq!(
        error["error"]["remediation"]["verifyWith"],
        "skz portfolio build latest"
    );
}

#[test]
fn agent_mark_read_and_inline_byok_use_the_correct_endpoints() {
    let server = MockServer::start();
    let mut body = request();
    body["llm"] = json!({"mode":"byok","apiKey":"dummy-api-key-no-real-access","baseUrl":"https://example.invalid","model":"test-model","saveAs":"my-model"});
    let start = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/portfolios/builds")
            .json_body(body.clone());
        then.status(202)
            .json_body(json!({"code":0,"data":run("running")}));
    });
    let read = server.mock(|when, then| {
        when.method(POST)
            .path("/strategy/portfolios/builds/RUN1/read");
        then.json_body(json!({"code":0,"data":true}));
    });
    let dir = config();
    let output = cli(&dir, &server)
        .args(["portfolio", "build", "start"])
        .write_stdin(body.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("dummy-api-key-no-real-access"));
    let output = cli(&dir, &server)
        .args(["portfolio", "build", "read", "RUN1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!(true)
    );
    start.assert_calls(1);
    read.assert_calls(1);
}
