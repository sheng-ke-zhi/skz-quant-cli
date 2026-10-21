# skz agent 运行契约

执行任何 `skz` CLI 命令或直接 API 请求前读本文件。它定义共同的身份、授权、重试和写后确认规则；CLI 的结构化 I/O 与退出码不等于裸 API 响应格式。

## 直接 API 调用边界

- 直接 API 继承本文件的身份、凭据、只读策略、人工确认、费用核验和未知写结果规则，不能作为绕过 CLI 的通道。优先使用已有 CLI 功能。
- 调用前核对当前身份及账户归属；有 CLI 时先 `skz auth status`、`skz whoami`。没有 CLI 或无法确定凭据归属、只读策略、scope 时暂停交给用户，不默认可写。不得自行换身份、换 key、提高权限或修改 `SKZ_READ_ONLY`。
- `readOnly:true` 下停止一切写入或付费触发，即使某项通常可自主执行也不例外。HTTP 方法不构成读写依据；按端点参考的实际副作用分类，未知先暂停核实，不试调用。
- 仅使用用户明确授权给当前身份和用途的凭据；平台 token 与用户模型 API Key 都不得写进对话、示例、日志、命令参数或持久脚本。参考中的 `sk_xxx` 是占位符，不应替换成真实密钥后粘贴或提交；实际请求从受限凭据来源安全注入，禁止 shell tracing 或输出认证头。
- 人工确认必须绑定本次端点、账户、资源和实际参数；PUT 模型配置更新及可能消耗模型额度的 probe 也须确认。直接 API 不免除费用与余额核验，缺少费用依据先暂停。
- 所有写请求不自动重试；超时、断连、回执缺失或异常时结果为“未知”，不能把 HTTP 方法的幂等性当作重发许可。用同一身份按可信任务 ID 或资源标识只读核验实际变更；查不到或查询失败仍是未知。没有可靠核验接口或可信标识则暂停交给用户，不盲重发、不创建替代任务；HTTP 成功也须检查端点业务回执及批量逐项结果。

## 身份与只读策略

每条工作流开工前运行：

```bash
skz auth status
skz whoami
```

- `active` 决定资产、余额和数据归属。不得根据读写权限自行切换身份。
- `readOnly:true` 表示当前身份或整台机器禁止写。停止所有写操作并交给用户，不得换 key、改环境变量或绕过 CLI。
- 没有 token、没有默认身份或缺 scope 时，按错误里的 `action` 和 `remediation` 处理。不得自行切换到权限更高的身份。
- 用户可通过 `skz auth login <identity> --read-only|--allow-write` 在浏览器授权，也可用 `auth add` 从 stdin 保存 API key；身份保存后由用户明确选择 `auth use <identity>`。不得自行提高写权限或覆盖身份。
- OAuth 业务请求前自动续期；`auth status` 的 `authType` 为 `oauth` 时，`expiresIn` 是 access token 剩余秒数。`loginRequired:true` 表示刷新失败或中断，交给用户重新登录（同名覆盖需 `--replace` 并指定写策略），不得自行重放刷新请求。`status` 只读本地文件，不保证服务端凭据仍有效。
- token 只保存在受限凭据文件；不要在输出中展示 token。浏览器授权链接不含 token，但包含短期 state 和 PKCE 参数，也不要转发。

## 人工确认边界

调用前必须获得用户明确确认的操作：

| 操作 | 原因 |
|---|---|
| `mine start` / `explore start` | 付费触发 |
| `promote start` | 免费，但会保存入库并消费候选 |
| `strategy refresh` | 按去重后的策略数付费更新实盘结果 |
| `strategy refresh-one` | 单策略付费更新的兼容入口 |
| `task create` / `task retry-payment` | 统一任务入队或重新扣费 |
| `llm-config create` / `update` / `delete` / `probe` | 保存或使用用户自有模型凭据；probe 可能消耗模型额度 |
| `portfolio build start` | 前端同款 Agent 构建，按模型模式核费后确认 |
| `portfolio create` / `portfolio refresh` | 触发组合优化；可能付费，费用核实后仍须明确许可 |
| `portfolio status --status 实盘|废弃` / `portfolio delete` | 真金运行、不可逆状态或物理删除 |
| `strategy status --status 实盘` / `strategy research-status --status 实盘` | 真金开始运行 |
| `strategy status --status 废弃` / `strategy research-status --status 废弃` | 不可逆并进入写保护 |
| `strategy register` | 免费，但会把未经回测的策略直接入库 |
| `factor delete` / `delete-batch` | 对已有资产作逻辑处置；批量确认须覆盖名称清单与共用理由 |
| `mining delete-run` | 永久删除单次挖掘产物 |
| `experiment delete` / `delete-run` | 永久删除候选或整次探索 |
| `factor-routes delete` | 永久删除路线并级联删除执行 |
| `stats delete-exploration-run` | 永久删除工作台统计记录 |
| `worker-task create` | 创建 Research Worker 任务 |
| 删除时增加 `--force` | 越过后端软护栏，必须二次确认 |
| `problem delete` | 物理删除且不可恢复 |
| `gift create` / `gift claim` | 不可撤回披露或写入资产 |
| `route create` / `factor-routes create` | 创建前完整展示待建路线，并取得明确许可后才调用 |
| `problem create` | 创建前完整展示待建问题，并取得明确许可后才调用 |

以下操作可以自主执行：所有读命令；`factor-routes delete --dry-run`；`gift preview/list/revoke`；切换到暂停；标签整理；正常写 memo。清空 memo 前先读回旧内容。

确认必须绑定本次实际参数。付费操作先按 [billing.md](billing.md) 核验费用与余额，再展示核心假设、失败信号、当前余额、预计耗时和本次费用并请求确认。有对应价目时运行 `skz wallet check`；组合创建与刷新没有对应报价能力时必须暂停，不能套用策略刷新报价，也不能以跳过余额检查绕过费用核验。验证计划可运行 `scripts/validate_plan.py`；脚本只校验，不执行写操作，不核验组合费用，也不代表授权。

## 结构化 I/O

- 成功：stdout 是一份 JSON，exit 0。空数组或 `total:0` 是成功，不是错误。
- `factor delete-batch` 的 exit 0 表示收到批量回执，部分或全部项目仍可能失败。检查 `failed_count` 和 `items[].success/code/msg`，不得把整批报告为成功或自动重放整批。
- 失败：stderr 是 `{"error":{"kind","action",...}}`。按 `action` 和退出码分支，不解析自然语言 `message`。
- 异步查询 exit 0 只表示查询成功。任务结果必须看 body：`done:true` 是已结束，`ok:false` 才是任务失败。
- 分页默认通常只有 5 条。先看 `total`，需要时显式翻页，不得把首页当全集。
- 先查看真实 schema，再写 `jq` 投影。字段猜错时 `jq` 可能静默返回 `null`。
- 中文键使用 bracket 记法，例如 `jq '.["夏普比率"]'`。
- 枚举和筛选值写错可能只返回空结果；看到空结果先核对合法值和大小写。

退出码动作：

| exit | action | 下一步 |
|---|---|---|
| 2 | `fix_params` | 读命令修正参数后重试；写命令先按下节核验与授权 |
| 3 | `fix_auth` | 交给用户修复身份、token 或 scope |
| 4 | `retry` | 只重试幂等读；尊重 `retryAfter` |
| 5 | `retry_later` | 等待后重试读，不重放写 |
| 6 | `give_up` | 停止并报告，例如额度不足 |
| 7 | `check_existing` | 查询已有资源或任务；不要直接重触发 |
| 8 | `not_permitted` | 本机禁止写，停止并交给用户 |

## 重试与写后确认

- 读命令和 poll 可以有限重试。
- 所有写命令一律不自动重试，即便单个端点看似幂等。
- 写命令遇到超时或连接失败时，结果是“未知”，不是“失败”。先运行 `scripts/verify_write.py` 或错误中的 `remediation.verifyWith` 读回确认。核验查询失败仍是未知；即使暂时查不到资产也不能证明原写不会稍后完成，脚本不会授权自动重试。
- `verify_write.py mine.start|explore.start --run-id <原始回执的fcRunId>` 按任务 ID 精确核验，可加 `--code <路线编号>` 约束同一列表行的路线。真实 CLI 的 runs 列表不输出 problemCode；探索的 `--code <问题编号>` 必须沿同一可信 ID 只读调用 `explore poll`，核对其显式 problemCode、任务 ID、已知状态及可选路线与列表一致；证据缺失、错配或读取失败均保留未知。挖掘不提供问题归属核验。只有路线编号时最多输出 `target_match_only / confirmed:null`；只有问题编号不扫描或认领其他任务。没有可信 ID、查不到、状态未知或分页不完整都保留未知，不将名称或嵌套结果中的同名字段当作回执 ID。
- `verify_write.py strategy.write --code <策略编号> --expected '{"status":"预期状态","memo":"预期笔记"}'` 比较本次预期字段（也支持 `tags`），策略编号必须一致；仅存在不等于本次写入成功。预期含 `memo` 或 `tags` 时分别使用 `strategy get --require-memo`、`--require-tags`（可同时要求），在 CLI 默认值填充前验证原始字段存在且类型合法。缺失/null/非法 tags 保留未知，真实 `[]` 才是清空标签的可观察证据，非空数组按预期比较；字段不符也不授权重放。严格开关要求支持该能力的 CLI，旧 CLI 不支持时停止核验，不回退到宽松读取。
- `resume.py --size 100` 的 size 是每页数量，不是总量上限。脚本不依赖 `active` 别名，读取 mine/explore 未筛选列表的全部页，核对总数稳定、页号/大小、任务 ID 唯一及状态。`queued/dispatching/running` 保留为在途；仅 `succeeded/failed` 且 done/ok 一致时视作终态。`timeout`、未知状态、矛盾字段或分页异常使 `ready:false`，`complete` 标明两类任务各自是否完整，已读任务仍保留供人工核查。`safe_to_retry:false` 始终不变；恢复定位不授予接管或续作授权。
- `active` 是否遗漏 dispatching 仍是待隔离验证的服务端事实，未作为已确认线上缺陷。分页没有服务端快照保证；校验只能发现可观测的总数变化、重复和缺页，不能证明翻页期间不存在等量替换；动态任务台应再次只读核对，不据此触发写入。

### 研究任务键与可信接管

- 挖掘的逻辑任务键是 `routeCode`；计划内多轮挖掘另记轮次。探索的逻辑任务键是 `(problemCode, routeCode)`，同路线不同问题是不同组合，同问题不同路线也不能合并。同键历史终态不等于本次已执行，也不授权再次付费。
- 这些是客户端计划与去重口径，不是服务端原子锁保证。探索 API 参考将 409 描述为同一问题与路线正在执行；服务端实际并发锁粒度、旧响应行为仍待隔离验证，不据此推断不同键必可并发。driver 对账号内未纳入账本的在途任务和同路线 timeout 的保守暂停是安全边界，不是后端锁证据。
- 接管只使用本次可信 start 回执或已持久化账本中的 `fcRunId`，只读查完整任务台并按该 ID poll；ID 必须唯一匹配，挖掘核对路线，探索同时核对问题与路线。任何错配或缺少归属证据都暂停，不自动认领列表中的其它任务。
- `problemCode` 按响应实际值处理：非空且匹配时可作归属证据；缺失、null、空字符串均为未知。列表缺失时可按可信 ID poll 补证，仍缺失或与列表矛盾则暂停。字段在 schema 中存在不保证值可用；`experiment list` 只补覆盖统计，不替代提交回执。
- `createdAt` 仅供展示和辅助排查。即使两项任务时间相近、参数相同，也不能据此猜本次 ID；没有可信 ID 时只读核验并交给用户，不自动接管、不释放未知费用、不重新付费提交。409 / `check_existing` 也不授予接管或重发许可。

### 写操作重试与续作唯一规则

退出码不能单独证明写入结果；先区分以下三类，再决定下一步。以下情形的自动追加写调用次数均为 **0**，等待或只读核验不等于授权重发。

| 结果类别 | 必需证据与只读核验 | 下一步 |
|---|---|---|
| 确定业务拒绝 | 完整业务回执明确拒绝本次写入；按 `remediation` 核对当前资源与拒绝原因 | 先解决原因并展示影响，再重新取得用户确认；不可按 exit 7 或 409 一概重发 |
| 确定部分完成 | 完整业务回执明确部分完成，且读回与回执一致，能列出已完成内容和剩余影响 | 仅可按下述受控续作条件申请一次续作；部分成功不是整次失败 |
| 传输结果未知 | 超时、断网、回执缺失/无法解析，或核验失败、字段缺失、证据冲突 | 停止写入，只读查证并交给用户；一次查询为空、404、`has_performance:false` 或当前无活动任务，均不是原写永不落地的证明 |

受控续作不是自动重试：必须先读回核对同一身份、同一目标的当前状态，向用户展示已完成内容、剩余影响及费用，并取得**绑定同一目标、参数和剩余影响的一次性明确续作授权**。原始操作许可不包含无限重发。未知写结果不能仅凭后续 `pending + resumable=true` 或用户同意就转为可续作；原请求是否仍会完成无法核清时继续暂停。付费操作须重新核价并确认；组合报价未知仍须暂停，不得用续作授权绕过费用核验。

- 路线删除：完整实删回执（不是 `--dry-run`）中的 `failed_mining_runs` 非空时，先用 `factor-routes list` 和 `mining runs --route <code>` 核对路线及剩余执行，展示失败 run 清单与级联影响；无法核清则停止。不得自行新增 `--force`，越过护栏仍须二次确认。
- gift 领取：先用 `gift preview` 核对 `claim_status`、`resumable`、`claim_reason`、资产清单与接收身份。`done` 只读确认已有 `target_code`，不再 claim；只有 `pending + resumable=true` 且剩余复制影响可核清时才能申请续作。`resumable` 只是业务能力，不是用户授权。
- gift「正在领取中」：仅表示本次并发请求被拒绝，不证明另一请求没有落库或占用名额；可有限退避后重读 preview，不能等待后直接 claim。名额用尽则停止并联系送方。
- 同一原始操作在本工作流内最多追加 **1 次**经重新确认的写调用（含拒绝后的重提或部分完成后的续作），不循环、不因重新 preview 而重置次数。追加后必须读回；再次拒绝、部分完成、未知结果、核验失败、授权缺失或影响变化均停止并报告，不再追加写入。

常见读回路径：

| 写操作 | 读回确认 |
|---|---|
| `route create` / `factor-routes create` | `factor-routes list` |
| `problem create` / `delete` | `problem list` / `problem get` |
| `mine start` / `explore start` | 对应 `runs --status active` |
| `promote start` / `strategy register` | `strategy list`，有 promotion id 时再 `promote get` |
| `strategy refresh` | `strategy refresh-active` |
| `experiment delete` / `delete-run` | `experiment strategies` / `experiment list` |
| `factor-routes delete` | `factor-routes list` 加 `mining runs --route` |
| `factor delete` / `delete-batch` | 按输入清单逐项 `factor get`，核对 `is_deleted` 与 `delete_reason` |
| `mining delete-run` | `mining runs`，确认目标 run_id 已消失 |
| `gift create` / `revoke` | `gift list` |
| `gift claim` | `gift preview` 的 `claim_status` / `resumable`；完成后使用 `target_code` |
| strategy status/tag/memo | `strategy get` |
| `portfolio create` / `refresh` | `portfolio list` / `portfolio get` 的 `has_performance` |
| `portfolio status` / `delete` | `portfolio get` / `portfolio list` |
| `llm-config create` / `update` / `delete` / `probe` | `llm-config list` |
| `task create` / `retry-payment` / `cancel` | `task list`，有 taskId 时再 `task poll` |
| `stats delete-exploration-run` | `stats exploration-runs` |
| `worker-task create` | `worker-task list` |

## 长任务等待与宿主生命周期

- **云端业务任务**：`mine start` / `explore start` 返回真实 `fcRunId` 后在服务端执行；CLI 返回不代表业务完成。
- **本地监控进程**：轮询并收集结果，不拥有云端任务的生命周期。本地停止、工具超时或回合结束不等于云端取消，也不代表可以重新提交。
- **宿主托管任务**：宿主提供的受管执行机制，不等同于云端业务任务；工具返回后台句柄本身不证明持久托管或完成唤醒。
- 默认使用前台阻塞等待并在当前回合收集结果。只有宿主明确保证持久托管和可靠完成唤醒时才可使用受管后台；任一保证缺失或未知，都不得后台启动后结束回合。不绑定平台专有工具，不新增后台服务。
- 在 Multica 等结束回合即回收运行的宿主，不得后台启动后退出回合，不使用 `nohup` 或裸 `&`；不能把工具的“稍后通知”当作可靠唤醒保证。
- 监控窗口耗尽或中断时，保存真实 `fcRunId`、配置、账本和已收集结果，报告“监控暂停，云端状态待核验”，不宣称取消或完成。续跑只按同一可信 ID 只读核验和轮询，不为恢复监控重发 `start`；无可信 ID 则暂停并交给用户，不猜测 ID。
- 自动投研以同一配置和独占目录恢复 `run_pipeline.sh`，保留 `pipeline_state.json` 与 `events.ndjson`；仍遵循[写操作重试与续作唯一规则](#写操作重试与续作唯一规则)，续跑不是新的付费授权。

## 公共安装工具与职责

### 安装与版本

安装用 `skz plugin install <claude|codex|openclaw|hermes|dsh|workbuddy|all>`，状态以
`skz plugin status <target>` 的 `needs_install` 为准；升级后若报告 stale，重新安装。
`skz --version` 输出 CLI 与 plugin contract，命令参数以 `skz --help` 为准。
DSH 网页版默认关闭 skill，装完后到 Settings → Plugins 确认 `skill-filesystem` 与
`tool-skill` 为 Enabled（CLI/headless 默认已开启）。

### 只读与校验工具

以下脚本从本次实际加载的 skill 根目录解析，均为只读或纯校验：

- `scripts/preflight.py --operation read|write|paid`：检查 CLI、身份和本地写策略。
- `scripts/resume.py`：跨会话重建在途的因子挖掘、探索和组合任务。
- `scripts/validate_plan.py`：校验付费计划；只返回 `approved:false`，绝不代替用户批准。
- `scripts/verify_write.py`：写超时后读回确认；绝不重放写命令。

### 技能职责

| 技能 | 负责 |
|---|---|
| `skz-wallet` | 资金和费用，不代为触发付费动作 |
| `skz-guide` | 研究导航、单步因子挖掘与策略探索 |
| `skz-auto-research` | 已确认计划的端到端批量编排与报告 |
| `skz-create-problem` | 定义研究问题 |
| `skz-factor` | 因子资产 |
| `skz-candidate` | 实验、候选和保存入库 |
| `skz-strategy` | 已入库策略与实盘更新 |
| `skz-portfolio` | 组合 |
| `skz-openapi` | 开放 API 与页面直达 |

任务跨边界时切换到对应技能，不在当前册猜另一册的契约；本表不扩大任何技能的写权限。

## 时间和原始标识

- 事件时刻已转换为东八区并带 `+08:00`；不要再加八小时。
- 交易日和区间边界字段原样使用，不做时区换算。
- `trades` 等松散块里的时间不保证转换。`kline_key` 是路径标识，必须逐字符原样传回。
- `skz --version` 输出 CLI 与 skill contract；`skz --help` 和子命令 `--help` 是命令真源。
