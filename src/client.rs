//! ureq（blocking + rustls）客户端：预定义 endpoint、query 编码、响应反序列化。
//! 构造时由调用方传入 base_url 与 token，自身不读文件或 env。

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::{Config, USER_AGENT};
use crate::error::{Error, ResearchHint};
use crate::models::common::Page;
use crate::models::experiment::{
    ExperimentDetail, ExperimentList, ExperimentStrategies, Promotion, ReviewMatrix, RunDeleted,
    StrategyDeleted,
};
use crate::models::factor::{
    FactorDetail, FactorList, FactorRoutesResponse, FactorSoftDeleted, FactorSummary, RouteDeleted,
};
use crate::models::gift::{GiftClaimed, GiftList, GiftPreview, GiftRevoked, GiftView};
use crate::models::live::{
    LatestWeights, MemoUpdated, StatusUpdated, StrategiesImported, StrategyDetail, StrategyList,
    StrategyNav, StrategyPeriodic, StrategyPositions, StrategyRecentEval, StrategySegments,
    TagUpdated, TradesResponse,
};
use crate::models::market::{CalendarDay, Market, Symbol};
use crate::models::mining::{MiningFactorList, MiningOverview, MiningRunDeleted, MiningRunList};
use crate::models::portfolio::{CreatePortfolioAck, PortfolioDetail, PortfolioList};
use crate::models::problem::{ProblemDeleted, ProblemList, ProblemMeta, ProblemView};
use crate::models::research::{RunProgress, RunSummary, WhoAmI};
use crate::models::strategy::{
    AdoptedRoute, ProblemCreated, ProblemData, ProblemEnvelope, RouteCreated, TriggerAck,
};
use crate::token::Token;

/// 空 query（研究面无参 GET 复用）。
const NO_QUERY: &[(&str, String)] = &[];

/// ureq 3.x 的响应类型是 `http::Response<Body>`；起个别名省得到处写全称。
type Resp = ureq::http::Response<ureq::Body>;

pub struct Client {
    base_url: String,
    token: Token,
    agent: ureq::Agent,
    /// 只读闸。由 `Config` 从环境注入——client 自身仍然不读 env，
    /// 好让 library 的其他入口（未来的 MCP server）能自己决定这个值。
    read_only: bool,
}

impl Client {
    pub fn new(cfg: &Config, token: Token) -> Self {
        // http_status_as_error(false)：非 2xx 也走 Ok(resp)，好让我们自己读
        // body/header 分类错误（errorCode、Retry-After）——3.x 默认会把 4xx/5xx
        // 直接变成 Err(StatusCode(u16))，只剩状态码，body 和 header 都没了。
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(cfg.timeout_secs)))
            .max_redirects(0)
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .build();
        let agent: ureq::Agent = config.into();
        Client {
            base_url: cfg.base_url.clone(),
            token,
            agent,
            read_only: cfg.read_only,
        }
    }

    /// 只读闸。**这是不变量那一道**：所有写都经过 `post_json` / `send_research_json`，
    /// 两个入口都先调它，所以新加的写默认就被拦住——想放行必须自己主动去写
    /// `post_json_readlike`，那是个显式动作，不会手滑漏掉。
    ///
    /// 也 pub 出去给调用方做**早退优化**：`portfolio create` 之类的写前面挂着免费预检读，
    /// 只在传输层拦的话那些读会白跑一趟才报错。但要分清主次——早退那道只是省几次请求，
    /// 哪怕漏了某条命令，最坏结果也只是多跑几次免费读，绝不会漏出一次写。
    pub fn ensure_writable(&self) -> Result<(), Error> {
        if self.read_only {
            return Err(Error::ReadOnly);
        }
        Ok(())
    }

    fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.base_url, path);
        let auth = format!("Bearer {}", self.token.expose());
        let mut req = self.agent.get(&url).header("Authorization", &auth);
        for pair in query {
            req = req.query(pair.0, pair.1.as_str());
        }
        match req.call() {
            Ok(mut resp) if resp.status().is_success() => {
                let body = resp
                    .body_mut()
                    .read_to_string()
                    .map_err(|e| Error::Network(format!("读取响应失败: {e}")))?;
                serde_json::from_str::<T>(&body)
                    .map_err(|e| Error::Internal(format!("响应 JSON 解析失败: {e}")))
            }
            Ok(resp) => Err(parse_api_error(resp)),
            Err(e) => Err(Error::Network(e.to_string())),
        }
    }

    /// POST 一份 JSON body 并反序列化响应。手动序列化 + `send_string`，
    /// 避免依赖 ureq 的 `json` feature（本项目为控体积未启用）。
    /// 是否重试由**调用方**决定（写不重试、poll 可重试），本方法不涉入。
    ///
    /// **默认按写处理**（过只读闸）。POST 但语义是读的走 `post_json_readlike`。
    fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.ensure_writable()?;
        self.post_json_inner(path, body, None)
    }

    /// POST 成功体仍按平台模型解析，但非 2xx 可选择识别下游研究信封。
    /// `/strategy/problems` 经 C# 原样透传 Rust `{code,msg,data}` 错误，不能把它降级成原始 JSON 文本。
    fn post_json_with_error_mode<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        research_error_is_read: Option<bool>,
    ) -> Result<T, Error> {
        self.ensure_writable()?;
        self.post_json_inner(path, body, research_error_is_read)
    }

    /// **POST 但语义是读**：只读模式下照常放行。目前只有两个 `poll`——后端用 POST 是因为
    /// 要带一批 run id，不是因为它改什么。
    ///
    /// 存在的意义是把"这是个例外"变成显式的、要人手动选的东西：`post_json` 默认拦，
    /// 于是新加的写不可能忘记过闸；而想开例外就得亲手写到这里来，逃不掉那一下思考。
    ///
    /// 注意"放行"只是就本 CLI 而言。这两个端点在后端会把超过 24h 的 run 翻成 `timeout`
    /// 并发起退款，所以只读模式承诺的是「本 CLI 不发起扣费、不改资产状态」，
    /// **不是「上游零副作用」**——真封掉这类端点，封掉的恰恰是给用户退钱的路径。
    fn post_json_readlike<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.post_json_inner(path, body, None)
    }

    fn post_json_inner<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        research_error_is_read: Option<bool>,
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.base_url, path);
        let auth = format!("Bearer {}", self.token.expose());
        let payload = serde_json::to_string(body)
            .map_err(|e| Error::Internal(format!("请求体序列化失败: {e}")))?;
        let req = self
            .agent
            .post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json");
        match req.send(&payload) {
            Ok(mut resp) if resp.status().is_success() => {
                let body = resp
                    .body_mut()
                    .read_to_string()
                    .map_err(|e| Error::Network(format!("读取响应失败: {e}")))?;
                serde_json::from_str::<T>(&body)
                    .map_err(|e| Error::Internal(format!("响应 JSON 解析失败: {e}")))
            }
            Ok(resp) => Err(match research_error_is_read {
                Some(is_read) => research_err(resp, is_read),
                None => parse_api_error(resp),
            }),
            Err(e) => Err(Error::Network(e.to_string())),
        }
    }

    // ── 研究面 helper（/research/*，统一信封 {code,msg,data}）────────────
    // 成功：code==0 → data。业务错多数骑非 2xx（HTTP == code 前三位），少数读接口
    // 会 HTTP 200 + code!=0（如 experiments/{id} 的 42201「数据尚未就绪」）——两条路径
    // 都按数值 code + is_read 分类，禁止再把 2xx 非 0 当成协议异常。
    // 非 research 信封（网关 {status,title,errorCode}）回落 parse_api_error。

    /// research 面 GET：拆信封取 data。`is_read=true`（读，42201 归 retry_later）。
    fn get_research_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.base_url, path);
        let auth = format!("Bearer {}", self.token.expose());
        let mut req = self.agent.get(&url).header("Authorization", &auth);
        for pair in query {
            req = req.query(pair.0, pair.1.as_str());
        }
        match req.call() {
            Ok(resp) if resp.status().is_success() => unwrap_research_2xx(resp, true),
            Ok(resp) => Err(research_err(resp, true)),
            Err(e) => Err(Error::Network(e.to_string())),
        }
    }

    /// research 面写（POST/PATCH/DELETE）：`method` 指定动词，`body` 可空（DELETE 无体）。
    /// `is_read=false`（写，42201 归 fix_params）；重试与否由调用方决定（写不套 with_retry）。
    ///
    /// **默认按写处理**（过只读闸）。DELETE 但语义是读的走 `send_research_json_readlike`。
    fn send_research_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, Error> {
        self.ensure_writable()?;
        self.send_research_inner(method, path, query, body)
    }

    /// 过只读闸的例外：动词是写但后端零修改的端点。目前只有 `factor-routes delete --dry-run`
    /// ——它只预告「将删几次挖掘执行、将留几个孤儿因子」，不碰任何数据。放行的理由是只读模式
    /// 的动机就是「让 agent 看清代价再交人决定」，把这个预告一并封掉恰好封掉了它自己想要的东西。
    /// **代价要说清**：只读模式下确实会有 DELETE 请求发出去，这条例外必须由调用方在 `dry_run`
    /// 为真时显式选择，不能让别的删除路径顺手复用。
    fn send_research_json_readlike<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, Error> {
        self.send_research_inner(method, path, query, body)
    }

    /// ureq 3.x 没有 2.x 那种 `agent.request(method, url)` 泛型入口了：
    /// `post`/`patch` 给 `WithBody`（有 `.send()`），`delete` 给 `WithoutBody`
    /// （只有 `.call()`）。`factor_delete` 偏偏是 DELETE 带 body，所以 DELETE
    /// 分支要用 `.force_send_body()` 这个逃生舱把 `WithoutBody` 转回 `WithBody`
    /// 才能 `.send()`。
    fn send_research_inner<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.base_url, path);
        let auth = format!("Bearer {}", self.token.expose());
        let payload = body
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| Error::Internal(format!("请求体序列化失败: {e}")))?;
        let sent = match method {
            "POST" | "PATCH" => {
                let mut req = if method == "POST" {
                    self.agent.post(&url)
                } else {
                    self.agent.patch(&url)
                }
                .header("Authorization", &auth)
                .header("Content-Type", "application/json");
                for pair in query {
                    req = req.query(pair.0, pair.1.as_str());
                }
                match &payload {
                    Some(p) => req.send(p),
                    None => req.send_empty(),
                }
            }
            "DELETE" => {
                let mut req = self.agent.delete(&url).header("Authorization", &auth);
                for pair in query {
                    req = req.query(pair.0, pair.1.as_str());
                }
                match &payload {
                    Some(p) => req
                        .header("Content-Type", "application/json")
                        .force_send_body()
                        .send(p),
                    None => req.call(),
                }
            }
            _ => unreachable!("send_research_inner 只用于 POST/PATCH/DELETE"),
        };
        match sent {
            Ok(resp) if resp.status().is_success() => unwrap_research_2xx(resp, false),
            Ok(resp) => Err(research_err(resp, false)),
            Err(e) => Err(Error::Network(e.to_string())),
        }
    }

    pub fn markets(&self) -> Result<Vec<Market>, Error> {
        let q: &[(&str, String)] = &[];
        self.get_json("/market/markets", q)
    }

    pub fn symbols(
        &self,
        market: Option<&str>,
        keyword: Option<&str>,
        page: u32,
        size: u32,
    ) -> Result<Page<Symbol>, Error> {
        let mut q: Vec<(&str, String)> =
            vec![("page", page.to_string()), ("size", size.to_string())];
        if let Some(m) = market {
            q.push(("market", m.to_string()));
        }
        if let Some(k) = keyword {
            q.push(("keyword", k.to_string()));
        }
        self.get_json("/market/symbols", &q)
    }

    pub fn calendar(
        &self,
        exchange: &str,
        start: Option<&str>,
        end: Option<&str>,
        only_open: bool,
    ) -> Result<Vec<CalendarDay>, Error> {
        let mut q: Vec<(&str, String)> = vec![("exchange", exchange.to_string())];
        if let Some(s) = start {
            q.push(("start", s.to_string()));
        }
        if let Some(e) = end {
            q.push(("end", e.to_string()));
        }
        if only_open {
            q.push(("onlyOpen", "true".to_string()));
        }
        self.get_json("/market/trading-calendar", &q)
    }

    // ── 策略业务：读 ────────────────────────────────────────────────
    // 以下均为读（含两个语义为读的 POST poll）；由 bin 侧包 `with_retry`。

    /// `GET /strategy/routes/adopted` 已采用路线（供 explore 挑 routeCode）。无参数。
    pub fn adopted_routes(&self) -> Result<Vec<AdoptedRoute>, Error> {
        let q: &[(&str, String)] = &[];
        self.get_json("/strategy/routes/adopted", q)
    }

    /// `GET /strategy/miner/runs` 策略挖掘运行列表（与 `miner_runs` 的 /research 视图区分）。
    pub fn strategy_miner_runs(
        &self,
        status: Option<&str>,
        page: u32,
        size: u32,
    ) -> Result<Page<RunSummary>, Error> {
        self.paged_runs("/strategy/miner/runs", status, page, size)
    }

    /// `POST /strategy/miner/poll` 批量查进度（POST 但语义为读、幂等，可重试）。
    pub fn strategy_miner_poll(&self, run_ids: &[String]) -> Result<Vec<RunProgress>, Error> {
        self.post_json_readlike(
            "/strategy/miner/poll",
            &serde_json::json!({ "runIds": run_ids }),
        )
    }

    /// `GET /strategy/explore/{fcRunId}` 单个探索任务实时进度。
    pub fn explore_get(&self, fc_run_id: &str) -> Result<RunProgress, Error> {
        let path = format!("/strategy/explore/{fc_run_id}");
        let q: &[(&str, String)] = &[];
        self.get_json(&path, q)
    }

    /// `GET /strategy/explore/runs` 探索运行列表。
    pub fn explore_runs(
        &self,
        status: Option<&str>,
        page: u32,
        size: u32,
    ) -> Result<Page<RunSummary>, Error> {
        self.paged_runs("/strategy/explore/runs", status, page, size)
    }

    /// `POST /strategy/explore/poll` 批量查进度（POST 但语义为读、幂等，可重试）。
    pub fn explore_poll(&self, run_ids: &[String]) -> Result<Vec<RunProgress>, Error> {
        self.post_json_readlike(
            "/strategy/explore/poll",
            &serde_json::json!({ "runIds": run_ids }),
        )
    }

    // ── 策略业务：写 ────────────────────────────────────────────────
    // 以下均为写/触发；由 bin 侧**不**包 `with_retry`（无幂等保证，触发即扣费）。

    /// `POST /strategy/routes` 创建研究方向，body 由 stdin JSON 透传。返回 `{ routeCode }`。
    pub fn create_route(&self, body: &serde_json::Value) -> Result<RouteCreated, Error> {
        self.post_json("/strategy/routes", body)
    }

    /// `POST /strategy/problems` 创建研究问题，body 由 stdin JSON 透传。
    /// 外部后端返回 `{ code, msg, data }` 信封：`code==0 && data.problemCode` 才算成功，
    /// 否则按协议异常上报（internal / exit 6），把后端的 `msg` 带出去。
    pub fn create_problem(&self, body: &serde_json::Value) -> Result<ProblemCreated, Error> {
        let env: ProblemEnvelope =
            self.post_json_with_error_mode("/strategy/problems", body, Some(false))?;
        match env {
            // 空串 problemCode 视同缺失：否则会拿一个 `{"problemCode":""}` 当成功回出去，
            // 下游 explore 又用不了。与 null/缺失一样归到非成功包（exit 6）。
            ProblemEnvelope {
                code: 0,
                data: Some(ProblemData { code: Some(pc) }),
                ..
            } if !pc.trim().is_empty() => Ok(ProblemCreated { problem_code: pc }),
            other => Err(Error::Internal(format!(
                "研究后端返回非成功包 (code={}): {}",
                other.code,
                if other.msg.is_empty() {
                    "无 msg"
                } else {
                    &other.msg
                }
            ))),
        }
    }

    /// `POST /strategy/miner/runs` 触发因子挖掘（要 routeCode）。返回 `{ fcRunId, status, routeCode }`。
    pub fn trigger_mine(&self, route_code: &str) -> Result<TriggerAck, Error> {
        self.post_json(
            "/strategy/miner/runs",
            &serde_json::json!({ "routeCode": route_code }),
        )
    }

    /// `POST /strategy/explore` 触发策略探索（要 problemCode + routeCode）。返回 `{ fcRunId, status }`。
    pub fn trigger_explore(
        &self,
        problem_code: &str,
        route_code: &str,
        conversation_id: Option<&str>,
        tool_call_id: Option<&str>,
    ) -> Result<TriggerAck, Error> {
        let mut body = serde_json::Map::new();
        body.insert("problemCode".into(), problem_code.into());
        body.insert("routeCode".into(), route_code.into());
        if let Some(c) = conversation_id {
            body.insert("conversationId".into(), c.into());
        }
        if let Some(t) = tool_call_id {
            body.insert("toolCallId".into(), t.into());
        }
        self.post_json("/strategy/explore", &serde_json::Value::Object(body))
    }

    // ── 研究面：读（/research/*）；由 bin 侧包 with_retry ────────────

    /// `GET /research/whoami` 开放平台身份自检（返回 user_id）。
    pub fn whoami(&self) -> Result<WhoAmI, Error> {
        let q: &[(&str, String)] = &[];
        self.get_research_json("/research/whoami", q)
    }

    // 因子库（读）
    /// `GET /research/factors/summary` 因子库概览（KPI + 引擎/路线/标签分布）。
    pub fn factor_summary(&self) -> Result<FactorSummary, Error> {
        let q: &[(&str, String)] = &[];
        self.get_research_json("/research/factors/summary", q)
    }

    /// `GET /research/factor-routes` 研究方向全集（含研究设计元信息）。
    pub fn factor_routes(&self) -> Result<FactorRoutesResponse, Error> {
        let q: &[(&str, String)] = &[];
        self.get_research_json("/research/factor-routes", q)
    }

    /// `GET /research/factors` 因子列表（分页/筛选/排序）。query 由 bin 侧组装。
    pub fn factors(&self, query: &[(&str, String)]) -> Result<FactorList, Error> {
        self.get_research_json("/research/factors", query)
    }

    /// `GET /research/factors/{factor_name}` 单因子详情（tags + 跨问题 evaluations）。
    pub fn factor_get(&self, factor_name: &str) -> Result<FactorDetail, Error> {
        let path = format!("/research/factors/{factor_name}");
        let q: &[(&str, String)] = &[];
        self.get_research_json(&path, q)
    }

    // 挖掘成果（读）
    /// `GET /research/mining/runs` 挖掘 run 列表（成果柜，可按 route_code 过滤）。
    pub fn mining_runs(&self, route: Option<&str>) -> Result<MiningRunList, Error> {
        let mut q: Vec<(&str, String)> = vec![];
        if let Some(r) = route {
            q.push(("route_code", r.to_string()));
        }
        self.get_research_json("/research/mining/runs", &q)
    }

    /// `DELETE /research/mining/runs/{run_id}` 物理删除单次已落盘挖掘结果。
    /// `force` 只越过“目录最近仍有写入”的软护栏。
    pub fn mining_delete_run(&self, run_id: &str, force: bool) -> Result<MiningRunDeleted, Error> {
        let path = format!("/research/mining/runs/{run_id}");
        let query: Vec<(&str, String)> = if force {
            vec![("force", "true".to_string())]
        } else {
            Vec::new()
        };
        self.send_research_json::<(), _>("DELETE", &path, &query, None)
            .map_err(|e| e.with_research_hint(ResearchHint::MiningRunDelete))
    }

    /// `GET /research/mining/{run_id}/overview` 单次 run 漏斗/KPI 概览。
    pub fn mining_overview(&self, run_id: &str) -> Result<MiningOverview, Error> {
        let path = format!("/research/mining/{run_id}/overview");
        let q: &[(&str, String)] = &[];
        self.get_research_json(&path, q)
    }

    /// `GET /research/mining/{run_id}/factors` 这次挖出的因子清单（分页/筛选/排序）。
    pub fn mining_factors(
        &self,
        run_id: &str,
        query: &[(&str, String)],
    ) -> Result<MiningFactorList, Error> {
        let path = format!("/research/mining/{run_id}/factors");
        self.get_research_json(&path, query)
    }

    // 因子库（写，不重试）
    /// `DELETE /research/factors/{factor_name}` 逻辑审核·软删（body 带 reason，可空串）。
    pub fn factor_delete(
        &self,
        factor_name: &str,
        reason: Option<&str>,
    ) -> Result<FactorSoftDeleted, Error> {
        let path = format!("/research/factors/{factor_name}");
        let body = serde_json::json!({ "reason": reason.unwrap_or("") });
        self.send_research_json("DELETE", &path, NO_QUERY, Some(&body))
    }

    /// `DELETE /research/factor-routes/{code}` 删研究路线（**物理删**）+ 级联删名下挖掘执行。
    ///
    /// `force` 越过两条软护栏（名下仍有因子 40907 / 执行目录最近有写入 40906）；后端在这个端点
    /// 上没有硬拒绝那一级——它不触发 miner，没有可查的权威运行态。
    ///
    /// `dry_run` 走 readlike 通道：后端零修改，只回「将删几次执行、将留几个孤儿因子」，
    /// 所以只读模式下也放行（见 `send_research_json_readlike`）。**它不绕过护栏**——护栏没过
    /// 时预演一样报 409，这是好事：agent 能在花任何代价之前就知道自己需要 `--force`。
    pub fn factor_route_delete(
        &self,
        code: &str,
        force: bool,
        dry_run: bool,
    ) -> Result<RouteDeleted, Error> {
        let path = format!("/research/factor-routes/{code}");
        // 只在为真时发这个键：省得给后端的 `#[serde(default)]` 送一个它本来就默认的值。
        let mut query: Vec<(&str, String)> = Vec::new();
        if force {
            query.push(("force", "true".to_string()));
        }
        if dry_run {
            query.push(("dry_run", "true".to_string()));
            return self
                .send_research_json_readlike::<(), _>("DELETE", &path, &query, None)
                .map_err(|e| e.with_research_hint(ResearchHint::DeleteGuardrail));
        }
        self.send_research_json::<(), _>("DELETE", &path, &query, None)
            .map_err(|e| e.with_research_hint(ResearchHint::DeleteGuardrail))
    }

    // 策略实盘富读（读）
    /// `GET /research/strategies` 实盘策略库列表（分页/筛选/排序，可内嵌 metrics）。
    pub fn strategy_list(&self, query: &[(&str, String)]) -> Result<StrategyList, Error> {
        self.get_research_json("/research/strategies", query)
    }

    /// `GET /research/strategies/{code}` 单策略详情。
    pub fn strategy_get(&self, code: &str) -> Result<StrategyDetail, Error> {
        self.get_research_json(&format!("/research/strategies/{code}"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/metrics` 回测/实盘统计（中文键松散 map → Value）。
    pub fn strategy_metrics(&self, code: &str) -> Result<serde_json::Value, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/metrics"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/nav` 净值/回撤序列。
    pub fn strategy_nav(&self, code: &str) -> Result<StrategyNav, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/nav"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/segments` 分时段指标。
    pub fn strategy_segments(&self, code: &str) -> Result<StrategySegments, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/segments"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/periodic` 月度/年度收益。
    pub fn strategy_periodic(&self, code: &str) -> Result<StrategyPeriodic, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/periodic"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/positions` 最新持仓。
    /// 官方文档只说「最新持仓权重明细列表」；实测回的是该策略最后算出权重的**最近 3 个日期**
    /// （每个日期一组逐标的权重），端点无日期/翻页参数,所以拿不到更早的持仓——别当时间序列用。
    pub fn strategy_positions(&self, code: &str) -> Result<StrategyPositions, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/positions"), NO_QUERY)
    }

    /// `GET /research/strategies/positions/latest` 按 ts/cs 读取全部策略的最新权重。
    pub fn strategy_latest_positions(&self, weight_type: &str) -> Result<LatestWeights, Error> {
        self.get_research_json(
            "/research/strategies/positions/latest",
            &[("weight_type", weight_type.to_string())],
        )
    }

    /// `GET /research/strategies/{code}/recent-eval` 近期评估结论。
    pub fn strategy_recent_eval(&self, code: &str) -> Result<StrategyRecentEval, Error> {
        self.get_research_json(
            &format!("/research/strategies/{code}/recent-eval"),
            NO_QUERY,
        )
    }

    /// `GET /research/strategies/{code}/definition` 策略定义（松散 → Value）。
    pub fn strategy_definition(&self, code: &str) -> Result<serde_json::Value, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/definition"), NO_QUERY)
    }

    /// `GET /research/strategies/{code}/trades` 关键交易复盘（items 松散）。
    pub fn strategy_trades(
        &self,
        code: &str,
        query: &[(&str, String)],
    ) -> Result<TradesResponse, Error> {
        self.get_research_json(&format!("/research/strategies/{code}/trades"), query)
    }

    /// `GET /research/strategies/{code}/trades/{kline_key}/kline` 出入场 K 线窗口（松散 → Value）。
    pub fn strategy_kline(&self, code: &str, kline_key: &str) -> Result<serde_json::Value, Error> {
        self.get_research_json(
            &format!("/research/strategies/{code}/trades/{kline_key}/kline"),
            NO_QUERY,
        )
    }

    // 实盘写（不重试）：状态经 C# realtime 包装口（同步实盘镜像）；tag 走 research 面
    /// `PATCH /strategy/realtime/strategies/{code}/status` 切换 实盘/暂停/废弃。
    pub fn strategy_status(&self, code: &str, status: &str) -> Result<StatusUpdated, Error> {
        let path = format!("/strategy/realtime/strategies/{code}/status");
        let body = serde_json::json!({ "status": status });
        self.send_research_json("PATCH", &path, NO_QUERY, Some(&body))
    }

    /// `POST /research/strategies/{code}/tags` 加标签。
    pub fn strategy_tag_add(&self, code: &str, tag: &str) -> Result<TagUpdated, Error> {
        let path = format!("/research/strategies/{code}/tags");
        let body = serde_json::json!({ "tag": tag });
        self.send_research_json("POST", &path, NO_QUERY, Some(&body))
    }

    /// `DELETE /research/strategies/{code}/tags/{tag}` 删标签（无 body）。
    pub fn strategy_tag_rm(&self, code: &str, tag: &str) -> Result<TagUpdated, Error> {
        let path = format!("/research/strategies/{code}/tags/{tag}");
        self.send_research_json::<(), _>("DELETE", &path, NO_QUERY, None)
    }

    /// `POST /research/strategy-imports` 批量上传自包含策略 TOML 并登记进实盘库。
    pub fn strategy_register(&self, tomls: &[String]) -> Result<StrategiesImported, Error> {
        let body = serde_json::json!({ "tomls": tomls });
        self.send_research_json("POST", "/research/strategy-imports", NO_QUERY, Some(&body))
    }

    /// `PATCH /research/strategies/{code}/memo` 写用户笔记（空串=清除）。
    /// 走 research 面而不是 status 那个 `/strategy/realtime/*` 包装口——那是另一个下游服务
    /// （C# 实盘镜像），memo 只存在于 research 侧，打过去必然 404。
    pub fn strategy_memo(&self, code: &str, memo: &str) -> Result<MemoUpdated, Error> {
        let path = format!("/research/strategies/{code}/memo");
        let body = serde_json::json!({ "memo": memo });
        self.send_research_json("PATCH", &path, NO_QUERY, Some(&body))
    }

    // 实验/评审（读 + 候选删除写）
    /// `GET /research/experiments` 探索实验列表。
    pub fn experiment_list(&self) -> Result<ExperimentList, Error> {
        self.get_research_json("/research/experiments", NO_QUERY)
    }

    /// `GET /research/experiments/{id}` 实验概览。
    pub fn experiment_get(&self, id: &str) -> Result<ExperimentDetail, Error> {
        self.get_research_json(&format!("/research/experiments/{id}"), NO_QUERY)
    }

    /// `GET /research/experiments/{id}/strategies` 候选策略。
    pub fn experiment_strategies(&self, id: &str) -> Result<ExperimentStrategies, Error> {
        self.get_research_json(&format!("/research/experiments/{id}/strategies"), NO_QUERY)
    }

    /// `GET /research/experiments/{id}/review-matrix` 评审矩阵。
    pub fn experiment_review_matrix(&self, id: &str) -> Result<ReviewMatrix, Error> {
        self.get_research_json(
            &format!("/research/experiments/{id}/review-matrix"),
            NO_QUERY,
        )
    }

    /// `DELETE /research/experiments/{id}/strategies/{code}` 删除未入库的探索候选。
    pub fn experiment_delete_strategy(
        &self,
        id: &str,
        code: &str,
    ) -> Result<StrategyDeleted, Error> {
        let path = format!("/research/experiments/{id}/strategies/{code}");
        self.send_research_json::<(), _>("DELETE", &path, NO_QUERY, None)
    }

    /// `DELETE /research/experiments/{id}` 删整次探索执行（**物理删**，候选连同 run 目录一起没）。
    ///
    /// 后端两级护栏，`force` 只越得过软的那一级（目录最近有写入 40906）；「该 run 有实盘预热任务
    /// 正在跑」（40905）是后端确知的冲突，硬拒绝，force 无效——撞上它就是老实等。
    pub fn experiment_delete_run(&self, id: &str, force: bool) -> Result<RunDeleted, Error> {
        let path = format!("/research/experiments/{id}");
        let query: Vec<(&str, String)> = if force {
            vec![("force", "true".to_string())]
        } else {
            Vec::new()
        };
        self.send_research_json::<(), _>("DELETE", &path, &query, None)
            .map_err(|e| e.with_research_hint(ResearchHint::DeleteGuardrail))
    }

    // ── 研究面：策略赠予（跨用户复制实盘策略）────────────
    // A 发码打包最多 10 条实盘策略，B 凭码在自己库里得到独立副本（定义 + 实盘绩效 +
    // 历史目标权重，不带 memo/tags，落地状态固定「暂停」）。后端 Redis 里只存**引用**：
    // A 事后删/废弃任意一条，整码即不可领；已领走的副本不受影响。

    /// `POST /research/gifts` 发赠予码（写，不重试）。
    ///
    /// **返回的 `gift_code` 就是策略的访问凭证**——拿到码的人不需要别的授权就能领走这些
    /// 策略的完整定义，且发出即不可撤回地披露（撤回只能挡住还没领的人）。
    pub fn gift_create(
        &self,
        strategy_codes: &[String],
        max_claims: u32,
        ttl_days: u8,
    ) -> Result<GiftView, Error> {
        let body = serde_json::json!({
            "strategy_codes": strategy_codes,
            "max_claims": max_claims,
            "ttl_days": ttl_days,
        });
        self.send_research_json("POST", "/research/gifts", NO_QUERY, Some(&body))
    }

    /// `GET /research/gifts` 本人发出、尚未过期的赠予码（读）。
    /// `claimed` 与 `unavailable_strategy_codes` 都是读时现算的。
    pub fn gift_list(&self) -> Result<GiftList, Error> {
        self.get_research_json("/research/gifts", NO_QUERY)
    }

    /// `DELETE /research/gifts/{gift_code}` 撤回自己发出的码（写，不重试）。
    /// 只挡住**还没领的人**；已领走的副本是对方的资产，撤不回来。
    pub fn gift_revoke(&self, gift_code: &str) -> Result<GiftRevoked, Error> {
        let path = format!("/research/gifts/{gift_code}");
        self.send_research_json::<(), _>("DELETE", &path, NO_QUERY, None)
    }

    /// `GET /research/gifts/{gift_code}/preview` 领取前预览（读，零副作用）。
    /// 逐条给出可领状态 + 剩余名额 + `claimable`/`already_claimed`。
    pub fn gift_preview(&self, gift_code: &str) -> Result<GiftPreview, Error> {
        let path = format!("/research/gifts/{gift_code}/preview");
        self.get_research_json(&path, NO_QUERY)
    }

    /// `POST /research/gifts/{gift_code}/claim` 领取（写，不重试）。
    ///
    /// 后端对同一用户是幂等的（领过再领原样回放，不重复拷贝、不重复占名额），但**照样不套
    /// `with_retry`**——写不重试这条规则的价值全在零例外（同 `strategy memo`）。
    ///
    /// 打 `GiftClaim` 标：这里的 409 与删除类命令的 409 数字撞车（40907），语义却相反，
    /// 不打标就会挂上一条「确认后带 --force 重发」的建议，而领取根本没有 force 一说。
    pub fn gift_claim(&self, gift_code: &str) -> Result<GiftClaimed, Error> {
        let path = format!("/research/gifts/{gift_code}/claim");
        self.send_research_json::<(), _>("POST", &path, NO_QUERY, None)
            .map_err(|e| e.with_research_hint(ResearchHint::GiftClaim))
    }

    // promote（写：候选→实盘，触 FC 算力）+ 轮询
    /// `POST /research/experiments/{id}/strategies/{code}/promote` 保存入库并预热实时结果。
    /// `memo` 可选：后端**只在本次真的新插入时**写入，复用已有入库记录时静默忽略。
    /// 不传时不发这个键（而不是发 `null`），保持与后端加字段前的请求体逐字一致。
    pub fn promote_start(
        &self,
        id: &str,
        code: &str,
        memo: Option<&str>,
    ) -> Result<Promotion, Error> {
        let path = format!("/research/experiments/{id}/strategies/{code}/promote");
        let mut body = serde_json::Map::new();
        if let Some(memo) = memo {
            body.insert(
                "memo".to_string(),
                serde_json::Value::String(memo.to_string()),
            );
        }
        self.send_research_json(
            "POST",
            &path,
            NO_QUERY,
            Some(&serde_json::Value::Object(body)),
        )
    }

    /// `GET /research/promotions/{promotion_id}` 轮询 promote 终态。
    pub fn promotion_get(&self, promotion_id: &str) -> Result<Promotion, Error> {
        self.get_research_json(&format!("/research/promotions/{promotion_id}"), NO_QUERY)
    }

    // 组合（读 + 写：建组合触发 FC 组合优化，不重试）
    /// `GET /research/portfolios` 组合库列表（无查询参数，永远全量返回）。
    pub fn portfolio_list(&self) -> Result<PortfolioList, Error> {
        self.get_research_json("/research/portfolios", NO_QUERY)
    }

    /// `GET /research/portfolios/{code}` 组合详情。
    /// ⚠️ 组合仍在异步生成中或生成失败时同样 404——**别用这个端点轮询进度**，
    /// 那会被分类成 fix_params（像是 code 打错了）。轮询用 `portfolio_list` 的 `job_status`。
    pub fn portfolio_get(&self, code: &str) -> Result<PortfolioDetail, Error> {
        self.get_research_json(&format!("/research/portfolios/{code}"), NO_QUERY)
    }

    /// `POST /research/portfolios` 建组合（body 由 stdin 透传）。202 Accepted：
    /// 异步触发 Function Compute 组合优化，终态靠 `portfolio_list` 的 `job_status` 轮询。
    pub fn portfolio_create(&self, body: &serde_json::Value) -> Result<CreatePortfolioAck, Error> {
        self.send_research_json("POST", "/research/portfolios", NO_QUERY, Some(body))
    }

    // 研究问题（create 在 /strategy 面；其余走 /research）
    /// `GET /research/problems/meta` 建 problem 前发现合法枚举。
    pub fn problem_meta(&self) -> Result<ProblemMeta, Error> {
        self.get_research_json("/research/problems/meta", NO_QUERY)
    }

    /// `GET /research/problems` 研究问题列表（可筛选）。
    pub fn problem_list(&self, query: &[(&str, String)]) -> Result<ProblemList, Error> {
        self.get_research_json("/research/problems", query)
    }

    /// `GET /research/problems/{code}` 研究问题详情（含 time_segments）。
    pub fn problem_get(&self, code: &str) -> Result<ProblemView, Error> {
        self.get_research_json(&format!("/research/problems/{code}"), NO_QUERY)
    }

    /// `DELETE /research/problems/{code}` 物理删除用户创建的研究问题。
    pub fn problem_delete(&self, code: &str) -> Result<ProblemDeleted, Error> {
        let path = format!("/research/problems/{code}");
        self.send_research_json::<(), ()>("DELETE", &path, NO_QUERY, None)?;
        Ok(ProblemDeleted {
            code: code.to_string(),
            deleted: true,
        })
    }

    /// 两个“运行列表”端点共用的分页 GET（`status/page/size`，`status` 缺省不发）。
    fn paged_runs(
        &self,
        path: &str,
        status: Option<&str>,
        page: u32,
        size: u32,
    ) -> Result<Page<RunSummary>, Error> {
        let mut q: Vec<(&str, String)> =
            vec![("page", page.to_string()), ("size", size.to_string())];
        if let Some(s) = status {
            q.push(("status", s.to_string()));
        }
        self.get_json(path, &q)
    }
}

/// research 信封 `{code,msg,data}`；先保留 data 的 JSON 形态，再按端点类型解码。
#[derive(serde::Deserialize)]
struct ResearchEnvelope {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    msg: String,
    data: serde_json::Value,
}

/// 只探 research 信封的 code/msg（不知 data 类型时，用于非 2xx 分类）。
#[derive(serde::Deserialize)]
struct ResearchErr {
    code: Option<i64>,
    #[serde(default)]
    msg: String,
}

/// research 面 2xx：拆信封。
/// - `code==0` → 按目标类型解 data（`T=()` 时 data=null 合法；普通结构体仍拒 null）
/// - `code!=0` → 业务错（研究后端少数读接口会 HTTP 200 带业务码，如 42201），
///   归 `Error::Research`，由 `is_read` 决定 action（读 422→retry_later，写 422→fix_params）
fn unwrap_research_2xx<T: DeserializeOwned>(mut resp: Resp, is_read: bool) -> Result<T, Error> {
    let http = resp.status().as_u16();
    let retry_after_ms = resp
        .headers()
        .get("Retry-After")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|s| s.saturating_mul(1000));
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::Network(format!("读取响应失败: {e}")))?;
    let env: ResearchEnvelope = serde_json::from_str(&body)
        .map_err(|e| Error::Internal(format!("研究信封解析失败: {e}")))?;
    if env.code == 0 {
        serde_json::from_value(env.data)
            .map_err(|e| Error::Internal(format!("研究信封 data 解析失败: {e}")))
    } else {
        Err(Error::Research {
            http_status: http,
            code: env.code,
            msg: if env.msg.is_empty() {
                format!("HTTP {http}")
            } else {
                env.msg
            },
            is_read,
            retry_after_ms,
            hint: crate::error::ResearchHint::None,
        })
    }
}

/// research 面非 2xx：先按 research 信封（数值 code）→ `Error::Research`；
/// 不是信封（网关平台错误 {status,title,errorCode}，如 INSUFFICIENT_SCOPE）→ 回落平台解析。
fn research_err(mut resp: Resp, is_read: bool) -> Error {
    let http = resp.status().as_u16();
    let retry_after_ms = resp
        .headers()
        .get("Retry-After")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|s| s.saturating_mul(1000));
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    match serde_json::from_str::<ResearchErr>(&body) {
        Ok(ResearchErr {
            code: Some(code),
            msg,
        }) => Error::Research {
            http_status: http,
            code,
            msg: if msg.is_empty() {
                format!("HTTP {http}")
            } else {
                msg
            },
            is_read,
            retry_after_ms,
            // 这里只知道 HTTP 与数值 code，不知道是哪个端点发的；要挂 remediation 的
            // 调用点自己 `.with_research_hint(...)` 打标（见 `error::ResearchHint`）。
            hint: crate::error::ResearchHint::None,
        },
        _ => parse_api_error_body(http, &body, retry_after_ms),
    }
}

/// 平台统一错误结构 `{status, title, errorCode}` → 我们的 `Error::Api`。
fn parse_api_error(mut resp: Resp) -> Error {
    let code = resp.status().as_u16();
    let retry_after_ms = resp
        .headers()
        .get("Retry-After")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|s| s.saturating_mul(1000));
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    parse_api_error_body(code, &body, retry_after_ms)
}

/// 从已读出的 body 串解析平台错误（供 parse_api_error 与 research 回落共用）。
fn parse_api_error_body(code: u16, body: &str, retry_after_ms: Option<u64>) -> Error {
    #[derive(serde::Deserialize)]
    struct PlatErr {
        #[serde(default)]
        title: String,
        #[serde(rename = "errorCode", default)]
        error_code: String,
    }

    match serde_json::from_str::<PlatErr>(body) {
        Ok(pe) if !pe.error_code.is_empty() => Error::Api {
            status: code,
            code: pe.error_code,
            title: if pe.title.is_empty() {
                format!("HTTP {code}")
            } else {
                pe.title
            },
            retry_after_ms,
        },
        _ => {
            let summary: String = body.chars().take(200).collect();
            Error::Api {
                status: code,
                code: String::new(),
                title: if summary.is_empty() {
                    format!("HTTP {code}")
                } else {
                    format!("HTTP {code}: {summary}")
                },
                retry_after_ms,
            }
        }
    }
}
