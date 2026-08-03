# CLAUDE.md — skz-quant-cli

面向 AI agent 的胜可知(Shengkezhi)开放平台执行器。Rust CLI,二进制名 `skz`。
`lib.rs` 是可复用的 client library,`bin/skz.rs` 只是它的一个入口(未来 MCP server 可直接复用 lib)。
主要能力:**市场数据只读查询** + **量化研究流程** + **因子/策略/组合资产管理(含写/触发)**。edition 2024,MSRV 跟随 stable(当前 `1.97.1`),I/O 契约版本 `2.7`。

**MSRV 策略:不压 MSRV。** 官方只发布预编译产物(PyPI wheel / GitHub Release 二进制);公开源码可供开发和自行构建,但不承诺兼容旧 rustc。压 MSRV 换不到官方分发兼容性、只会反过来钉住依赖(历史上 `ureq` 为守 1.80 被钉在 `~3.2`)。升级 stable 后直接把 `rust-version` 抬上去。

## 构建 / 测试 / 运行

⚠️ **profile 是拆开的,别搞混**(见 `Cargo.toml`):

- `cargo build` / `cargo test` —— 日常开发(debug,快)。
- `cargo build --release` —— 本地冒烟。**快,但不是出货二进制**(opt=2、无 lto、不 strip)。
- `cargo build --profile dist` —— 出货二进制(lto + opt=z + strip + panic=abort;慢、吃内存,由本地一键发布脚本调用)。**别用 `--release` 发布。**
- `cargo clippy` / `cargo fmt` —— lint / 格式化。

测试:`cargo test`;集成测试在 `tests/cli.rs`,用 `assert_cmd` 拉起二进制、`httpmock` 起 loopback mock。

本地跨平台构建使用纯 Python 入口，产物统一写到已 gitignore 的 `release-dist/`：

- WSL：`python3 scripts/release/build_wsl.py`，用 Zig 交叉构建 macOS arm64/x64 和 Linux arm64 musl，并构建 Linux x64 musl、Windows x64 GNU。依赖 `musl-tools`、`gcc-mingw-w64-x86-64`、`cargo-zigbuild` 和 Zig（本机可用 `uv tool install ziglang` 提供的 `python-zig`）。macOS 链接会提示找不到 Xcode SDK，但本项目不依赖 Apple framework，Zig 自带的系统库定义可完成 Mach-O 链接；正式产物仍需在真实 Mac 冒烟。
- macOS：`python3 scripts/release/build_macos.py`，构建 macOS arm64/x64。
- 两个入口共用 `scripts/release/build_target.py`，同时生成二进制、wheel 和带 SHA256/version/commit/dirty 状态的 host manifest；`--target` 可只重跑单个平台且会合并同 commit 的既有 manifest。
- **仅维护者使用**的 WSL 一键发布入口：`python3 scripts/release/release_wsl.py`。它先聚合检查 `main`、干净工作树、远端同步、WSL 工具链、GitHub 登录和 PyPI token，然后完成 PATCH bump、测试、五平台构建、Python 标准库归档、SHA256 校验、push、PyPI/GitHub Release/Homebrew/Scoop 发布与远端核验。`--check-only` 只做预检和 bump dry-run，不修改工作树、不 bump、不 push、不发布；bump 后失败用 `--resume` 继续同一版本。
- 发布属于维护者操作；普通贡献者不得运行，自动化代理只有在维护者明确要求发布时才可执行。当前发布只认上述 Python 入口，不使用 GitHub Actions 或 Bash 脚本。PyPI 已有 wheel 会跳过，GitHub Release assets 会覆盖，Formula/manifest 无变化会跳过提交；禁止 force push。

## PyPI 分发(`pypi/build_wheel.py`)

纯 Rust 二进制没有 Python 代码,走"手搓 wheel 塞二进制"的路子:`{pkg}.data/scripts/skz` 下的文件会被 pip 原样拷到 venv `bin/` 并保留可执行位(ruff/uv 同款机制),不用 maturin/setuptools。版本号单一来源 = `Cargo.toml`；打包脚本直接读取同一字段，`cz bump` 也经 `.cz.toml` 的 cargo provider 修改它。

- `python3 pypi/build_wheel.py [--target TRIPLE ...]` —— 默认构建全部 5 个 target(macOS x86_64/arm64、Windows x64、Linux x64/arm64 musl 静态)的 wheel 到 `dist/`;`--target` 可重复传,只挑着构建(`dist` profile 是 lto+opt=z,全打一遍慢)。
- WSL 一键发布通过 `build_target.py` 构建五个平台；本地仍可按 `build_wheel.py` docstring 单独配置交叉工具链。链接器配置不写进共享 `.cargo/config.toml`。
- Linux 两个 wheel 打复合 tag(`manylinux_2_17_{arch}.musllinux_1_1_{arch}`):musl target 默认 `+crt-static`,产物零动态 libc 依赖,两边兼容性承诺都诚实满足,不用分别编译两份。
- `build_wheel.py` 只负责构建和封装。发布脚本从权限为 `0600` 的 `.release.env` 读取 `UV_PUBLISH_TOKEN`，token 只注入 `uv publish` 子进程环境。

## Homebrew / Scoop 分发

公开主仓库 `sheng-ke-zhi/skz-quant-cli` 的 Release 是 Homebrew/Scoop 唯一下载源，不再维护二进制镜像。`release_wsl.py` 生成 4 个 tarball、1 个 zip、5 个 wheels 和 `SHA256SUMS`；`update_package_managers.py` 直接用其中的归档哈希渲染 `Formula/skz.rb` 与 `bucket/skz.json`，URL 指向主仓库同版本 Release。推送走临时 clone + commit/push，失败会让发布脚本非零退出并要求用 `--resume` 继续核验。

## 架构

- `client.rs` —— ureq(blocking + rustls)客户端;端点预定义;GET=读,POST=写/触发。自身不读文件/env,构造时注入 `base_url` + `token`。
- `config.rs` —— 默认 API 地址(`https://api.shengkezhi.com/open/v1`)+ 超时配置，创建联网客户端时读取。
- `credentials.rs` —— token 唯一来源 = 凭据文件;Unix(含 macOS)统一 `~/.config/skz`(macOS 手动覆盖 `directories` 默认给的 Apple Application Support,图终端用户跨 mac/linux 机器心智一致),Windows 走 `directories` 解析的 LocalAppData;Unix `0600` 原子写(temp + rename);`auth set/status/unset`。
- `token.rs` —— `Token` 类型,`Debug` 打码成 `Token(***)`,只在注入 Authorization header 时 `expose()`。
- `error.rs` —— `Error` 枚举 + 动作导向退出码 + JSON `ErrorBody` + API 错误分类。
- `retry.rs` —— 有限重试(≤3 次),`Retry-After` 优先否则带抖动指数退避;**仅当 `action == RetryLater` 才重试**。
- `models/` —— serde 类型:`common`(Page)、`experiment`、`factor`、`live`、`market`、`mining`、`portfolio`、`problem`、`research`、`strategy`。

## 面向 agent 的 I/O 契约(核心)

- 成功 → **stdout 一份紧凑 JSON + 换行 + exit 0**。`--pretty` 只美化、不改字段结构(给人排障,agent 勿依赖)。
- 失败 → **stderr `{"error":{kind,action,message,status?,code?,retryable?,retryAfterMs?,remediation?}}`** + 退出码。
- **退出码 = Action,1:1**。agent 控制流只 branch 在退出码(粗:下一步动作),细节读 JSON body:

| 码 | action | 含义 / agent 下一步 |
|---|---|---|
| 0 | — | 成功 |
| 2 | `fix_params` | 参数错(含 clap 解析失败)→ 改参数 |
| 3 | `fix_auth` | 缺/无效凭据 → `skz auth set` |
| 4 | `give_up` | 配额超限 / 多 IP / 402 余额不足 → 当天弃(402 带充值 remediation) |
| 5 | `retry_later` | 限流 / 5xx / 临时网络 → 稍后重试 |
| 6 | `internal` | 解析失败 / 未知码 / panic |
| 7 | `check_existing` | **别重发,先查现有状态**:① HTTP 409 已在跑 → 去对应 `runs --status active` 找在跑的 run 轮询;② **写的传输层错误(超时/连接失败)→ 结果未知**,照 `remediation.verifyWith` 查证 |

- **单次 stdout 写**:每次进程只有一处写 stdout(`emit_value`),别加第二个 writer。
- panic → JSON internal + exit 6(自定义 hook,只带位置、不带 payload)。

## 不可破坏的不变量

1. **读重试,写/触发绝不重试。** `retry::with_retry` 只包读命令(含幂等的 `poll`);`create` / `trigger`(`start`)直调 client **不重试**——触发即扣费、无幂等保证。加新命令时,写操作**不要**套 `with_retry`。
   - **写的传输层错误要标「结果未知」**:写命令统一 `.map_err(|e| e.into_write_unknown("<查证用的读命令>"))`,产出 `Error::WriteNetwork` → **`check_existing` / exit 7**(不是 5)。理由:契约要求 agent「照 action 分支」,而 `retry_later` 的字面意思就是重发,写超时恰恰不能重发;`check_existing` 的既有语义「别重触发、先查现有状态」与之同构。**曾用 `retry_later` + `retryable:false`,真机评测指出两个机读字段自相矛盾**——机读字段之间不能打架,别靠 prose remediation 兜底。加新写命令时一并挂上 `verify_with`。
   - `with_retry` 另有一道防御:`action==RetryLater` 且 `retryable != Some(false)` 才重试,防止将来误把写套进去。
   - **付费写可先做免费预检读**：预检读照常允许重试，全部通过后才执行一次不重试的写。预检阶段网络失败说明写尚未发生，返回 `retry_later`；真正进入写后的传输错误才是 `check_existing`。
   - **幂等写也不开口子。** `strategy memo` 是免费的幂等覆盖写、重发完全无害,语义上更像 `retry_later`,但**照样不重试**。这条规则的全部价值在于零例外:一旦承认"幂等写可以重试",此后每加一个写命令都要先判它属于哪边,而判错的代价是重复扣费——为省一次 memo 重试不值得。加新写命令时不要援引 memo 来论证例外。
2. **Token 永不泄露。** 别 log token;别给 `Token` 加会打印内容的 `Debug`/`Display`;取用只经 `expose()`。
3. **凭据不读环境变量。** token 只来自凭据文件。新增其他凭据来源前必须先明确其配置优先级与安全边界。
4. **先校验,再执行目标请求。** page/size、日期、run-id 数量(≤100)、固定枚举、stdin 结构、`problem create` 在 `stock`/`etf`/`future` 数据集下的 symbol 后缀先本地校验；`mine/explore start` 的资产 code、`portfolio create` 的 code 冲突与实盘候选、`mining factors --group` 再通过免费读动态预检。失败均为 exit 2，且目标请求不会发出；其余字段级合法性交后端(400 → `fix_params`)。
   - **判据是「后端对这个参数会静默失败」**,不是「这个参数看着重要」。三种静默失败都实测过:① **静默回空**(`strategy list --status` 传错 → `items:[]`,和真空仓库长得一模一样);② **静默忽略**(`strategy trades --kind` 传错 → 照样回全量,调用方以为筛过了);③ **受理后异步失败**(`mine/explore start` 传不存在的 route/problem → exit 0 起一个 run,7 秒后才 `ok:false`,**而这是花钱的接口**)。共性是**错误被伪装成一个合法结果**,agent 会照着错结论一路走下去。后端明确报 400/422 的字段不在此列——那本来就能正确分支到 `fix_params`,再加一道本地校验只会把值域钉死在 CLI 里。
   - **本地校验 vs 免费读预检,看值域会不会变**:值域固定且不随后端变(枚举、page/size 上限、symbol 后缀形态)→ 本地枚举,别为它发网络;值域是平台资产、随账号和时间变(route/problem code、`portfolio_code` 冲突、`mining factors --group` 的 `problem_groups[].prefix`)→ 免费读预检,**别把动态值域硬编码进 CLI**。
   - **预检翻页的退出条件不能只信后端 `total`。** `preflight_portfolio_create` 用 `received == 0 || live_codes.len() >= total` 双保险:`total` 报大时靠「这一页空了」兜底,否则会一直翻下去。加新的翻页预检时照抄这个形状。
   - **预检是同步读,会加在付费写的响应时间上**(`portfolio create` 最多 1 + N 次)。这是有意的交换——省下的是一次异步失败白烧的算力钱。但别为了「覆盖更全」无限加预检读:只挡①②③那三类静默失败,其余交后端。
5. **API 错误按 `errorCode` 优先分类,再看 HTTP status。** 两个 429 语义相反(`RATE_LIMITED`=重试 vs `QUOTA_EXCEEDED`=弃),必须先看 code(见 `error.rs::classify_api`)。
6. **时间戳输出转东八区,日期与 Value 透传块绝不碰。** 后端发 UTC,面向 A 股用户会被直接读成北京时间、差 8 小时,所以**事件时刻**字段用 `models::Timestamp`(零依赖,反序列化存原文、序列化才换算成 `+08:00` RFC3339,解析不了就原样透传)。**加新时间字段时先分类**:
   - **事件时刻**(`create_time`/`created_at`/`started_at`/`finished_at`/`run_at`/`update_time`/`last_heartbeat`/`generated_at`…)→ `Timestamp`。
   - **日期/区间边界**(`cal_date`/`dates`/`rebalance_dates`/`sdt`/`edt`/`dt`/`latest_weight_date`/`outsample_sdt`/`oos_start`…)→ 仍是 `String`。它们是交易日语义,±8h 整体跨日,「7月24日的持仓」会被读成 25 日。`Timestamp` 里「纯日期串原样输出」是挂错字段时的第二道防线,但别指望它。
   - **`serde_json::Value` 透传块一律不换算**(`metrics`/`trades`/`kline`/`definition`/`realtime`/`verdict`)。**`trades` 的 `kline_key` 内嵌时间却是要原样回传给 `strategy kline` 的路径参数**——改写它那根 K 线就永远查不到;别为了「覆盖更全」去递归改写 Value。

## 约定

- **注释用中文,解释「为什么」**,理由直接内联写在注释里,不引用外部设计文档的小节号。改代码沿用这个密度与风格。
- **stdin 输入**:`auth set`(token)、`route/problem/portfolio create`(一份 JSON body)。`problem create` 仅在 `dataset` 为 `stock`/`etf`/`future` 时额外校验 `symbols` 为字符串数组且每项带市场后缀；`portfolio create` 额外校验 code/候选结构，并在付费 POST 前动态预检 code 冲突与候选实盘状态；其余字段交后端。
- **`strategy register` 的 stdin 是 JSON/TOML 双模嗅探**:先试 `serde_json`,成功就当 `strategy definition` 的输出形态、剥掉 null 后转 TOML;失败才当裸 TOML。**顺序不能反**——TOML 的裸键语法几乎不可能被 serde_json 误判成合法 JSON,反向则不然。**null 必须剥**:TOML 表示不了空值,不剥直接 `unsupported unit type`(实测 `definition` 的 `problem.suffix` 就是 null)。上限 1 MiB 按**转换后**的字节判,因为后端收到的是那份 TOML。这是 CLI 里唯一一处 `toml` 依赖的用途——加它是因为 agent 唯一的合法输入源 `definition` 出的是 JSON,不转就得让 agent 手写没有 schema 文档的 TOML。
- **stdin 也收纯文本,不只 JSON**:`strategy memo` 从 stdin 读一段**裸文本**笔记(不解析 JSON——笔记本身就带引号和换行)。**空 stdin 报 exit 2 而不是当成「清除」**:memo 是覆盖写,一个手滑的空管道会静默抹掉已有笔记且不可恢复;清除必须显式 `--clear`。长度上限 10000 **按 Unicode 字符计不是字节**(中文一字三字节,用 `len()` 会在远未超限时误拦),且先 trim 再计数——跟后端同序,否则边界判定两边不一致。
- **`/research/*` 和 `/strategy/*` 是两个不同的下游服务**,不是同一后端的别名:网关 YARP 把 `/open/v1/research/*` 转给 Rust 投研后端(去前缀 + 加 `/api`),把 `/open/v1/strategy/*` 转给 C# 服务(去前缀 + 加 `/api/strategy`)。**加新端点前先确认资源住在哪一侧**——`status` 走 `/strategy/realtime/*`、`tags`/`memo` 走 `/research/*`,照着邻居抄前缀会 404。
- 端点集中在 `client.rs`;新端点加在那里,别散落别处。

## 技能套件(`skill/` + `src/skill.rs`)

四册独立技能:`factor`(因子资产)、`strategy`(策略资产+实盘)、`portfolio`(组合资产)、`guide`(强引导漏斗)。**拆开不是排版,是触发语义**——guide 要被显式召唤,factor/strategy/portfolio 要被自动想起,而触发由各自 frontmatter 的 `description` 决定,一个技能只装得下一个。

- `skill/_common.md` —— 共享前言(auth / HITL 底表 / I/O 契约)。四册正文里写一行 `<!-- COMMON -->` 占位,安装时就地展开。**源文件单处维护,装出来的副本由安装器写**,所以副本重复不是债。
- `skz skills install|status|uninstall|permissions|show` —— **安装器,不是打印器**。装成 harness 原生技能包。
- **四家 harness 全支持**:`--target claude|codex|openclaw|hermes|all`,根目录分别是 `~/.{claude,codex,openclaw,hermes}/skills/`。四家的约定一致(`<root>/skills/<name>/SKILL.md` + `name`/`description` frontmatter),**所以 adapter 只是换根目录、内容不必改写**——claude/codex 本机实证,openclaw/hermes 依官方文档(两者另有 `~/.agents/skills` 共享区,我们不碰)。
- `--target all` 只装**本机已存在**的 harness(探测 home 下有无 `.<name>` 目录);一家都没有则 exit 2 给可操作提示,不静默装 0 家。**单 target 仍输出对象**(形状不变、老脚本不破),多 target 才输出数组。
- **只写自己的技能目录,绝不碰用户配置**(settings.json / CLAUDE.md 一概不动)——卸载 = 删自己那几个目录,完全可逆。想要权限兜底的用户,`permissions` 只打印文本让他自己贴。
- `.skz-install.json` 是**归属证明 + 版本戳**:没有它的同名目录 = 别人的技能,install 不覆盖(整体拒绝,不装一半)、uninstall 不删;版本落后于二进制 → `status` 报 `stale`/`needs_install`。
- 技能根目录用 `home_dir()`,**不是** credentials 的路径解析——`~/.claude/` 在所有平台(含 Windows)都是固定 home 相对路径;credentials 在 Windows 上仍走 LocalAppData(macOS/Linux 虽也已是 home 相对的 `~/.config`,但两者语义不同,别划等号)。
- 内容真源是二进制(`include_str!`),所以技能不可能描述一个二进制没有的命令。

## 自更新(`src/update.rs`)

`skz update`:按 `current_exe()` 路径探测 Homebrew/Scoop/pipx/uv 安装渠道 → shell 出该渠道自己的 upgrade 命令(自动复用安装工具记下的 registry/index 与凭据,`skz` 全程不摸)→ 核对本机技能副本是否落后于(可能刚变化的)二进制版本。

- **支持渠道**:`brew upgrade skz`、`scoop update skz`、`pipx upgrade skz-quant-cli`、`uv tool upgrade skz-quant-cli`。渠道只从当前二进制路径识别，不因 `PATH` 里存在某个包管理器就猜测；Scoop 只支持 README 推荐的用户级安装，不处理 `--global`。
- **升级后入口**:pipx/uv 仍探测原路径；Homebrew 从 Cellar 路径转到同 prefix 的 `opt/skz/bin/skz`，Scoop 从版本目录转到同 root 的 `apps/skz/current/skz.exe`。版本自检和 delegated skill 刷新必须共用这个稳定入口；包管理器 exit 0 但入口不可用时输出 `updated:null` 并跳过 skill 新鲜度判断，不能误报成“没更新”。
- **识别不出渠道 ≠ 失败**:exit 0,`updated:false`,`remediation` 指回 README 的四种公开安装渠道。`Action::GiveUp` 的语义专属平台侧配额/余额场景，识别不出本机安装方式跟那个域不搭边。
- **升级子进程失败 → `retry_later`(exit 5,`Kind::Subprocess`)**,不是 `WriteNetwork`/`check_existing`——那套"结果未知"机制专门对应业务写的幂等顾虑,重跑 `skz update` 没有这个顾虑,盲重试完全安全。
- **技能新鲜度比对不能信 `skill::status()` 自带的 `stale` 字段**:那个字段硬编码比对 `env!(CARGO_PKG_VERSION)`,也就是"正在跑这次检查的进程自己的版本"——升级成功后仍在旧进程里跑,会用旧版本去跟旧标记比"完全一致",把真正该刷新的场景漏掉。`update.rs` 把比对基准做成显式参数,确认发生版本变化后传重新探测到的新版本;确认变了的刷新还要转手给磁盘上的新二进制自己执行 `skills install`,不能在旧进程里直接调 `skill::install()`(旧进程手上的 `include_str!` 内容本来就是要被换掉的那份)。
- **这是这个 CLI 里第一个、目前也是唯一一个原生交互式终端提示**(真终端时问要不要刷新过期技能)。它**不是** HITL 机制的一部分——问的是"要不要刷新本地文件",不是"要不要花钱/动资产",判据跟下面「HITL」一节完全不搭边;`skz skills install` 本来就不在那份清单里(本地可逆、不花钱)。**别把它当成"CLI 可以弹确认"的先例去改别的写命令**——那些命令"保持哑、不加 `--yes`"的规则不变。只在真终端(`stdin`/`stderr` 都是 tty)才触发,非交互(agent/管道调用)一律只报数据、零副作用,不加 `--yes` 之类的开关去跳过它。
- **已知限制**:子进程无超时(升级工具卡住会让 `skz update` 一直等待,没有整进程墙钟预算的先例可抄);Scoop/Windows 自替换尚未完成实机验证;pipx/uv 的 `detect_channel` 只嗅 `current_exe()` 路径里挨着的 `pipx/venvs`、`uv/tools` 两个 segment——如果用户设了 `PIPX_HOME`/`UV_TOOL_DIR` 之类的环境变量把安装根目录挪到别处,路径里就不会出现这两段,会被误判成 `unknown`(退化成"exit 0 + 指回 README"而不是崩,安全但不完整)。

## HITL(技能层契约,不是 CLI 功能)

花钱或不可逆的写 —— `mine/explore start`、`promote start`、`strategy status 实盘|废弃`、`factor delete`、`portfolio create` —— 技能规定 agent **在调用之前**先问人。**CLI 保持哑**:不弹确认、不加 `--yes`,thin-CLI / rich-skill 分层才不破。加新写命令时,同步更新 `_common.md` 的底表与 `skill::permissions()`。(`skz update` 的技能刷新问答不在这条规则管辖范围内,见上面「自更新」一节——判据不搭边,别误读成这条规则被开了口子。)
