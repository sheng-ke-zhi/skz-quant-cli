//! skz CLI：解析扁平命令、映射退出码、原子 JSON 输出、接管 clap、panic hook。
//!
//! 成功 → stdout 一份紧凑 JSON + exit 0；失败 → stderr `{"error":{...}}` + 动作退出码。

use std::io::{IsTerminal, Write};
use std::path::Path;

use clap::{Parser, Subcommand};
use skz::client::Client;
use skz::config::Config;
use skz::credentials;
use skz::error::{Error, ErrorBody};
use skz::retry;
use skz::skill;
use skz::update;

#[derive(Parser)]
#[command(
    name = "skz",
    about = "胜可知开放平台执行器（面向 AI agent）：市场数据只读查询 + 策略业务创建/触发",
    disable_version_flag = true,
    color = clap::ColorChoice::Never
)]
struct Cli {
    /// 人类排障用的美化输出（agent 勿用；不改变字段与结构）
    #[arg(long, global = true)]
    pretty: bool,

    /// 输出 CLI 与契约版本的 JSON
    #[arg(long = "version", short = 'V')]
    version: bool,

    /// 测试专用：指向 loopback mock（仅接受 127.0.0.1 / localhost）
    #[arg(long, hide = true, global = true)]
    base_url: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// 数据集及标的数量（先用它选 market）
    Markets,
    /// 标的分页查询；带 --keyword 即为搜索
    Symbols {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        size: u32,
    },
    /// 交易日历
    Calendar {
        exchange: String,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long = "only-open")]
        only_open: bool,
    },
    /// 研究方向：创建（写）/ 查已采用路线（读）
    Route {
        #[command(subcommand)]
        action: RouteCmd,
    },
    /// 研究问题：创建（写）/ 元数据·列表·详情（研究面读）
    Problem {
        #[command(subcommand)]
        action: ProblemCmd,
    },
    /// 因子挖掘（策略业务）：触发（写）/ 列表 / 批量轮询
    Mine {
        #[command(subcommand)]
        action: MineCmd,
    },
    /// 策略探索：触发（写）/ 单查 / 列表 / 批量轮询
    Explore {
        #[command(subcommand)]
        action: ExploreCmd,
    },
    /// 因子库（研究面读）：概览 / 列表 / 详情 / 软删
    Factor {
        #[command(subcommand)]
        action: FactorCmd,
    },
    /// 研究方向全集（研究面读）：GET /research/factor-routes
    #[command(name = "factor-routes")]
    FactorRoutes {
        #[command(subcommand)]
        action: FactorRoutesCmd,
    },
    /// 因子挖掘成果（研究面读）：run 列表 / overview / 挖出的因子清单
    Mining {
        #[command(subcommand)]
        action: MiningCmd,
    },
    /// 实盘策略（研究面读 + 实盘写）：库/详情/富读 + 状态/标签
    Strategy {
        #[command(subcommand)]
        action: StrategyCmd,
    },
    /// 探索实验（研究面）：列表 / 概览 / 候选 / 评审矩阵 / 删除候选
    Experiment {
        #[command(subcommand)]
        action: ExperimentCmd,
    },
    /// 毕业入库（写）+ 轮询：候选策略 → 实盘
    Promote {
        #[command(subcommand)]
        action: PromoteCmd,
    },
    /// 组合资产（研究面读 + 花钱写）：库列表/详情 + 建组合（触发 FC 组合优化）
    Portfolio {
        #[command(subcommand)]
        action: PortfolioCmd,
    },
    /// 开放平台身份自检（研究面读）：GET /research/whoami
    Whoami,
    /// 自更新：按安装渠道（pipx/uv）升级二进制，随后核对本机技能副本新鲜度
    Update,
    /// 凭据管理
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// 技能套件：安装到 harness / 查状态 / 卸载 / 权限建议 / 直读
    // 子命令叫 `skills`（复数）：这条命令操作的是一套四册技能，不是单个技能。
    #[command(name = "skills")]
    Skill {
        #[command(subcommand)]
        action: SkillCmd,
    },
}

#[derive(Subcommand)]
enum SkillCmd {
    /// 安装技能包到 harness 的技能目录（只写自己的目录，不碰任何配置文件）
    Install {
        /// claude | codex | openclaw | hermes | all（all = 本机装了的那些）
        #[arg(long, default_value = "claude")]
        target: String,
        /// user = 跨项目个人能力（默认）；project = 随当前仓库
        #[arg(long, default_value = "user")]
        scope: String,
    },
    /// 装没装 / 版本对不对（needs_install 为 true 就重装）
    Status {
        /// claude | codex | openclaw | hermes | all
        #[arg(long, default_value = "claude")]
        target: String,
        #[arg(long, default_value = "user")]
        scope: String,
    },
    /// 卸载（只删带 skz 安装标记的目录）
    Uninstall {
        /// claude | codex | openclaw | hermes | all
        #[arg(long, default_value = "claude")]
        target: String,
        #[arg(long, default_value = "user")]
        scope: String,
    },
    /// 打印建议的 HITL 权限规则（只输出文本，不改任何配置）
    Permissions,
    /// 直读技能正文：装不了的 harness 的兜底，也用于排障
    Show { name: Option<String> },
}

#[derive(Subcommand)]
enum RouteCmd {
    /// 创建研究方向（写）：从 stdin 读一份 JSON body，POST /strategy/routes，返回 {routeCode}
    Create,
    /// 已采用路线列表（读）：GET /strategy/routes/adopted，供 explore 挑 routeCode
    Adopted,
}

#[derive(Subcommand)]
enum ProblemCmd {
    /// 创建研究问题（写）：从 stdin 读一份 JSON body，POST /strategy/problems，返回 {problemCode}
    Create,
    /// 元数据（读）：GET /research/problems/meta —— 建 problem 前发现合法 type/dataset/freq
    Meta,
    /// 研究问题列表（读）：GET /research/problems
    List {
        #[arg(long)]
        q: Option<String>,
        #[arg(long)]
        dataset: Option<String>,
        #[arg(long)]
        freq: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long = "type")]
        problem_type: Option<String>,
    },
    /// 研究问题详情（读）：GET /research/problems/{code}
    Get { code: String },
}

#[derive(Subcommand)]
enum MineCmd {
    /// 触发因子挖掘（写，扣费）：POST /strategy/miner/runs，返回 {fcRunId,status,routeCode}
    Start {
        #[arg(long)]
        route: String,
    },
    /// 策略挖掘运行列表（读，分页）：GET /strategy/miner/runs
    Runs {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        size: u32,
    },
    /// 批量轮询进度（读）：POST /strategy/miner/poll，最多 100 个 fcRunId
    Poll {
        /// 一个或多个 fcRunId（空格分隔）
        ids: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ExploreCmd {
    /// 触发策略探索（写，扣费）：POST /strategy/explore，返回 {fcRunId,status}
    Start {
        #[arg(long)]
        problem: String,
        #[arg(long)]
        route: String,
        #[arg(long = "conversation-id")]
        conversation_id: Option<String>,
        #[arg(long = "tool-call-id")]
        tool_call_id: Option<String>,
    },
    /// 单个探索任务实时进度（读）：GET /strategy/explore/{fcRunId}
    Get { fc_run_id: String },
    /// 探索运行列表（读，分页）：GET /strategy/explore/runs
    Runs {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        size: u32,
    },
    /// 批量轮询进度（读）：POST /strategy/explore/poll，最多 100 个 fcRunId
    Poll {
        /// 一个或多个 fcRunId（空格分隔）
        ids: Vec<String>,
    },
}

#[derive(Subcommand)]
enum FactorCmd {
    /// 概览统计（读）：GET /research/factors/summary
    Summary,
    /// 因子列表（读，分页/筛选/排序）：GET /research/factors
    List {
        #[arg(long)]
        q: Option<String>,
        #[arg(long)]
        route: Option<String>,
        #[arg(long)]
        engine: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[arg(long = "include-deleted")]
        include_deleted: bool,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long = "page-size", default_value_t = 50)]
        page_size: u32,
    },
    /// 单因子详情（读）：GET /research/factors/{factor_name}
    Get { factor_name: String },
    /// 逻辑审核·软删（写，不重试）：DELETE /research/factors/{factor_name}
    Delete {
        factor_name: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum FactorRoutesCmd {
    /// 路线全集（读）：GET /research/factor-routes
    List,
}

#[derive(Subcommand)]
enum MiningCmd {
    /// 挖掘 run 列表（读）：GET /research/mining/runs
    Runs {
        #[arg(long)]
        route: Option<String>,
    },
    /// 单次 run 概览（读）：GET /research/mining/{run_id}/overview
    Overview { run_id: String },
    /// 单次 run 挖出的因子（读，分页/筛选/排序）：GET /research/mining/{run_id}/factors
    Factors {
        run_id: String,
        #[arg(long)]
        q: Option<String>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long = "pos-min")]
        pos_min: Option<f64>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long = "page-size", default_value_t = 20)]
        page_size: u32,
    },
}

#[derive(Subcommand)]
enum StrategyCmd {
    /// 实盘库列表（读）：GET /research/strategies
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        q: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[arg(long = "with-metrics")]
        with_metrics: bool,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long = "page-size", default_value_t = 20)]
        page_size: u32,
    },
    /// 详情（读）：GET /research/strategies/{code}
    Get { code: String },
    /// 指标（读）：GET /research/strategies/{code}/metrics
    Metrics { code: String },
    /// 净值（读）：GET /research/strategies/{code}/nav
    Nav { code: String },
    /// 分时段（读）：GET /research/strategies/{code}/segments
    Segments { code: String },
    /// 月度/年度（读）：GET /research/strategies/{code}/periodic
    Periodic { code: String },
    /// 持仓（读）：GET /research/strategies/{code}/positions
    Positions { code: String },
    /// 近期评估（读）：GET /research/strategies/{code}/recent-eval
    #[command(name = "recent-eval")]
    RecentEval { code: String },
    /// 定义（读）：GET /research/strategies/{code}/definition
    Definition { code: String },
    /// 关键交易复盘（读）：GET /research/strategies/{code}/trades
    Trades {
        code: String,
        #[arg(long)]
        year: Option<String>,
        #[arg(long)]
        kind: Option<String>,
    },
    /// 出入场 K 线（读）：GET /research/strategies/{code}/trades/{kline_key}/kline
    Kline { code: String, kline_key: String },
    /// 切换实盘状态（写，不重试）：PATCH /strategy/realtime/strategies/{code}/status
    Status {
        code: String,
        #[arg(long)]
        status: String,
    },
    /// 加标签（写）：POST /research/strategies/{code}/tags
    #[command(name = "tag-add")]
    TagAdd {
        code: String,
        #[arg(long)]
        tag: String,
    },
    /// 删标签（写）：DELETE /research/strategies/{code}/tags/{tag}
    #[command(name = "tag-rm")]
    TagRm { code: String, tag: String },
}

#[derive(Subcommand)]
enum ExperimentCmd {
    /// 列表（读）：GET /research/experiments
    List,
    /// 概览（读）：GET /research/experiments/{id}
    Get { id: String },
    /// 候选策略（读）：GET /research/experiments/{id}/strategies
    Strategies { id: String },
    /// 评审矩阵（读）：GET /research/experiments/{id}/review-matrix
    #[command(name = "review-matrix")]
    ReviewMatrix { id: String },
    /// 删除未入库候选（写，不重试）：DELETE /research/experiments/{id}/strategies/{code}
    Delete { id: String, code: String },
}

#[derive(Subcommand)]
enum PromoteCmd {
    /// 毕业入库（写，触 FC 算力，不重试）：POST /research/experiments/{id}/strategies/{code}/promote
    Start { id: String, code: String },
    /// 轮询 promote 终态（读）：GET /research/promotions/{promotion_id}
    Get { promotion_id: String },
}

#[derive(Subcommand)]
enum PortfolioCmd {
    /// 组合库列表（读，无查询参数，永远全量）：GET /research/portfolios
    List,
    /// 组合详情（读）：GET /research/portfolios/{code}
    /// ⚠️ 生成中/生成失败也是 404——轮询建组合进度用 `portfolio list` 的 job_status，别用这个
    Get { code: String },
    /// 建组合（写，触发 FC 组合优化，扣费，不重试）：从 stdin 读一份 JSON body，
    /// POST /research/portfolios，返回 {portfolio_code,status:"pending"}
    Create,
}

#[derive(Subcommand)]
enum AuthCmd {
    /// 从 stdin 读 token 并存入受限权限文件
    Set,
    /// 报告 token 是否就绪（JSON，不打印 token）
    Status,
    /// 删除本地 credentials
    Unset,
}

#[derive(serde::Serialize)]
struct ErrorEnvelope<'a> {
    error: &'a ErrorBody,
}

fn main() {
    install_panic_hook();
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => return handle_clap_error(e),
    };
    match dispatch(cli) {
        Ok(()) => 0,
        Err(err) => {
            emit_error(&err);
            err.exit_code()
        }
    }
}

/// clap 分派：显式 --help / --version → stdout + exit 0；真错误 → JSON + exit 2。
fn handle_clap_error(e: clap::Error) -> i32 {
    use clap::error::ErrorKind as Ck;
    match e.kind() {
        Ck::DisplayHelp | Ck::DisplayVersion => {
            let _ = e.print();
            0
        }
        _ => {
            let rendered = e.render().to_string();
            let msg = rendered.lines().next().unwrap_or("参数错误").to_string();
            emit_error(&Error::Args(msg));
            2
        }
    }
}

fn dispatch(cli: Cli) -> Result<(), Error> {
    if cli.version {
        emit_value(
            // 契约版本单处定义：技能安装标记也比对它，两处若各写字面量就会
            // 出现「防漂移功能自己漂移」——skill status 与 --version 各说各话。
            &serde_json::json!({ "cli": env!("CARGO_PKG_VERSION"), "contract": skill::CONTRACT }),
            false,
        );
        return Ok(());
    }

    let Cli {
        pretty,
        base_url,
        command,
        ..
    } = cli;
    let command = command.ok_or_else(|| Error::Args("缺少子命令；见 `skz --help`".to_string()))?;

    match command {
        Command::Skill { action } => run_skill(action, pretty),
        Command::Auth { action } => run_auth(action),
        Command::Markets => {
            let client = make_client(base_url)?;
            let data = retry::with_retry(|| client.markets())?;
            emit_value(&data, pretty);
            Ok(())
        }
        Command::Symbols {
            market,
            keyword,
            page,
            size,
        } => {
            validate_page_size(page, size)?;
            if let Some(k) = &keyword
                && k.is_empty()
            {
                return Err(Error::Args("keyword 不得为空字符串".to_string()));
            }
            let client = make_client(base_url)?;
            let data = retry::with_retry(|| {
                client.symbols(market.as_deref(), keyword.as_deref(), page, size)
            })?;
            emit_value(&data, pretty);
            Ok(())
        }
        Command::Calendar {
            exchange,
            start,
            end,
            only_open,
        } => {
            if exchange.trim().is_empty() {
                return Err(Error::Args("exchange 不得为空".to_string()));
            }
            if let Some(s) = &start {
                validate_date(s)?;
            }
            if let Some(e) = &end {
                validate_date(e)?;
            }
            if let (Some(s), Some(e)) = (&start, &end)
                && s > e
            {
                return Err(Error::Args("start 必须 <= end".to_string()));
            }
            let client = make_client(base_url)?;
            let data = retry::with_retry(|| {
                client.calendar(&exchange, start.as_deref(), end.as_deref(), only_open)
            })?;
            emit_value(&data, pretty);
            Ok(())
        }
        Command::Route { action } => run_route(action, base_url, pretty),
        Command::Problem { action } => run_problem(action, base_url, pretty),
        Command::Mine { action } => run_mine(action, base_url, pretty),
        Command::Explore { action } => run_explore(action, base_url, pretty),
        Command::Factor { action } => run_factor(action, base_url, pretty),
        Command::FactorRoutes { action } => run_factor_routes(action, base_url, pretty),
        Command::Mining { action } => run_mining(action, base_url, pretty),
        Command::Strategy { action } => run_strategy(action, base_url, pretty),
        Command::Experiment { action } => run_experiment(action, base_url, pretty),
        Command::Promote { action } => run_promote(action, base_url, pretty),
        Command::Portfolio { action } => run_portfolio(action, base_url, pretty),
        Command::Whoami => {
            let client = make_client(base_url)?;
            let data = retry::with_retry(|| client.whoami())?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 零 HTTP 调用，不吃 base_url——跟其它分支的样板代码不一样，别顺手抄过来。
        Command::Update => run_update(pretty),
    }
}

// ── 策略业务分派 ────────────────────────────────────────────────
// 约定：读命令（含 poll）走 `retry::with_retry`；写/触发命令**直接调用不重试**
// （无幂等保证、触发即扣费）。

fn run_route(action: RouteCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        // 写：不重试
        RouteCmd::Create => {
            let body = read_stdin_json()?;
            let data = client
                .create_route(&body)
                .map_err(|e| e.into_write_unknown("skz factor-routes list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        RouteCmd::Adopted => {
            let data = retry::with_retry(|| client.adopted_routes())?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_problem(action: ProblemCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        // 写：不重试
        ProblemCmd::Create => {
            let body = read_stdin_json()?;
            let data = client
                .create_problem(&body)
                .map_err(|e| e.into_write_unknown("skz problem list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        ProblemCmd::Meta => {
            let data = retry::with_retry(|| client.problem_meta())?;
            emit_value(&data, pretty);
            Ok(())
        }
        ProblemCmd::List {
            q,
            dataset,
            freq,
            source,
            problem_type,
        } => {
            let mut query: Vec<(&str, String)> = vec![];
            push_opt(&mut query, "q", &q);
            push_opt(&mut query, "dataset", &dataset);
            push_opt(&mut query, "freq", &freq);
            push_opt(&mut query, "source", &source);
            push_opt(&mut query, "problem_type", &problem_type);
            let data = retry::with_retry(|| client.problem_list(&query))?;
            emit_value(&data, pretty);
            Ok(())
        }
        ProblemCmd::Get { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.problem_get(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

// ── 研究面：策略实盘（读 + 实盘写）/ 实验 / promote ────────────

fn run_strategy(action: StrategyCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        StrategyCmd::List {
            status,
            q,
            sort,
            order,
            with_metrics,
            page,
            page_size,
        } => {
            validate_page_size_max(page, page_size, 1000)?;
            let mut query: Vec<(&str, String)> = vec![
                ("page", page.to_string()),
                ("page_size", page_size.to_string()),
            ];
            push_opt(&mut query, "status", &status);
            push_opt(&mut query, "q", &q);
            push_opt(&mut query, "sort", &sort);
            push_opt(&mut query, "order", &order);
            if with_metrics {
                query.push(("with_metrics", "true".to_string()));
            }
            let data = retry::with_retry(|| client.strategy_list(&query))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Get { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_get(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Metrics { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_metrics(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Nav { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_nav(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Segments { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_segments(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Periodic { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_periodic(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Positions { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_positions(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::RecentEval { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_recent_eval(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Definition { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.strategy_definition(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Trades { code, year, kind } => {
            require_nonempty(&code, "code")?;
            let mut query: Vec<(&str, String)> = vec![];
            push_opt(&mut query, "year", &year);
            push_opt(&mut query, "kind", &kind);
            let data = retry::with_retry(|| client.strategy_trades(&code, &query))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Kline { code, kline_key } => {
            require_nonempty(&code, "code")?;
            require_nonempty(&kline_key, "kline_key")?;
            let data = retry::with_retry(|| client.strategy_kline(&code, &kline_key))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试
        StrategyCmd::Status { code, status } => {
            require_nonempty(&code, "code")?;
            validate_live_status(&status)?;
            let data = client
                .strategy_status(&code, &status)
                .map_err(|e| e.into_write_unknown("skz strategy get <code>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::TagAdd { code, tag } => {
            require_nonempty(&code, "code")?;
            require_nonempty(&tag, "tag")?;
            let data = client
                .strategy_tag_add(&code, &tag)
                .map_err(|e| e.into_write_unknown("skz strategy get <code>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::TagRm { code, tag } => {
            require_nonempty(&code, "code")?;
            require_nonempty(&tag, "tag")?;
            let data = client
                .strategy_tag_rm(&code, &tag)
                .map_err(|e| e.into_write_unknown("skz strategy get <code>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_experiment(
    action: ExperimentCmd,
    base_url: Option<String>,
    pretty: bool,
) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        ExperimentCmd::List => {
            let data = retry::with_retry(|| client.experiment_list())?;
            emit_value(&data, pretty);
            Ok(())
        }
        ExperimentCmd::Get { id } => {
            require_nonempty(&id, "id")?;
            let data = retry::with_retry(|| client.experiment_get(&id))?;
            emit_value(&data, pretty);
            Ok(())
        }
        ExperimentCmd::Strategies { id } => {
            require_nonempty(&id, "id")?;
            let data = retry::with_retry(|| client.experiment_strategies(&id))?;
            emit_value(&data, pretty);
            Ok(())
        }
        ExperimentCmd::ReviewMatrix { id } => {
            require_nonempty(&id, "id")?;
            let data = retry::with_retry(|| client.experiment_review_matrix(&id))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试。超时后用候选清单确认 code 是否仍存在。
        ExperimentCmd::Delete { id, code } => {
            require_nonempty(&id, "id")?;
            require_nonempty(&code, "code")?;
            let data = client
                .experiment_delete_strategy(&id, &code)
                .map_err(|e| e.into_write_unknown("skz experiment strategies <id>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_promote(action: PromoteCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        // 写：不重试（触发即扣算力）
        PromoteCmd::Start { id, code } => {
            require_nonempty(&id, "id")?;
            require_nonempty(&code, "code")?;
            let data = client
                .promote_start(&id, &code)
                .map_err(|e| e.into_write_unknown("skz strategy list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        PromoteCmd::Get { promotion_id } => {
            require_nonempty(&promotion_id, "promotion_id")?;
            let data = retry::with_retry(|| client.promotion_get(&promotion_id))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_portfolio(
    action: PortfolioCmd,
    base_url: Option<String>,
    pretty: bool,
) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        PortfolioCmd::List => {
            let data = retry::with_retry(|| client.portfolio_list())?;
            emit_value(&data, pretty);
            Ok(())
        }
        PortfolioCmd::Get { code } => {
            require_nonempty(&code, "code")?;
            let data = retry::with_retry(|| client.portfolio_get(&code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试（触发即扣 FC 算力）
        PortfolioCmd::Create => {
            let body = read_stdin_json()?;
            let data = client
                .portfolio_create(&body)
                .map_err(|e| e.into_write_unknown("skz portfolio list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_mine(action: MineCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        // 写/触发：不重试
        MineCmd::Start { route } => {
            if route.trim().is_empty() {
                return Err(Error::Args("route 不得为空".to_string()));
            }
            let data = client
                .trigger_mine(&route)
                .map_err(|e| e.into_write_unknown("skz mine runs --status active"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        MineCmd::Runs { status, page, size } => {
            validate_page_size(page, size)?;
            let data =
                retry::with_retry(|| client.strategy_miner_runs(status.as_deref(), page, size))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读（poll，幂等）：重试
        MineCmd::Poll { ids } => {
            validate_run_ids(&ids)?;
            let data = retry::with_retry(|| client.strategy_miner_poll(&ids))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_explore(action: ExploreCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        // 写/触发：不重试
        ExploreCmd::Start {
            problem,
            route,
            conversation_id,
            tool_call_id,
        } => {
            if problem.trim().is_empty() {
                return Err(Error::Args("problem 不得为空".to_string()));
            }
            if route.trim().is_empty() {
                return Err(Error::Args("route 不得为空".to_string()));
            }
            let data = client
                .trigger_explore(
                    &problem,
                    &route,
                    conversation_id.as_deref(),
                    tool_call_id.as_deref(),
                )
                .map_err(|e| e.into_write_unknown("skz explore runs --status active"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        ExploreCmd::Get { fc_run_id } => {
            if fc_run_id.trim().is_empty() {
                return Err(Error::Args("fcRunId 不得为空".to_string()));
            }
            let data = retry::with_retry(|| client.explore_get(&fc_run_id))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        ExploreCmd::Runs { status, page, size } => {
            validate_page_size(page, size)?;
            let data = retry::with_retry(|| client.explore_runs(status.as_deref(), page, size))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读（poll，幂等）：重试
        ExploreCmd::Poll { ids } => {
            validate_run_ids(&ids)?;
            let data = retry::with_retry(|| client.explore_poll(&ids))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

// ── 研究面：因子库 / 挖掘成果（全读，除 factor delete 是写）────────────

fn run_factor(action: FactorCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        FactorCmd::Summary => {
            let data = retry::with_retry(|| client.factor_summary())?;
            emit_value(&data, pretty);
            Ok(())
        }
        FactorCmd::List {
            q,
            route,
            engine,
            tag,
            sort,
            order,
            include_deleted,
            page,
            page_size,
        } => {
            validate_page_size_max(page, page_size, 200)?;
            let mut query: Vec<(&str, String)> = vec![
                ("page", page.to_string()),
                ("page_size", page_size.to_string()),
            ];
            push_opt(&mut query, "q", &q);
            push_opt(&mut query, "route", &route);
            push_opt(&mut query, "engine", &engine);
            push_opt(&mut query, "tag", &tag);
            push_opt(&mut query, "sort", &sort);
            push_opt(&mut query, "order", &order);
            if include_deleted {
                query.push(("include_deleted", "true".to_string()));
            }
            let data = retry::with_retry(|| client.factors(&query))?;
            emit_value(&data, pretty);
            Ok(())
        }
        FactorCmd::Get { factor_name } => {
            require_nonempty(&factor_name, "factor_name")?;
            let data = retry::with_retry(|| client.factor_get(&factor_name))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试
        FactorCmd::Delete {
            factor_name,
            reason,
        } => {
            require_nonempty(&factor_name, "factor_name")?;
            let data = client
                .factor_delete(&factor_name, reason.as_deref())
                .map_err(|e| e.into_write_unknown("skz factor get <factor_name>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_factor_routes(
    action: FactorRoutesCmd,
    base_url: Option<String>,
    pretty: bool,
) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        FactorRoutesCmd::List => {
            let data = retry::with_retry(|| client.factor_routes())?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_mining(action: MiningCmd, base_url: Option<String>, pretty: bool) -> Result<(), Error> {
    let client = make_client(base_url)?;
    match action {
        MiningCmd::Runs { route } => {
            let data = retry::with_retry(|| client.mining_runs(route.as_deref()))?;
            emit_value(&data, pretty);
            Ok(())
        }
        MiningCmd::Overview { run_id } => {
            require_nonempty(&run_id, "run_id")?;
            let data = retry::with_retry(|| client.mining_overview(&run_id))?;
            emit_value(&data, pretty);
            Ok(())
        }
        MiningCmd::Factors {
            run_id,
            q,
            group,
            pos_min,
            sort,
            order,
            page,
            page_size,
        } => {
            require_nonempty(&run_id, "run_id")?;
            validate_page_size_max(page, page_size, 200)?;
            let mut query: Vec<(&str, String)> = vec![
                ("page", page.to_string()),
                ("page_size", page_size.to_string()),
            ];
            push_opt(&mut query, "q", &q);
            push_opt(&mut query, "group", &group);
            push_opt(&mut query, "sort", &sort);
            push_opt(&mut query, "order", &order);
            if let Some(pm) = pos_min {
                query.push(("pos_min", pm.to_string()));
            }
            let data = retry::with_retry(|| client.mining_factors(&run_id, &query))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_auth(action: AuthCmd) -> Result<(), Error> {
    match action {
        AuthCmd::Set => credentials::set_from_stdin(),
        AuthCmd::Status => {
            let present = credentials::is_present()?;
            emit_value(&serde_json::json!({ "present": present }), false);
            Ok(())
        }
        AuthCmd::Unset => credentials::unset(),
    }
}

/// 技能套件分派。install/status/uninstall/permissions 同样守 I/O 契约：
/// 一份紧凑 JSON + exit 0；只有 `show` 出 markdown 正文（它就是给人/agent 直读的）。
fn run_skill(action: SkillCmd, pretty: bool) -> Result<(), Error> {
    match action {
        SkillCmd::Install { target, scope } => {
            let sc = parse_scope(&scope)?;
            emit_multi(
                &parse_targets(&target, sc)?
                    .into_iter()
                    .map(|t| skill::install(t, sc))
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        SkillCmd::Status { target, scope } => {
            let sc = parse_scope(&scope)?;
            emit_multi(
                &parse_targets(&target, sc)?
                    .into_iter()
                    .map(|t| skill::status(t, sc))
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        SkillCmd::Uninstall { target, scope } => {
            let sc = parse_scope(&scope)?;
            emit_multi(
                &parse_targets(&target, sc)?
                    .into_iter()
                    .map(|t| skill::uninstall(t, sc))
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        SkillCmd::Permissions => {
            emit_value(&skill::permissions(), pretty);
            Ok(())
        }
        SkillCmd::Show { name } => {
            emit_raw(&skill::show(name.as_deref())?);
            Ok(())
        }
    }
}

/// `run_update` 内部装配用的中间态，不对外输出——避免用一个宽元组在几个分支间传参数。
struct UpdateOutcome {
    attempted: bool,
    updated: Option<bool>,
    cli_after: Option<String>,
    /// 技能 staleness 比对基准：没升级（或确认没变）时是 `env!()` 自己，确认升级后是
    /// 新探测到的版本。`ref_contract` 同理。
    ref_cli: String,
    ref_contract: String,
    /// 确认发生了版本变化 → 刷新要转手给磁盘上的新二进制（`refresh_delegated`）；
    /// 否则当前进程内嵌的内容仍然权威（`refresh_in_process`）。
    delegate_refresh: bool,
    remediation: Option<serde_json::Value>,
}

/// 自更新：探测渠道 → shell 出升级命令（子进程真失败就 `?` 直接走错误路径，技能核对
/// 不跑——I/O 契约里成功/失败二选一，没有"顺便"报告技能状态的空间，重跑命令会在下次
/// 成功时一并拿到两个事实）→ 成功后重新探测磁盘上的新版本 → 核对本机技能副本新鲜度 →
/// 真终端时问要不要刷新。
fn run_update(pretty: bool) -> Result<(), Error> {
    let exe = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| Error::Internal(format!("无法定位当前可执行文件路径: {e}")))?;
    let channel = update::detect_channel(&exe);

    let outcome = match channel {
        update::Channel::Unknown => UpdateOutcome {
            attempted: false,
            updated: Some(false),
            cli_after: None,
            ref_cli: env!("CARGO_PKG_VERSION").to_string(),
            ref_contract: skill::CONTRACT.to_string(),
            delegate_refresh: false,
            remediation: Some(unknown_channel_remediation()),
        },
        _ => {
            update::upgrade(channel)?;
            match update::probe_version(&exe) {
                Some(v) if v.cli != env!("CARGO_PKG_VERSION") || v.contract != skill::CONTRACT => {
                    UpdateOutcome {
                        attempted: true,
                        updated: Some(true),
                        cli_after: Some(v.cli.clone()),
                        ref_cli: v.cli,
                        ref_contract: v.contract,
                        delegate_refresh: true,
                        remediation: None,
                    }
                }
                Some(v) => UpdateOutcome {
                    attempted: true,
                    updated: Some(false),
                    cli_after: Some(v.cli),
                    ref_cli: env!("CARGO_PKG_VERSION").to_string(),
                    ref_contract: skill::CONTRACT.to_string(),
                    delegate_refresh: false,
                    remediation: None,
                },
                None => UpdateOutcome {
                    attempted: true,
                    updated: None,
                    cli_after: None,
                    ref_cli: String::new(),
                    ref_contract: String::new(),
                    delegate_refresh: false,
                    remediation: None,
                },
            }
        }
    };

    let skills = build_skills_report(&outcome, &exe)?;

    emit_value(
        &update::UpdateReport {
            channel: channel.as_str(),
            attempted: outcome.attempted,
            updated: outcome.updated,
            cli: env!("CARGO_PKG_VERSION"),
            cli_after: outcome.cli_after,
            remediation: outcome.remediation,
            skills,
        },
        pretty,
    );
    Ok(())
}

/// 组装技能新鲜度小节，真终端场景下顺带问人要不要刷新。`current_exe()` 已经在
/// `run_update` 里探测过；这里剩下唯二"不纯"的输入是 stdin/stderr 的 TTY 探测——
/// 天然只属于"直接终端调用"这个场景，留在 bin 这层，不下沉进 lib。
fn build_skills_report(outcome: &UpdateOutcome, exe: &Path) -> Result<update::SkillsReport, Error> {
    let targets = skill::present_targets();
    let checked_targets: Vec<&'static str> = targets.iter().map(|t| t.as_str()).collect();

    if outcome.updated.is_none() {
        // 升级子进程成功了，但确认不了磁盘上的新版本号——没有可信的比对基准，
        // 宁可不评估，也不要拿一个可能错的基准假装评估过。
        return Ok(update::SkillsReport {
            checked_targets,
            evaluated: false,
            skip_reason: Some(
                "升级后无法确认磁盘上的新版本号（--version 自检失败），跳过技能新鲜度核对；\
                 重跑 `skz update` 或手动跑 `skz skills status` 确认"
                    .to_string(),
            ),
            stale: vec![],
            refresh_offered: false,
            refresh_accepted: None,
            refreshed: None,
        });
    }

    let marked = update::installed_books(&targets)?;
    let stale = update::find_stale(&marked, &outcome.ref_cli, &outcome.ref_contract);

    let mut refresh_offered = false;
    let mut refresh_accepted = None;
    let mut refreshed = None;

    if !stale.is_empty() && std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        refresh_offered = true;
        if prompt_refresh(&stale) {
            refresh_accepted = Some(true);
            let mut stale_targets: Vec<skill::Target> = Vec::new();
            for s in &stale {
                // target 字符串只可能来自 Target::as_str()，四选一必中；expect 是诚实的
                // 断言，不是掩盖真会失败的路径。
                let t = parse_target(s.target).expect("target 来自 Target::as_str()");
                if !stale_targets.contains(&t) {
                    stale_targets.push(t);
                }
            }
            refreshed = Some(if outcome.delegate_refresh {
                update::refresh_delegated(exe, &stale_targets)
            } else {
                update::refresh_in_process(&stale_targets)
            });
        } else {
            refresh_accepted = Some(false);
        }
    }

    Ok(update::SkillsReport {
        checked_targets,
        evaluated: true,
        skip_reason: None,
        stale,
        refresh_offered,
        refresh_accepted,
        refreshed,
    })
}

/// 真终端时问要不要刷新过期技能；读失败按"否"处理，宁可保守。
fn prompt_refresh(stale: &[update::StaleSkill]) -> bool {
    let mut err = std::io::stderr();
    let _ = write!(
        err,
        "{} 个技能副本落后于当前二进制版本，现在刷新吗？[y/N] ",
        stale.len()
    );
    let _ = err.flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// 识别不出安装渠道时的兜底：指回 README 的两条公开安装命令。
fn unknown_channel_remediation() -> serde_json::Value {
    serde_json::json!({
        "howTo": "本机没能识别出受支持的 uv tool 或 pipx 安装，跳过自更新。可用下面任一公开渠道重新安装：",
        "commands": [
            "pipx install skz-quant-cli",
            "uv tool install skz-quant-cli"
        ]
    })
}

fn parse_target(s: &str) -> Result<skill::Target, Error> {
    match s {
        "claude" => Ok(skill::Target::Claude),
        "codex" => Ok(skill::Target::Codex),
        "openclaw" => Ok(skill::Target::Openclaw),
        "hermes" => Ok(skill::Target::Hermes),
        other => Err(Error::Args(format!(
            "未知 target {other}；可选 claude | codex | openclaw | hermes | all"
        ))),
    }
}

/// `--target` 解析成一组：`all` = 本机装了的那些 harness（user scope 才有意义，
/// 因为探测看的是 home 下的配置目录）。**不给不存在的 harness 造目录**——
/// 那既没用又是噪音。project scope 下 `all` 退化成四家全列（cwd 里本来就没有痕迹可探）。
fn parse_targets(s: &str, scope: skill::Scope) -> Result<Vec<skill::Target>, Error> {
    if s != "all" {
        return Ok(vec![parse_target(s)?]);
    }
    let found: Vec<_> = match scope {
        skill::Scope::User => skill::present_targets(),
        skill::Scope::Project => skill::Target::ALL.to_vec(),
    };
    if found.is_empty() {
        return Err(Error::Args(
            "本机没发现任何受支持的 harness（找不到 ~/.claude、~/.codex、~/.openclaw、~/.hermes）；\
             用 --target <名字> 指定一个，或先装上对应 harness"
                .to_string(),
        ));
    }
    Ok(found)
}

fn parse_scope(s: &str) -> Result<skill::Scope, Error> {
    match s {
        "user" => Ok(skill::Scope::User),
        "project" => Ok(skill::Scope::Project),
        other => Err(Error::Args(format!(
            "未知 scope {other}；可选 user | project"
        ))),
    }
}

fn make_client(base_url: Option<String>) -> Result<Client, Error> {
    let cfg = Config::new(base_url)?;
    let token = credentials::load_token()?;
    Ok(Client::new(&cfg, token))
}

fn validate_page_size(page: u32, size: u32) -> Result<(), Error> {
    if page < 1 {
        return Err(Error::Args("page 必须 >= 1".to_string()));
    }
    if !(1..=200).contains(&size) {
        return Err(Error::Args("size 必须在 1..=200".to_string()));
    }
    Ok(())
}

/// page/size 校验，size 上限按端点给（factor≤200 / strategy≤1000）。
fn validate_page_size_max(page: u32, size: u32, max: u32) -> Result<(), Error> {
    if page < 1 {
        return Err(Error::Args("page 必须 >= 1".to_string()));
    }
    if !(1..=max).contains(&size) {
        return Err(Error::Args(format!("page-size 必须在 1..={max}")));
    }
    Ok(())
}

/// 位置参数（code / id / run_id / factor_name）非空本地校验。
fn require_nonempty(s: &str, name: &str) -> Result<(), Error> {
    if s.trim().is_empty() {
        Err(Error::Args(format!("{name} 不得为空")))
    } else {
        Ok(())
    }
}

/// 可选 query 参数：Some 才推入（key 为静态字面量）。
fn push_opt<'a>(q: &mut Vec<(&'a str, String)>, key: &'a str, val: &Option<String>) {
    if let Some(v) = val {
        q.push((key, v.clone()));
    }
}

/// 实盘状态枚举本地校验（先本地失败，省一次网络往返；写命令）。
fn validate_live_status(s: &str) -> Result<(), Error> {
    match s {
        "实盘" | "暂停" | "废弃" => Ok(()),
        _ => Err(Error::Args("status 仅接受 实盘|暂停|废弃".to_string())),
    }
}

/// poll 的 fcRunId 列表：非空、且不超过平台上限 100（本地校验，发网络前失败）。
fn validate_run_ids(ids: &[String]) -> Result<(), Error> {
    if ids.is_empty() {
        return Err(Error::Args("至少给一个 fcRunId".to_string()));
    }
    if ids.len() > 100 {
        return Err(Error::Args("一次最多 100 个 fcRunId".to_string()));
    }
    if ids.iter().any(|s| s.trim().is_empty()) {
        return Err(Error::Args("fcRunId 不得为空字符串".to_string()));
    }
    Ok(())
}

/// 从 stdin 读一份 JSON body（创建类写命令的复杂输入）。
/// 只本地校验“是合法 JSON object”——非法/空/非对象在发网络前失败（exit 2）；
/// 字段合法性交后端（400 → fix_params）。
fn read_stdin_json() -> Result<serde_json::Value, Error> {
    use std::io::Read as _;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::Internal(format!("读取 stdin 失败: {e}")))?;
    if buf.trim().is_empty() {
        return Err(Error::Args(
            "stdin 为空：创建类命令需从 stdin 传入一份 JSON body".to_string(),
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(&buf).map_err(|e| Error::Args(format!("stdin 不是合法 JSON: {e}")))?;
    if !value.is_object() {
        return Err(Error::Args("stdin JSON 必须是一个对象 {…}".to_string()));
    }
    Ok(value)
}

/// yyyy-MM-dd 校验（含真实日历日；ISO 字典序=时间序，比较交给调用方）。
fn validate_date(s: &str) -> Result<(), Error> {
    let b = s.as_bytes();
    let shape = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit);
    if !shape {
        return Err(Error::Args(format!("日期必须为 yyyy-MM-dd: {s}")));
    }
    let y: i32 = s[0..4].parse().unwrap();
    let m: u32 = s[5..7].parse().unwrap();
    let d: u32 = s[8..10].parse().unwrap();
    let dim = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    };
    if m == 0 || d == 0 || d > dim {
        return Err(Error::Args(format!("非法日期: {s}")));
    }
    Ok(())
}

/// 多 target 的技能报告：**单 target 仍出原来的对象**（形状不变、老脚本不破），
/// 只有 `--target all` 命中多家时才出数组。仍是单次 stdout 写。
fn emit_multi<T: serde::Serialize>(reports: &[T], pretty: bool) {
    match reports {
        [one] => emit_value(one, pretty),
        many => emit_value(&many, pretty),
    }
}

/// 技能正文直读：唯一非 JSON 的 stdout 出口（`skill show`），同样单次写。
fn emit_raw(text: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(text.as_bytes());
    let _ = lock.flush();
}

/// 成功输出：单次写、结尾换行、flush；进程内无第二处写 stdout。
fn emit_value<T: serde::Serialize>(data: &T, pretty: bool) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if pretty {
        let _ = serde_json::to_writer_pretty(&mut lock, data);
    } else {
        let _ = serde_json::to_writer(&mut lock, data);
    }
    let _ = lock.write_all(b"\n");
    let _ = lock.flush();
}

fn emit_error(err: &Error) {
    let body = err.to_body();
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = serde_json::to_writer(&mut lock, &ErrorEnvelope { error: &body });
    let _ = lock.write_all(b"\n");
    let _ = lock.flush();
}

/// panic → JSON internal 错误 + exit 6（release 用 panic=abort，故不走 catch_unwind）。
/// 只带位置、不带 payload，避免万一泄露。
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        emit_error(&Error::Internal(format!("panic at {loc}")));
        std::process::exit(6);
    }));
}
