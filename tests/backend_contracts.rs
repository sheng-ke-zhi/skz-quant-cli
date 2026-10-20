//! 来自 user-research-backend typed DTO/生成 OpenAPI 的响应快照，检查解析和已知字段保留。
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use skz::models::{experiment, factor, gift, live, market, mining, portfolio, problem, research};

fn assert_fields(input: &Value, output: &Value, path: &str) {
    match input {
        Value::Object(fields) => {
            for (key, value) in fields {
                // 后端对部分 Option 字段使用 skip_serializing_if；null 与省略均合法。
                let Some(output) = output.get(key) else {
                    assert!(value.is_null(), "{path}.{key} 被 CLI 丢弃");
                    continue;
                };
                assert_fields(value, output, &format!("{path}.{key}"));
            }
        }
        Value::Array(items) => {
            let output = output
                .as_array()
                .unwrap_or_else(|| panic!("{path} 不再是数组"));
            assert_eq!(items.len(), output.len(), "{path} 长度");
            for (index, (item, output)) in items.iter().zip(output).enumerate() {
                assert_fields(item, output, &format!("{path}[{index}]"));
            }
        }
        Value::Null => assert!(output.is_null(), "{path} 的 null 被改写"),
        _ => assert_eq!(input, output, "{path} 的值被改写"),
    }
}

fn check<T: DeserializeOwned + Serialize>(name: &str) {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/backend-contracts/research.json")).unwrap();
    let mut failures = Vec::new();
    for (variant, input) in fixtures[name].as_object().unwrap() {
        match serde_json::from_value::<T>(input.clone()) {
            Ok(value) => {
                let output = serde_json::to_value(value).unwrap();
                assert_fields(input, &output, &format!("{name}/{variant}"));
            }
            Err(error) => failures.push(format!("{name}/{variant}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn experimentlist_matches_backend() {
    check::<experiment::ExperimentList>("ExperimentList");
}

#[test]
fn experimentdetail_matches_backend() {
    check::<experiment::ExperimentDetail>("ExperimentDetail");
}

#[test]
fn experimentreviewmatrix_matches_backend() {
    check::<experiment::ReviewMatrix>("ExperimentReviewMatrix");
}

#[test]
fn experimentstrategylist_matches_backend() {
    check::<experiment::ExperimentStrategies>("ExperimentStrategyList");
}

#[test]
fn performancereport_matches_backend() {
    check::<experiment::PerformanceReport>("PerformanceReport");
}

#[test]
fn promotionview_matches_backend() {
    check::<experiment::Promotion>("PromotionView");
}

#[test]
fn factorroutelist_matches_backend() {
    check::<factor::FactorRoutesResponse>("FactorRouteList");
}

#[test]
fn routedeleted_matches_backend() {
    check::<factor::RouteDeleted>("RouteDeleted");
}

#[test]
fn factorlist_matches_backend() {
    check::<factor::FactorList>("FactorList");
}

#[test]
fn factorsummary_matches_backend() {
    check::<factor::FactorSummary>("FactorSummary");
}

#[test]
fn factordetail_matches_backend() {
    check::<factor::FactorDetail>("FactorDetail");
}

#[test]
fn factorsoftdeleted_matches_backend() {
    check::<factor::FactorSoftDeleted>("FactorSoftDeleted");
}

#[test]
fn factorssoftdeleted_matches_backend() {
    check::<factor::FactorsSoftDeleted>("FactorsSoftDeleted");
}

#[test]
fn giftlist_matches_backend() {
    check::<gift::GiftList>("GiftList");
}

#[test]
fn giftview_matches_backend() {
    check::<gift::GiftView>("GiftView");
}

#[test]
fn giftclaimed_matches_backend() {
    check::<gift::GiftClaimed>("GiftClaimed");
}

#[test]
fn giftpreview_matches_backend() {
    check::<gift::GiftPreview>("GiftPreview");
}

#[test]
fn receivedgiftlist_matches_backend() {
    check::<gift::ReceivedGiftList>("ReceivedGiftList");
}

#[test]
fn giftrevoked_matches_backend() {
    check::<gift::GiftRevoked>("GiftRevoked");
}

#[test]
fn futurecontractsresolved_matches_backend() {
    check::<market::FutureContractsResolved>("FutureContractsResolved");
}

#[test]
fn miningrunlist_matches_backend() {
    check::<mining::MiningRunList>("MiningRunList");
}

#[test]
fn miningfactorlist_matches_backend() {
    check::<mining::MiningFactorList>("MiningFactorList");
}

#[test]
fn miningoverview_matches_backend() {
    check::<mining::MiningOverview>("MiningOverview");
}

#[test]
fn portfoliolist_matches_backend() {
    check::<portfolio::PortfolioList>("PortfolioList");
}

#[test]
fn createportfolioack_matches_backend() {
    check::<portfolio::CreatePortfolioAck>("CreatePortfolioAck");
}

#[test]
fn portfoliodetail_matches_backend() {
    check::<portfolio::PortfolioDetail>("PortfolioDetail");
}

#[test]
fn portfolioconfigcontent_matches_backend() {
    check::<portfolio::PortfolioConfigContent>("PortfolioConfigContent");
}

#[test]
fn portfoliolatestpositions_matches_backend() {
    check::<portfolio::PortfolioLatestPositions>("PortfolioLatestPositions");
}

#[test]
fn refreshstate_matches_backend() {
    check::<portfolio::PortfolioRefreshStatus>("RefreshState");
}

#[test]
fn problemlist_matches_backend() {
    check::<problem::ProblemList>("ProblemList");
}

#[test]
fn problemview_matches_backend() {
    check::<problem::ProblemView>("ProblemView");
}

#[test]
fn problemmeta_matches_backend() {
    check::<problem::ProblemMeta>("ProblemMeta");
}

#[test]
fn strategylist_matches_backend() {
    check::<live::StrategyList>("StrategyList");
}

#[test]
fn latestweights_matches_backend() {
    check::<live::LatestWeights>("LatestWeights");
}

#[test]
fn strategydetail_matches_backend() {
    check::<live::StrategyDetail>("StrategyDetail");
}

#[test]
fn livestrategyanalysis_matches_backend() {
    check::<live::LiveAnalysis>("LiveStrategyAnalysis");
}

#[test]
fn livetradesdata_matches_backend() {
    check::<live::TradesResponse>("LiveTradesData");
}

#[test]
fn strategynav_matches_backend() {
    check::<live::StrategyNav>("StrategyNav");
}

#[test]
fn strategyperiodic_matches_backend() {
    check::<live::StrategyPeriodic>("StrategyPeriodic");
}

#[test]
fn strategyrecenteval_matches_backend() {
    check::<live::StrategyRecentEval>("StrategyRecentEval");
}

#[test]
fn strategysegments_matches_backend() {
    check::<live::StrategySegments>("StrategySegments");
}

#[test]
fn strategypositions_matches_backend() {
    check::<live::StrategyPositions>("StrategyPositions");
}

#[test]
fn strategysymboldailyreturns_matches_backend() {
    check::<live::StrategySymbolDailyReturns>("StrategySymbolDailyReturns");
}

#[test]
fn memoupdated_matches_backend() {
    check::<live::MemoUpdated>("MemoUpdated");
}

#[test]
fn tagupdated_matches_backend() {
    check::<live::TagUpdated>("TagUpdated");
}

#[test]
fn strategiesimported_matches_backend() {
    check::<live::StrategiesImported>("StrategiesImported");
}

#[test]
fn whoami_matches_backend() {
    check::<research::WhoAmI>("WhoAmI");
}

#[test]
fn gift_job_states_are_supported() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/backend-contracts/research.json")).unwrap();
    for status in ["processing", "failed"] {
        let mut value = fixtures["GiftPreview"]["populated"].clone();
        value["claim_status"] = Value::String(status.into());
        let preview: gift::GiftPreview = serde_json::from_value(value).unwrap();
        assert_eq!(
            serde_json::to_value(preview).unwrap()["claim_status"],
            status
        );
    }
}

#[test]
fn calendar_nullable_pretrade_date_and_wallet_contract_match_backend() {
    let wallet = serde_json::json!({"userId":1,"cash":{"accountId":1,"currency":"CNY","balanceCent":0,"frozenCent":0,"overdraftLimitCent":0,"availableCent":0},"purses":[],"totalAvailableCent":500});
    let parsed: skz::models::wallet::WalletSummary =
        serde_json::from_value(wallet.clone()).unwrap();
    assert_fields(
        &wallet,
        &serde_json::to_value(parsed).unwrap(),
        "WalletSummary",
    );
    let calendar = serde_json::json!({"exchange":"SSE","calDate":"1990-12-19","isOpen":true,"pretradeDate":null});
    let parsed: market::CalendarDay = serde_json::from_value(calendar.clone()).unwrap();
    assert_fields(
        &calendar,
        &serde_json::to_value(parsed).unwrap(),
        "CalendarDay",
    );
}

#[test]
fn queued_task_and_exploration_identity_are_not_dropped() {
    let ack = serde_json::json!({"fcRunId":"QUEUE1","taskId":"QUEUE1","status":"queued","routeCode":"R1"});
    let parsed: skz::models::strategy::TriggerAck = serde_json::from_value(ack.clone()).unwrap();
    assert_fields(&ack, &serde_json::to_value(parsed).unwrap(), "TriggerAck");
    let run = serde_json::json!({"fcRunId":"RUN1","problemCode":"PB1","routeCode":"R1","status":"running","statusText":"running","done":false,"ok":false,"errorCode":null,"errorMessage":null,"resultPath":null,"createdAt":"contract_value","finishedAt":null});
    let parsed: research::RunSummary = serde_json::from_value(run.clone()).unwrap();
    assert_fields(&run, &serde_json::to_value(parsed).unwrap(), "RunSummary");
}

#[test]
fn audit_covers_every_registered_backend_route() {
    let routes: Vec<String> =
        serde_json::from_str(include_str!("plugins/open_api_routes.json")).unwrap();
    let audit: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/backend-contracts/routes.json")).unwrap();
    let actual: std::collections::BTreeSet<_> = audit
        .iter()
        .map(|entry| entry["route"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(routes.len(), audit.len());
    assert_eq!(
        routes
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        actual
    );
    assert!(
        audit
            .iter()
            .all(|entry| !entry["backend"].as_str().unwrap().is_empty())
    );
}

#[test]
fn event_times_and_position_data_axes_keep_distinct_meanings() {
    let snapshot: portfolio::PortfolioPositionSnapshot =
        serde_json::from_value(serde_json::json!({
        "dt":"2026-10-20T12:00:00Z","data_at":"2026-10-20T12:00:00Z",
        "updated_at":"2026-10-20T13:00:00Z","weights":[]}))
        .unwrap();
    let output = serde_json::to_value(snapshot).unwrap();
    assert_eq!(output["dt"], "2026-10-20T12:00:00Z");
    assert_eq!(output["data_at"], "2026-10-20T12:00:00Z");
    assert_eq!(output["updated_at"], "2026-10-20T21:00:00+08:00");
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/backend-contracts/research.json")).unwrap();
    for input in [
        serde_json::json!("2026-10-20T12:00:00Z"),
        serde_json::json!({"source":"external","sequence":7}),
    ] {
        let mut payload = fixtures["ExperimentList"]["populated"].clone();
        payload["items"][0]["run_at"] = input.clone();
        let parsed: experiment::ExperimentList = serde_json::from_value(payload).unwrap();
        let output = serde_json::to_value(parsed).unwrap();
        assert_eq!(
            output["items"][0]["run_at"],
            if input.is_string() {
                serde_json::json!("2026-10-20T20:00:00+08:00")
            } else {
                input
            }
        );
    }
}

#[test]
fn older_experiment_summaries_keep_missing_optional_fields_compatible() {
    let item = serde_json::json!({"id":"E1","strategy_count":0,"symbols_count":0});
    let parsed: experiment::ExperimentListItem = serde_json::from_value(item).unwrap();
    let output = serde_json::to_value(parsed).unwrap();
    assert!(output["route"].is_null());
    assert!(output["run_at"].is_null());
}
