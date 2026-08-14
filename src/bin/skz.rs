//! skz CLI：解析扁平命令、映射退出码、原子 JSON 输出、接管 clap、panic hook。
//!
//! 成功 → stdout 一份紧凑 JSON + exit 0；失败 → stderr `{"error":{...}}` + 动作退出码。

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use skz::client::Client;
use skz::config::{self, Config};
use skz::credentials;
use skz::error::{Error, ErrorBody};
use skz::plugin;
use skz::retry;
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
        #[arg(long, default_value_t = 5)]
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
    /// 研究问题：创建·删除（写）/ 元数据·列表·详情（研究面读）
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
    /// 保存入库（写）+ 轮询：候选策略 → 暂停态策略资产
    Promote {
        #[command(subcommand)]
        action: PromoteCmd,
    },
    /// 策略赠予（研究面）：发码 / 我发出的码 / 撤回 / 预览 / 领取
    Gift {
        #[command(subcommand)]
        action: GiftCmd,
    },
    /// 组合资产（研究面读 + 花钱写）：库列表/详情 + 建组合（触发 FC 组合优化）
    Portfolio {
        #[command(subcommand)]
        action: PortfolioCmd,
    },
    /// 开放平台身份自检（研究面读）：GET /research/whoami
    Whoami,
    /// 自更新：按安装渠道升级，随后核对本机 plugin
    Update,
    /// 凭据管理
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// SKZ plugin：安装 / 查状态 / 升级 / 卸载
    Plugin {
        #[command(subcommand)]
        action: PluginCmd,
    },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// 安装当前 CLI 随附的 SKZ plugin
    Install {
        /// claude | codex | openclaw | hermes | all
        #[arg(value_name = "TARGET")]
        target: String,
    },
    /// 检查原生 plugin、随包版本和内容完整性
    Status {
        #[arg(value_name = "TARGET")]
        target: String,
    },
    /// 重装当前 CLI 随附的 SKZ plugin
    Upgrade {
        #[arg(value_name = "TARGET")]
        target: String,
    },
    /// 卸载由 SKZ 管理的 plugin
    Uninstall {
        #[arg(value_name = "TARGET")]
        target: String,
    },
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
    /// 删除用户研究问题（写，物理删，不重试）：DELETE /research/problems/{code}
    Delete { code: String },
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
        #[arg(long, default_value_t = 5)]
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
        #[arg(long, default_value_t = 5)]
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
        #[arg(long = "page-size", default_value_t = 5)]
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
    /// 删路线 + 级联删名下挖掘执行（写，物理删，不重试）：DELETE /research/factor-routes/{code}
    Delete {
        code: String,
        /// 越过两条软护栏（名下仍有因子 / 执行目录最近有写入）
        #[arg(long)]
        force: bool,
        /// 只预告将删几次执行、将留几个孤儿因子，零修改
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum MiningCmd {
    /// 挖掘 run 列表（读）：GET /research/mining/runs
    Runs {
        #[arg(long)]
        route: Option<String>,
    },
    /// 物理删除单次已落盘挖掘结果（写，不重试）：DELETE /research/mining/runs/{run_id}
    #[command(name = "delete-run")]
    DeleteRun {
        run_id: String,
        /// 越过“目录最近仍有写入”的软护栏
        #[arg(long)]
        force: bool,
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
        #[arg(long = "page-size", default_value_t = 5)]
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
        #[arg(long = "page-size", default_value_t = 5)]
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
    /// 批量最新仓位（读）：GET /research/strategies/positions/latest?weight_type=ts|cs
    #[command(name = "latest-positions")]
    LatestPositions {
        /// ts = 时序策略逐标的最新权重；cs = 截面策略最新完整截面
        #[arg(long = "weight-type", value_parser = ["ts", "cs"])]
        weight_type: String,
    },
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
    /// 批量登记策略进实盘库（写，不重试）：POST /research/strategy-imports
    /// 给文件参数时一次上传 1–100 份 JSON/TOML；不传文件时从 stdin 读一份。
    /// JSON 走 `strategy definition <code>` 的输出形态，由 CLI 转成 TOML 再上传。
    /// ⚠️ 这不是研究流程的入口——它不跑回测，策略直接进实盘库（暂停态）且没有任何
    /// 样本内外指标。只用于克隆/迁移**已经验证过**的策略。
    Register {
        /// 待登记的 JSON/TOML 文件；不传则读取 stdin（单份定义）
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },
    /// 写用户笔记（写，不重试）：PATCH /research/strategies/{code}/memo
    /// 笔记内容从 stdin 读（长文本/换行不必在 shell 里转义）；清除已有笔记要显式 `--clear`。
    Memo {
        code: String,
        /// 清除已有笔记（写入空串）。给了这个就不读 stdin。
        #[arg(long)]
        clear: bool,
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
    // 独立动词，**不是**把上面那条的 `code` 改成可选：两者位置参数只差一个，
    // 合并的话 agent 少传一个 code 就从「删一个候选」静默升级成「删掉整次探索」，
    // 而且不可逆——正是那种「错误被伪装成合法结果」的形状。
    /// 删整次探索执行（写，物理删，不重试）：DELETE /research/experiments/{id}
    #[command(name = "delete-run")]
    DeleteRun {
        id: String,
        /// 越过「目录最近有写入」软护栏；对「实盘更新任务正在运行」无效（那条硬拒绝）
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum PromoteCmd {
    /// 保存入库（写，触 FC 实时结果预热，不重试；受理后消费候选产物）
    Start {
        id: String,
        code: String,
        /// 顺带写入用户笔记。⚠️ 后端**只在本次真的新插入时**写入：该策略若已在实盘库里
        /// （promote 复用已有记录），这个 memo 会被静默忽略、不报错。要改已入库策略的
        /// 笔记请用 `skz strategy memo <code>`。
        #[arg(long)]
        memo: Option<String>,
    },
    /// 轮询 promote 终态（读）：GET /research/promotions/{promotion_id}
    Get { promotion_id: String },
}

#[derive(Subcommand)]
enum GiftCmd {
    /// 发赠予码（写，不重试）：POST /research/gifts
    ///
    /// ⚠️ 码本身就是这些策略的访问凭证：拿到码的人不需要别的授权就能领走完整定义，
    /// 且**发出即不可撤回地披露**（`revoke` 只挡得住还没领的人）。
    Create {
        /// 要赠予的实盘库策略编号，可重复传，1～10 条（重复项会去重）
        #[arg(long = "strategy", value_name = "CODE")]
        strategy: Vec<String>,
        /// 允许领取的**去重人数**上限，1～100（同一人重复领取幂等，不多占名额）
        #[arg(long = "max-claims")]
        max_claims: u32,
        /// 有效期天数，仅 1 / 3 / 7
        #[arg(long = "ttl-days", default_value_t = 3)]
        ttl_days: u8,
    },
    /// 我发出的、尚未过期的码（读）：GET /research/gifts
    List,
    /// 撤回自己发出的码（写，不重试）：DELETE /research/gifts/{gift_code}
    Revoke { gift_code: String },
    /// 领取前预览（读，零副作用）：GET /research/gifts/{gift_code}/preview
    Preview { gift_code: String },
    /// 领取（写，不重试）：POST /research/gifts/{gift_code}/claim
    ///
    /// 副本落进自己的实盘库，状态固定「暂停」，不带 memo/tags；后续操作用返回的
    /// `strategy_code`（撞名会带 `_G{n}` 后缀），不是 `origin_strategy_code`。
    Claim { gift_code: String },
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
    /// 兼容旧版：从 stdin 覆盖 default 身份并设为默认
    Set,
    /// 添加命名身份；token 从 stdin 读取
    Add {
        identity: String,
        /// 账户归属；缺省与 identity 同名
        #[arg(long)]
        account: Option<String>,
        /// 本地强制只读，所有写/触发请求均不发送
        #[arg(
            long,
            conflicts_with = "allow_write",
            required_unless_present = "allow_write"
        )]
        read_only: bool,
        /// 允许 CLI 发写请求；实际权限仍以后端 key scope 为准
        #[arg(long, conflicts_with = "read_only")]
        allow_write: bool,
        /// 显式覆盖同名身份
        #[arg(long)]
        replace: bool,
    },
    /// 列出全部身份及当前默认身份（不打印 token）
    List,
    /// 设置机器级持久默认身份
    Use { identity: String },
    /// 删除命名身份；若它是默认身份则同时清空默认选择
    Remove { identity: String },
    /// 报告当前身份与最终只读状态（JSON，不打印 token）
    Status,
    /// 兼容旧版：只删除 default 身份
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
            // 契约版本单处定义，plugin receipt 与 --version 使用同一个值。
            &serde_json::json!({ "cli": env!("CARGO_PKG_VERSION"), "contract": plugin::CONTRACT }),
            false,
        );
        return Ok(());
    }

    let Cli {
        pretty, command, ..
    } = cli;
    let command = command.ok_or_else(|| Error::Args("缺少子命令；见 `skz --help`".to_string()))?;

    match command {
        Command::Plugin { action } => run_plugin(action, pretty),
        Command::Auth { action } => run_auth(action),
        Command::Markets => {
            let client = make_client()?;
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
            let client = make_client()?;
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
            let client = make_client()?;
            let data = retry::with_retry(|| {
                client.calendar(&exchange, start.as_deref(), end.as_deref(), only_open)
            })?;
            emit_value(&data, pretty);
            Ok(())
        }
        Command::Route { action } => run_route(action, pretty),
        Command::Problem { action } => run_problem(action, pretty),
        Command::Mine { action } => run_mine(action, pretty),
        Command::Explore { action } => run_explore(action, pretty),
        Command::Factor { action } => run_factor(action, pretty),
        Command::FactorRoutes { action } => run_factor_routes(action, pretty),
        Command::Mining { action } => run_mining(action, pretty),
        Command::Strategy { action } => run_strategy(action, pretty),
        Command::Experiment { action } => run_experiment(action, pretty),
        Command::Promote { action } => run_promote(action, pretty),
        Command::Gift { action } => run_gift(action, pretty),
        Command::Portfolio { action } => run_portfolio(action, pretty),
        Command::Whoami => {
            let client = make_client()?;
            let data = retry::with_retry(|| client.whoami())?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 零 HTTP 调用，不读取服务器配置——跟其它分支的样板代码不一样，别顺手抄过来。
        Command::Update => run_update(pretty),
    }
}

// ── 策略业务分派 ────────────────────────────────────────────────
// 约定：读命令（含 poll）走 `retry::with_retry`；写/触发命令**直接调用不重试**
// （无幂等保证、触发即扣费）。

fn run_route(action: RouteCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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

fn run_problem(action: ProblemCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        // 写：不重试
        ProblemCmd::Create => {
            let body = read_stdin_json()?;
            validate_problem_symbols(&body)?;
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
        // 写：不重试。超时后按 code 查询，确认问题是否仍存在。
        ProblemCmd::Delete { code } => {
            require_nonempty(&code, "code")?;
            let data = client
                .problem_delete(&code)
                .map_err(|e| e.into_write_unknown("skz problem get <code>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

// ── 研究面：策略实盘（读 + 实盘写）/ 实验 / promote ────────────

fn run_strategy(action: StrategyCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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
            validate_strategy_list_status(status.as_deref())?;
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
        StrategyCmd::LatestPositions { weight_type } => {
            let data = retry::with_retry(|| client.strategy_latest_positions(&weight_type))?;
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
            validate_trade_kind(kind.as_deref())?;
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
        StrategyCmd::Register { files } => {
            let tomls = read_strategy_tomls(&files)?;
            let data = client
                .strategy_register(&tomls)
                .map_err(|e| e.into_write_unknown("skz strategy list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        StrategyCmd::Memo { code, clear } => {
            require_nonempty(&code, "code")?;
            let memo = if clear {
                String::new()
            } else {
                read_stdin_memo()?
            };
            let data = client
                .strategy_memo(&code, &memo)
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

fn run_experiment(action: ExperimentCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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
        // 写：不重试。超时后用实验清单确认这次探索是否还在。
        ExperimentCmd::DeleteRun { id, force } => {
            require_nonempty(&id, "id")?;
            let data = client
                .experiment_delete_run(&id, force)
                .map_err(|e| e.into_write_unknown("skz experiment list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_promote(action: PromoteCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        // 写：不重试（触发即扣算力）
        PromoteCmd::Start { id, code, memo } => {
            require_nonempty(&id, "id")?;
            require_nonempty(&code, "code")?;
            let memo = memo.map(validate_memo_arg).transpose()?;
            let data = client
                .promote_start(&id, &code, memo.as_deref())
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

/// 赠予码形态：32 位小写十六进制。后端把形态不对的码一律当「不存在」（404，故意不区分
/// 「格式对但不存在」，免得成为探测口），到 CLI 这边是 exit 2；本地先拦一道只是把
/// 「手滑贴少了几位」和「码真的过期了」分开，省一次往返，不改变 action。
fn validate_gift_code(code: &str) -> Result<(), Error> {
    let ok = code.len() == 32
        && code
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if ok {
        Ok(())
    } else {
        Err(Error::Args(format!(
            "gift_code 必须是 32 位小写十六进制串，当前: {code:?}"
        )))
    }
}

fn run_gift(action: GiftCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        // 写：不重试。三个上限都是**固定枚举/值域**，不随账号和时间变，所以本地枚举、
        // 不为它发网络（见 CLAUDE.md「本地校验 vs 免费读预检」）。策略编号是否真的在
        // 实盘库里则交后端——那是动态值域，且后端会明确报 422，不是静默失败。
        GiftCmd::Create {
            strategy,
            max_claims,
            ttl_days,
        } => {
            let mut codes: Vec<String> = Vec::new();
            for raw in &strategy {
                let code = raw.trim();
                if code.is_empty() {
                    return Err(Error::Args("--strategy 不能是空串".to_string()));
                }
                if !codes.iter().any(|c| c == code) {
                    codes.push(code.to_string());
                }
            }
            if codes.is_empty() {
                return Err(Error::Args(
                    "至少用 --strategy 指定一条实盘库策略".to_string(),
                ));
            }
            if codes.len() > 10 {
                return Err(Error::Args(format!(
                    "一个赠予码最多打包 10 条策略，去重后当前 {} 条",
                    codes.len()
                )));
            }
            if max_claims == 0 || max_claims > 100 {
                return Err(Error::Args(
                    "--max-claims 须在 1～100 之间（按去重人数计）".to_string(),
                ));
            }
            if !matches!(ttl_days, 1 | 3 | 7) {
                return Err(Error::Args("--ttl-days 仅支持 1 / 3 / 7".to_string()));
            }
            let data = client
                .gift_create(&codes, max_claims, ttl_days)
                .map_err(|e| e.into_write_unknown("skz gift list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试
        GiftCmd::List => {
            let data = retry::with_retry(|| client.gift_list())?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试。超时后用 `gift list` 看这个码还在不在。
        GiftCmd::Revoke { gift_code } => {
            validate_gift_code(&gift_code)?;
            let data = client
                .gift_revoke(&gift_code)
                .map_err(|e| e.into_write_unknown("skz gift list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 读：重试。零副作用，领取前该先跑它。
        GiftCmd::Preview { gift_code } => {
            validate_gift_code(&gift_code)?;
            let data = retry::with_retry(|| client.gift_preview(&gift_code))?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试。后端对同一用户幂等（领过原样回放），但幂等写也不开重试的口子——
        // 规则的价值全在零例外（同 `strategy memo`）。超时后用 preview 的
        // `already_claimed` 查证，那比翻策略库准（撞名时编号会带 `_G{n}` 后缀）。
        GiftCmd::Claim { gift_code } => {
            validate_gift_code(&gift_code)?;
            let data = client
                .gift_claim(&gift_code)
                .map_err(|e| e.into_write_unknown("skz gift preview <gift_code>"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_portfolio(action: PortfolioCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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
            let (portfolio_code, candidate_strategies) = portfolio_create_identity(&body)?;
            preflight_portfolio_create(&client, &portfolio_code, &candidate_strategies)?;
            let data = client
                .portfolio_create(&body)
                .map_err(|e| e.into_write_unknown("skz portfolio list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_mine(action: MineCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        // 写/触发：不重试
        MineCmd::Start { route } => {
            if route.trim().is_empty() {
                return Err(Error::Args("route 不得为空".to_string()));
            }
            preflight_route(&client, &route)?;
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

fn run_explore(action: ExploreCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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
            preflight_route(&client, &route)?;
            retry::with_retry(|| client.problem_get(&problem))?;
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

fn run_factor(action: FactorCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
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

fn run_factor_routes(action: FactorRoutesCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        FactorRoutesCmd::List => {
            let data = retry::with_retry(|| client.factor_routes())?;
            emit_value(&data, pretty);
            Ok(())
        }
        // 写：不重试（`--dry-run` 也不重试——它虽零修改，但走的是 DELETE，
        // 不给它开重试口子，省得以后有人照着这条援引「幂等写可以重试」）。
        // 超时后用路线清单确认这条 code 是否还在；`failed_mining_runs` 非空时重发即续删。
        FactorRoutesCmd::Delete {
            code,
            force,
            dry_run,
        } => {
            require_nonempty(&code, "code")?;
            let data = client
                .factor_route_delete(&code, force, dry_run)
                .map_err(|e| e.into_write_unknown("skz factor-routes list"))?;
            emit_value(&data, pretty);
            Ok(())
        }
    }
}

fn run_mining(action: MiningCmd, pretty: bool) -> Result<(), Error> {
    let client = make_client()?;
    match action {
        MiningCmd::Runs { route } => {
            let data = retry::with_retry(|| client.mining_runs(route.as_deref()))?;
            emit_value(&data, pretty);
            Ok(())
        }
        MiningCmd::DeleteRun { run_id, force } => {
            require_nonempty(&run_id, "run_id")?;
            let data = client
                .mining_delete_run(&run_id, force)
                .map_err(|e| e.into_write_unknown("skz mining runs"))?;
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
            validate_page_size_max(page, page_size, 100)?;
            if let Some(group) = group.as_deref() {
                preflight_mining_group(&client, &run_id, group)?;
            }
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
    let value = match action {
        AuthCmd::Set => serde_json::to_value(credentials::set_from_stdin()?)
            .map_err(|e| Error::Internal(format!("序列化 auth 结果失败: {e}")))?,
        AuthCmd::Add {
            identity,
            account,
            read_only,
            allow_write: _,
            replace,
        } => {
            let policy = if read_only {
                credentials::WritePolicy::Deny
            } else {
                credentials::WritePolicy::Allow
            };
            serde_json::to_value(credentials::add_from_stdin(
                &identity,
                account.as_deref(),
                policy,
                replace,
            )?)
            .map_err(|e| Error::Internal(format!("序列化 auth 结果失败: {e}")))?
        }
        AuthCmd::List => serde_json::to_value(credentials::list()?)
            .map_err(|e| Error::Internal(format!("序列化 auth 结果失败: {e}")))?,
        AuthCmd::Use { identity } => {
            let selected = credentials::use_identity(&identity)?;
            serde_json::json!({
                "active": selected.name,
                "account": selected.account,
                "writePolicy": selected.write_policy,
                "persistent": true,
            })
        }
        AuthCmd::Status => {
            // `readOnly` 放这里而不是 `--version`：`--version` 描述的是**这个二进制**
            // （`update.rs` 还拿它做升级自检），而只读是**这台机器当下**的策略，属于
            // "我现在能干什么"，跟凭据同一个问题。
            //
            // 这个字段是最终只读状态的唯一验证手段，别删：它同时合并当前身份策略和
            // env 开关；后者变量名打错会静默变成"没设"，必须配完跑一次这条亲眼确认。
            let global_read_only = config::read_only_from_env()?;
            serde_json::to_value(credentials::status(global_read_only)?)
                .map_err(|e| Error::Internal(format!("序列化 auth 结果失败: {e}")))?
        }
        AuthCmd::Remove { identity } => credentials::remove(&identity)?,
        AuthCmd::Unset => credentials::unset()?,
    };
    emit_value(&value, false);
    Ok(())
}

/// Plugin 生命周期分派：每次只写一份紧凑 JSON。
fn run_plugin(action: PluginCmd, pretty: bool) -> Result<(), Error> {
    match action {
        PluginCmd::Install { target } => {
            emit_multi(
                &parse_targets(&target)?
                    .into_iter()
                    .map(plugin::install)
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        PluginCmd::Status { target } => {
            emit_multi(
                &parse_targets(&target)?
                    .into_iter()
                    .map(plugin::status)
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        PluginCmd::Upgrade { target } => {
            emit_multi(
                &parse_targets(&target)?
                    .into_iter()
                    .map(plugin::upgrade)
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
            Ok(())
        }
        PluginCmd::Uninstall { target } => {
            emit_multi(
                &parse_targets(&target)?
                    .into_iter()
                    .map(plugin::uninstall)
                    .collect::<Result<Vec<_>, _>>()?,
                pretty,
            );
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
    /// 否则当前进程同级的 bundle 仍然权威（`refresh_in_process`）。
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
    let post_upgrade_exe = update::post_upgrade_exe(channel, &exe);

    let outcome = match channel {
        update::Channel::Unknown => UpdateOutcome {
            attempted: false,
            updated: Some(false),
            cli_after: None,
            ref_cli: env!("CARGO_PKG_VERSION").to_string(),
            ref_contract: plugin::CONTRACT.to_string(),
            delegate_refresh: false,
            remediation: Some(unknown_channel_remediation()),
        },
        _ => {
            update::upgrade(channel)?;
            match update::probe_version(&post_upgrade_exe) {
                Some(v) if v.cli != env!("CARGO_PKG_VERSION") || v.contract != plugin::CONTRACT => {
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
                    ref_contract: plugin::CONTRACT.to_string(),
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

    let plugins = build_plugins_report(&outcome, &post_upgrade_exe)?;

    emit_value(
        &update::UpdateReport {
            channel: channel.as_str(),
            attempted: outcome.attempted,
            updated: outcome.updated,
            cli: env!("CARGO_PKG_VERSION"),
            cli_after: outcome.cli_after,
            remediation: outcome.remediation,
            plugins,
        },
        pretty,
    );
    Ok(())
}

/// 组装技能新鲜度小节，真终端场景下顺带问人要不要刷新。`current_exe()` 已经在
/// `run_update` 里探测过；这里剩下唯二"不纯"的输入是 stdin/stderr 的 TTY 探测——
/// 天然只属于"直接终端调用"这个场景，留在 bin 这层，不下沉进 lib。
fn build_plugins_report(
    outcome: &UpdateOutcome,
    exe: &Path,
) -> Result<update::PluginsReport, Error> {
    let targets = plugin::present_targets();
    let checked_targets: Vec<&'static str> = targets.iter().map(|t| t.as_str()).collect();

    if outcome.updated.is_none() {
        // 升级子进程成功了，但确认不了磁盘上的新版本号——没有可信的比对基准，
        // 宁可不评估，也不要拿一个可能错的基准假装评估过。
        return Ok(update::PluginsReport {
            checked_targets,
            evaluated: false,
            skip_reason: Some(
                "升级后无法确认磁盘上的新版本号（--version 自检失败），跳过技能新鲜度核对；\
                 重跑 `skz update` 或手动跑 `skz plugin status <target>` 确认"
                    .to_string(),
            ),
            stale: vec![],
            refresh_offered: false,
            refresh_accepted: None,
            refreshed: None,
        });
    }

    let marked = update::installed_plugins(&targets)?;
    let stale = update::find_stale(&marked, &outcome.ref_cli, &outcome.ref_contract);

    let mut refresh_offered = false;
    let mut refresh_accepted = None;
    let mut refreshed = None;

    if !stale.is_empty() && std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        refresh_offered = true;
        if prompt_refresh(&stale) {
            refresh_accepted = Some(true);
            let mut stale_targets: Vec<plugin::Target> = Vec::new();
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

    Ok(update::PluginsReport {
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
fn prompt_refresh(stale: &[update::StalePlugin]) -> bool {
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

/// 识别不出安装渠道时的兜底：指回 README 的四种公开安装渠道。
fn unknown_channel_remediation() -> serde_json::Value {
    serde_json::json!({
        "howTo": "本机没能从当前 skz 路径识别出受支持的 Homebrew 或 Scoop 安装，跳过自更新。可用对应平台包管理器重新安装：",
        "commands": [
            "brew install sheng-ke-zhi/tap/skz",
            "scoop bucket add skz https://github.com/sheng-ke-zhi/scoop-bucket",
            "scoop install skz"
        ]
    })
}

fn parse_target(s: &str) -> Result<plugin::Target, Error> {
    match s {
        "claude" => Ok(plugin::Target::Claude),
        "codex" => Ok(plugin::Target::Codex),
        "openclaw" => Ok(plugin::Target::Openclaw),
        "hermes" => Ok(plugin::Target::Hermes),
        other => Err(Error::Args(format!(
            "未知 target {other}；可选 claude | codex | openclaw | hermes | all"
        ))),
    }
}

/// `all` 只处理 PATH 中能找到原生 CLI 的 harness。
fn parse_targets(s: &str) -> Result<Vec<plugin::Target>, Error> {
    if s != "all" {
        return Ok(vec![parse_target(s)?]);
    }
    let found = plugin::present_targets();
    if found.is_empty() {
        return Err(Error::Args(
            "PATH 中没发现 claude、codex、openclaw 或 hermes；请先安装对应 harness".to_string(),
        ));
    }
    Ok(found)
}

fn make_client() -> Result<Client, Error> {
    let mut cfg = Config::new()?;
    let selected = credentials::load_selected()?;
    cfg.read_only = cfg.read_only || selected.write_policy.is_read_only();
    Ok(Client::new(&cfg, selected.token))
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

/// 策略列表后端会把错 status 当成“无匹配”，本地拦下以免把参数错当成空仓库。
fn validate_strategy_list_status(status: Option<&str>) -> Result<(), Error> {
    match status {
        None | Some("实盘" | "暂停" | "废弃") => Ok(()),
        Some(_) => Err(Error::Args(
            "status 仅接受 实盘|暂停|废弃；无效值会被后端静默当成空结果".to_string(),
        )),
    }
}

/// trades 后端会静默忽略错 kind，导致调用方误以为拿到的是筛选结果。
fn validate_trade_kind(kind: Option<&str>) -> Result<(), Error> {
    match kind {
        None | Some("win" | "loss" | "all") => Ok(()),
        Some(_) => Err(Error::Args(
            "kind 仅接受 win|loss|all；无效值会被后端静默忽略".to_string(),
        )),
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

/// route 触发端点不查存在性，错 code 也会受理付费任务；先用免费资产读确认。
fn preflight_route(client: &Client, route: &str) -> Result<(), Error> {
    // 只读模式早退：闸的不变量在 client 传输层，这里纯粹是免得白跑一趟预检读。
    client.ensure_writable()?;
    let routes = retry::with_retry(|| client.factor_routes())?;
    if routes.items.iter().any(|item| item.code == route) {
        Ok(())
    } else {
        Err(Error::Args(format!(
            "route 不存在: {route}；请先用 `skz factor-routes list` 获取有效 code"
        )))
    }
}

/// portfolio 写端点只查候选非空且会复用既有 code；把会浪费算力的情况挡在 POST 前。
fn preflight_portfolio_create(
    client: &Client,
    portfolio_code: &str,
    candidate_strategies: &[String],
) -> Result<(), Error> {
    // 同 preflight_route：早退只为省下这里最多 1+N 次翻页读，不是闸本身。
    client.ensure_writable()?;
    let portfolios = retry::with_retry(|| client.portfolio_list())?;
    if portfolios
        .items
        .iter()
        .any(|item| item.code == portfolio_code)
    {
        return Err(Error::Args(format!(
            "portfolio_code 已存在: {portfolio_code}；请使用新 code，避免重新触发并覆盖既有组合"
        )));
    }

    let mut live_codes = Vec::new();
    let mut page = 1u32;
    loop {
        let query = vec![
            ("status", "实盘".to_string()),
            ("page", page.to_string()),
            ("page_size", "1000".to_string()),
        ];
        let live = retry::with_retry(|| client.strategy_list(&query))?;
        let received = live.items.len();
        let total = live.total.max(0) as usize;
        live_codes.extend(live.items.into_iter().map(|item| item.code));
        if received == 0 || live_codes.len() >= total {
            break;
        }
        page += 1;
    }
    let invalid: Vec<&str> = candidate_strategies
        .iter()
        .filter(|code| !live_codes.iter().any(|live| live == *code))
        .map(String::as_str)
        .collect();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(Error::Args(format!(
            "candidate_strategies 中存在无效或非实盘策略: {}；请先用 `skz strategy list --status 实盘` 核对",
            serde_json::to_string(&invalid).expect("字符串数组序列化不会失败")
        )))
    }
}

/// group 的合法值随 run 变化，直接取 overview.problem_groups[].prefix 动态校验。
fn preflight_mining_group(client: &Client, run_id: &str, group: &str) -> Result<(), Error> {
    require_nonempty(group, "group")?;
    let overview = retry::with_retry(|| client.mining_overview(run_id))?;
    if overview
        .problem_groups
        .iter()
        .any(|item| item.prefix == group)
    {
        return Ok(());
    }
    let valid: Vec<&str> = overview
        .problem_groups
        .iter()
        .map(|item| item.prefix.as_str())
        .collect();
    Err(Error::Args(format!(
        "group 对当前 run 无效: {group}；可选值: {}",
        serde_json::to_string(&valid).expect("字符串数组序列化不会失败")
    )))
}

/// 从 stdin 读一份 JSON body（创建类写命令的复杂输入）。
/// 只本地校验“是合法 JSON object”——非法/空/非对象在发网络前失败（exit 2）；
/// 各命令需要提前拦截的静默失败由调用方继续校验，其余字段交后端（400 → fix_params）。
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

/// 单个策略 TOML 的字节上限，跟后端 `MAX_TOML_BYTES` 对齐。**这个是字节不是字符**
/// （后端判的是 `toml.len()`），跟 memo 的字符上限不是一回事，别互相抄。
const STRATEGY_TOML_MAX_BYTES: usize = 1024 * 1024;
/// 单批策略数与解码后 TOML 总字节上限，跟后端批量导入边界对齐。
const STRATEGY_BATCH_MAX_ITEMS: usize = 100;
const STRATEGY_BATCH_MAX_TOML_BYTES: usize = 10 * 1024 * 1024;

/// 后端 `parse_strategy_toml_with_meta` 要求的顶层键，缺一个就 42201。
/// 本地先拦，省掉一次注定失败的往返；同时给出比后端更具体的「缺哪个」。
const STRATEGY_TOML_REQUIRED: &[&str] = &[
    "strategy",
    "problem",
    "runtime",
    "model_config",
    "post_process",
    "route",
    "factors",
];

/// TOML 表示不了 null，而 `strategy definition` 的输出里就有（实测 `problem.suffix`）。
/// 递归剥掉空值——不剥的话 `toml::to_string` 直接报 "unsupported unit type"。
/// 「键不存在」和「键为 null」对后端是同一件事（它只读它认识的那几个键），所以这个
/// 丢弃是安全的；但它是一次静默转换，help 与技能文档里都写明了。
fn strip_nulls(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => serde_json::Value::Object(
            m.iter()
                .filter(|(_, x)| !x.is_null())
                .map(|(k, x)| (k.clone(), strip_nulls(x)))
                .collect(),
        ),
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(strip_nulls).collect())
        }
        other => other.clone(),
    }
}

/// 解析一份策略定义，**JSON 与 TOML 双模**，统一产出 TOML 文本。
///
/// 嗅探而不是加 `--format` 开关：agent 的正常路径是 `strategy definition | jq | register`
/// （JSON），人的正常路径是 `register x.toml`，两边都不该被迫记住一个格式参数。
/// 先试 JSON——TOML 的裸键语法几乎不可能被 `serde_json` 误判成合法 JSON，反向则不然
/// （`{...}` 在 TOML 里不是合法顶层文档），所以这个顺序不会误判。
fn parse_strategy_toml(buf: String, source: &str) -> Result<String, Error> {
    if buf.trim().is_empty() {
        return Err(Error::Args(format!(
            "{source} 为空：需要一份 JSON 或 TOML 策略定义"
        )));
    }

    let text = match serde_json::from_str::<serde_json::Value>(&buf) {
        Ok(v) => {
            let obj = v.as_object().ok_or_else(|| {
                Error::Args(format!(
                    "{source} JSON 必须是一个对象 {{…}}（strategy definition 的输出形态）"
                ))
            })?;
            check_required_keys(|k| obj.contains_key(k))?;
            toml::to_string(&strip_nulls(&v))
                .map_err(|e| Error::Args(format!("{source} JSON 转 TOML 失败: {e}")))?
        }
        Err(_) => {
            let parsed: toml::Value = toml::from_str(&buf).map_err(|e| {
                Error::Args(format!("{source} 既不是合法 JSON 也不是合法 TOML: {e}"))
            })?;
            let table = parsed
                .as_table()
                .ok_or_else(|| Error::Args(format!("{source} 的策略 TOML 顶层必须是一张表")))?;
            check_required_keys(|k| table.contains_key(k))?;
            buf
        }
    };

    // 长度按**转换后**的字节数判：后端收到的是这份 TOML，不是原始 JSON。
    if text.len() > STRATEGY_TOML_MAX_BYTES {
        return Err(Error::Args(format!(
            "策略 TOML 不能超过 1 MiB（转换后 {} 字节）",
            text.len()
        )));
    }
    Ok(text)
}

/// 有文件参数时批量读取；无参数时保留管道友好的单份 stdin 输入。
fn read_strategy_tomls(files: &[PathBuf]) -> Result<Vec<String>, Error> {
    if files.len() > STRATEGY_BATCH_MAX_ITEMS {
        return Err(Error::Args(format!(
            "strategy register 每批最多读取 {STRATEGY_BATCH_MAX_ITEMS} 个文件，当前 {} 个",
            files.len()
        )));
    }

    let mut inputs = Vec::with_capacity(files.len().max(1));
    if files.is_empty() {
        use std::io::Read as _;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| Error::Internal(format!("读取 stdin 失败: {e}")))?;
        inputs.push(("stdin".to_string(), buf));
    } else {
        for path in files {
            let buf = std::fs::read_to_string(path)
                .map_err(|e| Error::Args(format!("读取策略文件 {} 失败: {e}", path.display())))?;
            inputs.push((format!("策略文件 {}", path.display()), buf));
        }
    }

    let mut total_bytes = 0usize;
    let mut tomls = Vec::with_capacity(inputs.len());
    for (source, input) in inputs {
        let toml = parse_strategy_toml(input, &source)?;
        total_bytes += toml.len();
        if total_bytes > STRATEGY_BATCH_MAX_TOML_BYTES {
            return Err(Error::Args(format!(
                "每批策略 TOML 总大小不能超过 10 MiB（转换后 {total_bytes} 字节）"
            )));
        }
        tomls.push(toml);
    }
    Ok(tomls)
}

fn check_required_keys(has: impl Fn(&str) -> bool) -> Result<(), Error> {
    let missing: Vec<&str> = STRATEGY_TOML_REQUIRED
        .iter()
        .copied()
        .filter(|k| !has(k))
        .collect();
    if !missing.is_empty() {
        return Err(Error::Args(format!(
            "策略定义缺少必需字段: {}（完整字段见 `skz strategy definition <已有策略>` 的输出）",
            missing.join(", ")
        )));
    }
    Ok(())
}

/// memo 长度上限，跟后端 `MAX_MEMO_CHARS` 对齐。**按 Unicode 字符计，不是字节**——
/// 一段中文笔记的字节数是字符数的 3 倍，用 `len()` 会在远未超限时误拦。
const MEMO_MAX_CHARS: usize = 10_000;

/// trim 后做长度校验。后端也是先 trim 再数长度，这里照抄同一顺序——否则「首尾一堆空白
/// 正好顶到上限」这种边界上两边判定会不一致：本地拦下了，后端其实收得下。
/// `empty_hint` 是空输入时的提示语，两个调用点的补救动作不同（一个指向 `--clear`，
/// 一个指向"别传这个 flag"），所以由调用方给。
fn normalize_memo(raw: &str, empty_hint: &str) -> Result<String, Error> {
    let memo = raw.trim();
    if memo.is_empty() {
        return Err(Error::Args(empty_hint.to_string()));
    }
    let chars = memo.chars().count();
    if chars > MEMO_MAX_CHARS {
        return Err(Error::Args(format!(
            "memo 最多 {MEMO_MAX_CHARS} 个字符（按 Unicode 字符计，非字节），当前 {chars} 个"
        )));
    }
    Ok(memo.to_string())
}

/// 从 stdin 读一段纯文本笔记（不是 JSON，笔记本身可能就带引号和换行）。
/// **空输入报错而不是当成"清除"**：memo 是覆盖写，一个手滑的空管道（上游命令没输出、
/// 或 `< /dev/null`）会静默抹掉已有笔记且不可恢复；要清除就显式 `--clear`。
fn read_stdin_memo() -> Result<String, Error> {
    use std::io::Read as _;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::Internal(format!("读取 stdin 失败: {e}")))?;
    normalize_memo(
        &buf,
        "stdin 为空：memo 内容需从 stdin 传入；要清除已有笔记请显式加 --clear",
    )
}

/// 校验 `promote start --memo` 的取值。空白值直接报错而不是发一个空 memo 上去——
/// 后端会当没传处理，agent 却以为写成功了，静默无操作比报错难查得多。
fn validate_memo_arg(raw: String) -> Result<String, Error> {
    normalize_memo(&raw, "--memo 不能是空白；不需要写笔记就别传这个参数")
}

/// 提取组合预检需要的两个字段，同时把后端只会晚报的结构错误提前成 fix_params。
fn portfolio_create_identity(body: &serde_json::Value) -> Result<(String, Vec<String>), Error> {
    let portfolio_code = body
        .get("portfolio_code")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty() && *value == value.trim())
        .ok_or_else(|| Error::Args("portfolio_code 必须是非空且无首尾空格的字符串".to_string()))?;
    let candidates = body
        .get("candidate_strategies")
        .and_then(serde_json::Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| Error::Args("candidate_strategies 必须是非空字符串数组".to_string()))?;
    let candidates = candidates
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|code| !code.trim().is_empty() && *code == code.trim())
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::Args(
                        "candidate_strategies 必须是非空且无首尾空格的字符串数组".to_string(),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((portfolio_code.to_string(), candidates))
}

/// 股票、ETF、期货 problem 的后端会接受裸代码却无法给出明确反馈；
/// 在发请求前要求这些数据集使用市场标准 symbol。
fn validate_problem_symbols(body: &serde_json::Value) -> Result<(), Error> {
    let validate_suffix = body
        .get("dataset")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|dataset| matches!(dataset, "stock" | "etf" | "future"));
    if !validate_suffix {
        return Ok(());
    }

    let Some(symbols) = body.get("symbols") else {
        return Ok(());
    };
    let symbols = symbols
        .as_array()
        .ok_or_else(|| Error::Args("symbols 必须是字符串数组".to_string()))?;

    let mut invalid = Vec::new();
    for symbol in symbols {
        let Some(symbol) = symbol.as_str() else {
            return Err(Error::Args("symbols 必须是字符串数组".to_string()));
        };
        let trimmed = symbol.trim();
        let qualified = symbol == trimmed
            && trimmed
                .rsplit_once('.')
                .is_some_and(|(code, suffix)| !code.is_empty() && !suffix.is_empty());
        if !qualified {
            invalid.push(symbol);
        }
    }

    if invalid.is_empty() {
        Ok(())
    } else {
        Err(Error::Args(format!(
            "symbols 必须包含市场后缀（如 000001.SZ）；无效值: {}。可先用 `skz symbols --keyword <代码>` 查询标准 symbol",
            serde_json::to_string(&invalid).expect("字符串数组序列化不会失败")
        )))
    }
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
/// 只有 target `all` 命中多家时才出数组。仍是单次 stdout 写。
fn emit_multi<T: serde::Serialize>(reports: &[T], pretty: bool) {
    match reports {
        [one] => emit_value(one, pretty),
        many => emit_value(&many, pretty),
    }
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
