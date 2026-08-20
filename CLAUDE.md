# CLAUDE.md — skz-quant-cli

面向 AI agent 的胜可知(Shengkezhi)开放平台执行器。Rust CLI,二进制名 `skz`。
`lib.rs` 是可复用的 client library,`bin/skz.rs` 只是它的一个入口(未来 MCP server 可直接复用 lib)。
主要能力:**市场数据只读查询** + **量化研究流程** + **因子/策略/组合资产管理(含写/触发)**。edition 2024,MSRV 跟随 stable(当前 `1.97.1`),skill 契约版本 `4.1`。

**MSRV 策略:不压 MSRV。** 官方只发布预编译 GitHub Release 归档;公开源码可供开发和自行构建,但不承诺兼容旧 rustc。压 MSRV 换不到官方分发兼容性、只会反过来钉住依赖。升级 stable 后直接把 `rust-version` 抬上去。

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
- 两个入口共用 `scripts/release/build_target.py`，生成二进制和带 SHA256/version/commit/dirty 状态的 host manifest；`--target` 可只重跑单个平台。
- Plugin 作者源在 `plugin-src/`；运行 `python3 scripts/release/build_plugins.py --sync-only` 生成四家 `plugins/<harness>/plugins/skz/` 原生产物和开发 manifest。
- **仅维护者使用**的 WSL 一键发布入口：`python3 scripts/release/release_wsl.py`。它完成 PATCH bump、测试、五平台构建、外置 bundle、归档、SHA256、push、GitHub Release/Homebrew/Scoop 发布与远端核验。`--check-only` 不修改或发布；失败后用 `--resume`。
- 发布属于维护者操作；普通贡献者不得运行，自动化代理只有在维护者明确要求发布时才可执行。禁止 force push。

## Homebrew / Scoop 分发

公开主仓库 Release 是 Homebrew/Scoop 唯一下载源。每个归档都包含真实二进制及同级 `plugins/`；Homebrew 一起装进 `libexec`，Scoop 原样解压。

## 架构

- `client.rs` —— ureq(blocking + rustls)客户端;端点预定义;GET=读,POST=写/触发。自身不读文件/env,构造时注入 `base_url` + `token`。
- `config.rs` —— 默认 API 地址(`https://api.shengkezhi.com/open/v1`)+ 超时配置，创建联网客户端时读取。
- `credentials.rs` —— token 唯一来源 = 版本化凭据文件；保存命名身份、账户归属、写策略和机器级 active 身份，兼容旧纯文本 token 为 `default`；Unix `0600` 原子写(temp + rename)，`auth add/list/use/remove` 管理。
- `token.rs` —— `Token` 类型,`Debug` 打码成 `Token(***)`,只在注入 Authorization header 时 `expose()`。
- `error.rs` —— `Error` 枚举 + 动作导向退出码 + JSON `ErrorBody` + API 错误分类。
- `retry.rs` —— 有限重试(≤3 次),`Retry-After` 优先否则带抖动指数退避;**仅当 `action == RetryLater` 才重试**。
- `models/` —— serde 类型:`common`(Page)、`experiment`、`factor`、`gift`、`live`、`market`、`mining`、`portfolio`、`problem`、`research`、`strategy`。

## 面向 agent 的 I/O 契约(核心)

- 成功 → **stdout 一份紧凑 JSON + 换行 + exit 0**。`--pretty` 只美化、不改字段结构(给人排障,agent 勿依赖)。
- 失败 → **stderr `{"error":{kind,action,message,status?,code?,retryable?,retryAfterMs?,remediation?}}`** + 退出码。
- **退出码 = Action,1:1**。agent 控制流只 branch 在退出码(粗:下一步动作),细节读 JSON body:

| 码 | action | 含义 / agent 下一步 |
|---|---|---|
| 0 | — | 成功 |
| 2 | `fix_params` | 参数错(含 clap 解析失败)→ 改参数 |
| 3 | `fix_auth` | 缺/无效凭据或未选默认身份 → 照 remediation 添加/选择身份 |
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
3. **凭据不读环境变量。** token 只来自凭据文件；联网命令只用 `auth use` 持久化的 active 身份。新增其他凭据来源或临时身份选择器前必须先明确优先级与安全边界。
   - **这条只管凭据,不管行为开关。** `config.rs` 一直在读 `SKZ_BASE_URL`,现在又加了 `SKZ_READ_ONLY`(见下「只读模式」),都不违反这条——读 env 拿 token 会把密钥摊进进程表和各种 dump,读 env 拿一个 URL 或布尔开关没有这个问题。别把这条援引成「skz 不读 env」。
4. **先校验,再执行目标请求。** page/size、日期、run-id 数量(≤100)、固定枚举、stdin 结构、`problem create` 在 `stock`/`etf`/`future` 数据集下的 symbol 后缀先本地校验；`mine/explore start` 的资产 code、`portfolio create` 的 code 冲突与实盘候选、`mining factors --group` 再通过免费读动态预检。失败均为 exit 2，且目标请求不会发出；其余字段级合法性交后端(400 → `fix_params`)。
   - **判据是「后端对这个参数会静默失败」**,不是「这个参数看着重要」。三种静默失败都实测过:① **静默回空**(`strategy list --status` 传错 → `items:[]`,和真空仓库长得一模一样);② **静默忽略**(`strategy trades --kind` 传错 → 照样回全量,调用方以为筛过了);③ **受理后异步失败**(`mine/explore start` 传不存在的 route/problem → exit 0 起一个 run,7 秒后才 `ok:false`,**而这是花钱的接口**)。共性是**错误被伪装成一个合法结果**,agent 会照着错结论一路走下去。后端明确报 400/422 的字段不在此列——那本来就能正确分支到 `fix_params`,再加一道本地校验只会把值域钉死在 CLI 里。
   - **本地校验 vs 免费读预检,看值域会不会变**:值域固定且不随后端变(枚举、page/size 上限、symbol 后缀形态)→ 本地枚举,别为它发网络;值域是平台资产、随账号和时间变(route/problem code、`portfolio_code` 冲突、`mining factors --group` 的 `problem_groups[].prefix`)→ 免费读预检,**别把动态值域硬编码进 CLI**。
   - **预检翻页的退出条件不能只信后端 `total`。** `preflight_portfolio_create` 用 `received == 0 || live_codes.len() >= total` 双保险:`total` 报大时靠「这一页空了」兜底,否则会一直翻下去。加新的翻页预检时照抄这个形状。
   - **预检是同步读,会加在付费写的响应时间上**(`portfolio create` 最多 1 + N 次)。这是有意的交换——省下的是一次异步失败白烧的算力钱。但别为了「覆盖更全」无限加预检读:只挡①②③那三类静默失败,其余交后端。
5. **API 错误按 `errorCode` 优先分类,再看 HTTP status。** 两个 429 语义相反(`RATE_LIMITED`=重试 vs `QUOTA_EXCEEDED`=弃),必须先看 code(见 `error.rs::classify_api`)。
   - **research 面的数值 `code` 是端点局部的,同一个数字在不同端点语义相反**,所以 `remediation` 不能按裸数字挂。实例:`40907` 在 `factor-routes delete` 是「路线名下还有因子」(可 `--force` 越过),在 `gift claim` 是「领取名额已用尽」(压根没有 force 一说)。挂错的代价不是文案难看,是**教 agent 去 force 一个 force 不了的东西**。机制是 `Error::Research` 上的 `hint: ResearchHint`:`research_err` 一律填 `None`(它只看得到 HTTP + 数字),要挂 remediation 的调用点自己 `.with_research_hint(...)` 打标。**默认不解释,好过默认解释错**;加新端点时除非确认 code 家族无歧义,否则别打标。
6. **时间戳输出转东八区,日期与 Value 透传块绝不碰。** 后端发 UTC,面向 A 股用户会被直接读成北京时间、差 8 小时,所以**事件时刻**字段用 `models::Timestamp`(零依赖,反序列化存原文、序列化才换算成 `+08:00` RFC3339,解析不了就原样透传)。**加新时间字段时先分类**:
   - **事件时刻**(`create_time`/`created_at`/`started_at`/`finished_at`/`run_at`/`update_time`/`last_heartbeat`/`generated_at`…)→ `Timestamp`。
   - **日期/区间边界**(`cal_date`/`dates`/`rebalance_dates`/`sdt`/`edt`/`dt`/`latest_weight_date`/`outsample_sdt`/`oos_start`…)→ 仍是 `String`。它们是交易日语义,±8h 整体跨日,「7月24日的持仓」会被读成 25 日。`Timestamp` 里「纯日期串原样输出」是挂错字段时的第二道防线,但别指望它。
   - **`serde_json::Value` 透传块一律不换算**(`metrics`/`trades`/`kline`/`definition`/`realtime`/`verdict`)。**`trades` 的 `kline_key` 内嵌时间却是要原样回传给 `strategy kline` 的路径参数**——改写它那根 K 线就永远查不到;别为了「覆盖更全」去递归改写 Value。

## 约定

- **注释用中文,解释「为什么」**,理由直接内联写在注释里,不引用外部设计文档的小节号。改代码沿用这个密度与风格。
- **stdin 输入**:`auth add`/兼容入口 `auth set`(token)、`route/problem/portfolio create`(一份 JSON body)。`problem create` 仅在 `dataset` 为 `stock`/`etf`/`future` 时额外校验 `symbols` 为字符串数组且每项带市场后缀；`portfolio create` 额外校验 code/候选结构，并在付费 POST 前动态预检 code 冲突与候选实盘状态；其余字段交后端。
- **`strategy register` 的 stdin 是 JSON/TOML 双模嗅探**:先试 `serde_json`,成功就当 `strategy definition` 的输出形态、剥掉 null 后转 TOML;失败才当裸 TOML。**顺序不能反**——TOML 的裸键语法几乎不可能被 serde_json 误判成合法 JSON,反向则不然。**null 必须剥**:TOML 表示不了空值,不剥直接 `unsupported unit type`(实测 `definition` 的 `problem.suffix` 就是 null)。上限 1 MiB 按**转换后**的字节判,因为后端收到的是那份 TOML。这是 CLI 里唯一一处 `toml` 依赖的用途——加它是因为 agent 唯一的合法输入源 `definition` 出的是 JSON,不转就得让 agent 手写没有 schema 文档的 TOML。
- **stdin 也收纯文本,不只 JSON**:`strategy memo` 从 stdin 读一段**裸文本**笔记(不解析 JSON——笔记本身就带引号和换行)。**空 stdin 报 exit 2 而不是当成「清除」**:memo 是覆盖写,一个手滑的空管道会静默抹掉已有笔记且不可恢复;清除必须显式 `--clear`。长度上限 10000 **按 Unicode 字符计不是字节**(中文一字三字节,用 `len()` 会在远未超限时误拦),且先 trim 再计数——跟后端同序,否则边界判定两边不一致。
- **`/research/*` 和 `/strategy/*` 是两个不同的下游服务**,不是同一后端的别名:网关 YARP 把 `/open/v1/research/*` 转给 Rust 投研后端(去前缀 + 加 `/api`),把 `/open/v1/strategy/*` 转给 C# 服务(去前缀 + 加 `/api/strategy`)。**加新端点前先确认资源住在哪一侧**——`status` 走 `/strategy/realtime/*`、`tags`/`memo` 走 `/research/*`,照着邻居抄前缀会 404。
- **删除类命令的三条约定**(`experiment delete-run`、`factor-routes delete`):
  - **不靠可选位置参数区分删除粒度。** `experiment delete <id> <code>`(删一个候选)与 `delete-run <id>`(删整次探索)只差一个参数,合并成 `code: Option<String>` 的话 agent 少传一个就从删一条静默升级成删一批,且不可逆——正是「错误被伪装成合法结果」那一类。宁可多一个动词。
  - **软护栏 409(40906/40907)仍走 `check_existing`/exit 7,靠 `remediation` 补差。** 这两条的正确下一步是「确认后带 `--force` 重发」,跟 exit 7 的字面意思相反;但**不为它新开退出码**——码即 action,加一个就得让每个 agent 重学映射表,而「先查现有状态」本来就是这里对的第一步,差的只是查完怎么办,那属于细节、本来就该读 body。硬拒绝的 40905(实盘任务在跑)**不挂**这条 remediation,否则等于教 agent 撞墙。
  - **`factor-routes delete` 会「exit 0 但删了一半」**(路线行已删、个别执行目录没清掉,后端仍回 200)。退出码保持 0——用户意图达成、重发即续删——所以 `failed_mining_runs` 必须原样透出,并在 `_common.md` 显式教 agent 看它。这是本 CLI 唯一一处 exit 0 不代表事情做完,别再造第二处。
- 端点集中在 `client.rs`;新端点加在那里,别散落别处。

## 原生 Plugin（`plugins/` + `src/plugin.rs`）

每个 harness 只安装一个名为 `skz` 的原生 plugin；plugin 内含 guide/create-problem/factor/candidate/strategy/portfolio 六个独立 skills。

- 公开命令只有 `skz plugin install|status|upgrade|uninstall <target>`；target 必填，无 project scope、show 或 permissions。
- `all` 只处理 PATH 中存在原生 CLI 的 harness；单 target 输出对象，多 target 输出数组。
- bundle 同步到 `~/.skz/plugins/<target>/source`，receipt 位于同级 `.skz-plugin-install.json`；contract 当前为 `4.1`。
- Claude/Codex 使用本地 marketplace，OpenClaw 使用 Claude-compatible marketplace，Hermes 使用 `plugin.yaml` 和原生 skills 注册。
- 安装成功后才清理带可信 SKZ marker 的旧 skills；外来或不可确认目录在任何写入前报错。
- 资源只从 `SKZ_PLUGINS_DIR` 或 `canonicalize(current_exe()).parent()/plugins` 加载，并严格校验 manifest、SHA256、mode、路径和版本。

## 自更新(`src/update.rs`)

`skz update`:按 `current_exe()` 路径探测渠道；Homebrew/Scoop 执行升级并核对 plugin。

- **支持升级渠道**:`brew upgrade skz`、`scoop update skz`。
- **升级后入口**:版本自检和 delegated plugin 刷新共用 Homebrew `opt` / Scoop `current` 稳定入口；入口不可用时输出 `updated:null`。
- **识别不出渠道 ≠ 失败**:exit 0,`updated:false`,`remediation` 指回 README 的四种公开安装渠道。`Action::GiveUp` 的语义专属平台侧配额/余额场景，识别不出本机安装方式跟那个域不搭边。
- **升级子进程失败 → `retry_later`(exit 5,`Kind::Subprocess`)**,不是 `WriteNetwork`/`check_existing`——那套"结果未知"机制专门对应业务写的幂等顾虑,重跑 `skz update` 没有这个顾虑,盲重试完全安全。
- Plugin 新鲜度使用 receipt 与显式的新 CLI/contract 基准；确认升级后由磁盘上的新二进制执行 `plugin upgrade`。
- 真终端可询问是否刷新过期 plugin；非交互调用只报告、零副作用。
- **已知限制**:子进程无超时(升级工具卡住会让 `skz update` 一直等待,没有整进程墙钟预算的先例可抄);Scoop/Windows 自替换尚未完成实机验证。

## 身份写策略与全局只读模式

每个命名身份必须在 `auth add` 时显式选 `--read-only` 或 `--allow-write`；最终只读状态为「身份只读 OR `SKZ_READ_ONLY=1`」。命中后所有写/触发 exit 8(`not_permitted`)，**请求不发出**。

- **不是安全边界,是防手滑。** token 就在 agent 读得到的文件里,它想 curl 随时能 curl。防的是健忘/顺手的 agent,不是对抗。**别在文档里把它写成「保证」**——真要不可绕过,得让 key 主人发一把窄 scope 的 key(网关 `OpenPlatformScopes` 按路径+方法查 scope,去掉 `strategy:write` 能服务端硬拦掉 `mine/explore start` 与 `strategy status`;但 research 面一个 scope 管读写,`promote`/`portfolio create` 切不开,那正是本模式要补的缺口)。
- **闸装在 `client.rs` 的 `post_json` / `send_research_json`,default-deny。** 新加的写自动被拦;动词是写但后端零修改的要显式走 `post_json_readlike`(两个 `poll`)或 `send_research_json_readlike`(只有 `factor-routes delete --dry-run`)。**dry-run 那条例外的理由要记住**:只读模式的动机就是「让 agent 看清代价再交人决定」,把这份代价预告一并封掉恰好封掉了它自己想要的东西;代价是「只读模式下确实会有 DELETE 发出去」,所以例外必须由调用方在 `dry_run` 为真时**显式选择**,别让别的删除路径顺手复用。**别在别处再加一道判断**——`ensure_writable` 也 pub 给了 `preflight_*` 做早退,但那只是省几次免费读的优化,漏了不影响正确性;传输层那道才是不变量。
- **只认 unset 为关闭,`SKZ_READ_ONLY=0` 报 exit 2 而不是关闭。** agent 撞到 exit 8 后最顺手的下一步就是 `SKZ_READ_ONLY=0 skz ...` 再试一次,认 `0` 等于白做。顺带也堵掉值写错(`ture`)静默退化成「关闭」。**变量名写错仍是静默失效**,只能靠 `skz auth status` 的 `readOnly` 字段人工确认——所以那个字段是功能的一部分,别删。
- **不提供单次命令逃生舱。** 身份策略只能在 auth 管理面设置；全局 `SKZ_READ_ONLY` 仍只认 unset 为关闭。agent 撞到只读错误后不得自行 `auth use` 切换可写身份。
- **`remediation` 的措辞是功能的一部分**:agent 撞到工具报错的默认反应是换条路达成目标,所以必须写「停手交人、别找别的路」,且**不出现 API 地址与凭据文件路径**。这跟本 CLI 别处「照 `verifyWith` 接着验证」的语气正好相反,是有意的。
- **「只读」= 本 CLI 不发起扣费、不改资产状态,不等于上游零副作用。** 两个 `poll` 和几个 GET 在后端会把 >24h 的 run 翻成 `timeout` 并**退款**;真封掉它们,封掉的恰恰是给用户退钱的路径。
- 本地参数校验排在闸前面(闸在传输层),所以参数也错的写命令先拿 exit 2、改对了才拿 exit 8。这是接受的取舍:把闸提前到每条命令入口就要维护一份命令清单,漏登记的代价是漏出一次真的写。
- 加新写命令时，除了公共运行契约的 HITL 底表，还要把它加进 `tests/cli.rs` 的 `write_commands()` 表。

## HITL(技能层契约,不是 CLI 功能)

花钱或不可逆的写 —— `mine/explore start`、`promote start`、`strategy status 实盘|废弃`、`factor delete`、`experiment delete`/`delete-run`、`factor-routes delete`、`gift create`/`gift claim`、`portfolio create` —— 技能规定 agent **在调用之前**先问人。**CLI 保持哑**：不弹确认、不加 `--yes`。加新写命令时同步更新公共运行契约的底表。

两条赠予命令都不花钱,进表靠的是「不可逆」那一半:`gift create` 发出的码**本身就是这几条策略的访问凭证**,拿到码的人不需要别的授权就能领走完整定义,`gift revoke` 只挡得住还没领的人;`gift claim` 往自己实盘库写入最多 10 条策略,而实盘库没有删除命令,进来了就只能改状态。`gift preview`/`list`/`revoke` 不进表——预览零副作用,撤回是收回自己的披露、方向安全。
