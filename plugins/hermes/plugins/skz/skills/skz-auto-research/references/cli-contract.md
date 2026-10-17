# skz CLI 契约与字段口径（实测）

> 本文件说明编排取数与判据；共同 operating-contract 优先，命令参数以当前 `skz <cmd> --help` 为准。
> “实测”样例来自原作者历史观察，未经本次在线复测，不是当前价格、规模、耗时或服务端行为保证。

## 0. 基础

- 二进制：`skz`（macOS Homebrew 默认装在 `/opt/homebrew/bin/skz`，不在 PATH 时用 `SKZ_BIN`
  环境变量指过去）。`skz` = 胜可知开放平台执行器，**只读行情 + 策略业务创建/触发**。
- 输出：一律 JSON 到 stdout；错误走 `{"error":{"kind","action","message"}}` + exit 2（参数）/ 5（数据未就绪）。
- `--pretty` 只给人排障用，**agent 别加**（不改变结构但会破坏单行解析习惯）。
- 身份：先 `skz auth status` 检查本地只读策略，再 `skz whoami` 自检。凭据路径由 CLI 按平台管理，不读取或输出 token。
  `SKZ_BIN` 仅供 driver 选择可执行文件；不得通过修改 `SKZ_READ_ONLY` 或身份绕过限制。
- **本 driver 不调用取消接口**：`mine` / `explore` 命令族无 cancel；统一 `task cancel` 是另一条工作流，
  不能据此断言本流程可撤回、会退款或云端已停止。串行挖掘避免低产后仍继续扣费。
- **禁止写命令盲重发，默认不付费补跑**。用户单独授权后，明确执行失败可经 readback/预算闸门最多补跑一次；
  这是新逻辑尝试，不是传输重试。未知提交、缺 ID、timeout 一律只读核验后暂停，详见第 9 节。

## 1. 研究问题（problem · 读为主）

```bash
skz problem meta                 # 建问题前必查：合法 type/dataset/freq + default_time_segments + max_time_segment_date
skz problem list                 # ⚠️ 无分页参数，一次返回全部（实测 60 条）
skz problem get <code>           # 详情：dataset/freq/symbols[]/problem_type/description
```

`problem list` 单条字段（实测 2026-09-19）：

```json
{"code":"STS_QS_LEADERS","name":"券商龙头时序研究","dataset":"stock","freq":"日线",
 "symbols":["600030.SH",...],"problem_type":"TimeSeriesProblem","type_label":"时序择时",
 "source":"user","editable":true,"description":"..."}
```

本 driver 只接收已确认且存在的问题，不自动创建或删除问题。定义、标的、时间分段、
平台硬上限与创建核验统一交给 `skz-create-problem`；先查当前 `problem meta`，不能照抄历史日期。
时序标的一般 ≤10 个且高相关是经验建议，不是平台硬限制；两条流水线面向时序量价择时，
不因枚举中存在截面类型就默认支持截面流程。

## 2. 因子路线（route / factor-routes）

```bash
skz factor-routes list           # 方向全集（研究面真源）。字段：code/name/key_inspect/economic_logic/
                                 # why_effective/market_mechanism/failure_scenarios/tags/create_time/creator
skz route adopted                # ⚠️ 只是"已采用"子集，可能回空而账号其实有很多路线 → 别用它判有无
skz route create < body.json     # 写。提交 7 个核心字段：name/key_inspect/economic_logic/
                                 # why_effective/market_mechanism/failure_scenarios/tags；可选 compute_engine
skz factor-routes delete <code>  # 写·物理删 + 级联删名下挖掘执行
```

- **`factor-routes` 没有 update**：建之前逐字核对，创建后仍逐字段 readback。
  读回差异暂停人审，不自动接受、不删除重建；用户接受差异不等于通用 driver 支持该例外，见第 10 节。
- **本 driver 的路线创建契约不接收 `describe` 或 `description` 字段**，不得提交或参与提交字段的读回比较。
  CLI 的 `FactorRoute` 读模型仍含 `description`，不要把创建契约误称为完整响应 schema；额外读回字段不代表 driver 支持写入。
  研究要求与约束写入 `key_inspect`；这不影响研究问题（problem）的 `description` 等其他对象字段。
- `compute_engine` 实测全库都是 `TimeSeriesAstEngine`。
- **`market_mechanism` 后端合法枚举共 10 个**（逐字，禁组合/禁后缀/禁近义改写）：
  `错误定价` · `风险补偿` · `行为偏差` · `流动性溢价` · `制度性套利` · `自我实现预言` ·
  `微观结构` · `信息扩散` · `趋势跟踪` · `套保压力`
  ⚠️ 本账号实测（2026-09-19 复核 68 条）**只出现过其中 7 个**：
  `行为偏差`(24) / `趋势跟踪`(21) / `信息扩散`(7) / `错误定价`(6) / `自我实现预言`(6) /
  `风险补偿`(2) / `套保压力`(1)（另 1 条为空）。剩下 3 个从未出现 ≠ 非法——别用
  "账号里没见过"否掉一个值。
  ⚠️ 这 10 个值**没有发现端点**（route 侧没有 `meta` 命令）→ 怀疑清单过期时，用账号实际值反查：
  `skz factor-routes list | jq -r '.items[].market_mechanism' | sort | uniq -c`，以真实返回为准。
- `key_inspect` 是引擎的**主指令面**，必须自包含，完整承载研究要求、定义与约束。
  用户偏好全中文自然语言、先给定义/底层逻辑再给算子链。

## 3. 因子挖掘（mine · 写；mining · 读）

```bash
skz mine start --route <routeCode>     # 写·扣费 → {fcRunId,status,routeCode}，1 秒返回
skz mine poll <fcRunId> [<id2> ...]    # 读·批量（≤100 id）→ 裸数组
skz mine runs [--status <s>] [--page N] [--size M]   # 任务台：含失败的。默认 size=5，必须显式翻页
skz mining runs [--route <code>]       # 成果柜：只有挖成的。⚠️ 无分页参数
skz mining overview <RUN_ID>           # 漏斗 + 淘汰分解 + kpi
skz mining factors <RUN_ID> [--page N --page-size M]  # 挖出的因子（默认 page-size 5）
```

**`mine poll` 返回（裸数组，不是 `{items:[]}`）**：

```json
[{"fcRunId","status","routeCode","statusText","done","ok","createdAt"}]
```

- 在途状态 `queued/dispatching/running`，`dispatching` 表示准备执行；应为 `done:false` 且 `ok` 不为 true。
  `done:false/ok:false` 不代表失败。**没有 `percent` 字段**（explore 才有）。
- 先核对 `status/done`，终态再结合 `ok`：成功要求 `succeeded/done:true/ok:true`；
  明确失败要求 `failed/done:true/ok:false`。未知/矛盾状态、timeout 暂停，不猜测已停止计算。
  `SKZ_LOGIC_ERROR` 不能泛化为可补跑执行失败；权限等错误原文交给人，禁止盲重发。
- 解析要判 `isinstance(d, list)`；id 必须**逐个显式列出**（见坑位表 zsh 条）。

**`mine runs` 单条（任务台，实测 total=230）**：

```json
{"fcRunId":"9635d2fb...","status":"succeeded","routeCode":"06750d36e232",
 "createdAt":"2026-09-12T22:59:46+08:00","finishedAt":"2026-09-12T23:45:59+08:00"}
```

- `fcRunId` 与 `mining runs` 的 `run_id` **是同一个 id**，可交叉回查。
- `status` 见过 `succeeded` / `timeout`（超时按 2 小时档判）。`--status active` 可能混进
  `status:"timeout"` 且 `done:true` 的行——暂停核验，不视为计算已停止，不发新任务或补跑。

**`mining runs` 单条（成果柜，实测）**：

```json
{"run_id":"9635d2fb59b94707a9f1b3590d54401a","route_code":"06750d36e232","route_name":"波动率调整动量",
 "retained":36,"total_candidates":3000,"retain_rate":0.012,"elapsed_s":2356.0,
 "started_at":"2026-09-13T07:01:03+08:00","status":"succeeded"}
```

→ **`retained` = 单次挖掘的有效因子数（终选保留）**，这是本技能早停判据的取数字段。

**`mining overview <run_id>` 结构（实测）**：

```
.kpi.{total_candidates, retained, retain_rate, eliminated, total_evaluations, problem_count, evaluate_methods[]}
.funnel[]                 # [{stage, step, remaining, eliminated}] —— 生成→去重初筛→因子体检→正向测试→终选保留
.elimination_breakdown[]  # [{kind, stage, reason, count}] —— 淘汰原因分解
.problem_groups[]         # [{label, prefix, count}] —— 评估覆盖的问题分组
.route{...}               # 该 run 所属路线的完整定义（可作报告素材）
.run_dir                  # pipeline_results/... （不是可直接读的本地文件）
```

实测漏斗样例：3000 候选 → retained 36（1.2%）；**去重初筛是最大的淘汰环节**（高相关冗余），
说明同路线反复挖的边际收益递减。

⚠️ `agg.best_sharpe`（因子库聚合）是 **16 problem × 4 模型 = 64 次评估里的最大值**，
是多重比较产物，**不能当"因子真实夏普"**。看质量用 `factor summary` 的 `top_factors[].sharpe`。
（本技能默认**不**去拉 `factor list` 全量因子——几十万条，导出报告不需要它。）

## 4. 策略探索（explore · 写）与实验（experiment · 读）

```bash
skz explore start --problem <code> --route <code>   # 写·扣费 → {fcRunId,status}
skz explore get <fcRunId>                           # 单查
skz explore runs [--status <s>] [--page N] [--size M]   # ⚠️ 没有 list 子命令
skz explore poll <id1> <id2> ...                    # 读·批量（≤100）→ 裸数组
skz experiment list                                 # ⚠️ 无分页，一次返回全部（实测 102 条）
skz experiment get <id>                             # ⚠️ 结论字段在顶层 overview 下
skz experiment strategies <id>                      # 只返回【通过】的候选
skz experiment review-matrix <id>                   # 五个分段 × 全部回测的展开行
skz experiment performance-report <id> <code> [--chart-rows] [--from .. --to ..]
```

**`explore poll` 返回（裸数组）**：

```json
[{"fcRunId","status","problemCode","routeCode","statusText","done","ok","percent","resultPath","createdAt","finishedAt"}]
```

- explore **有 `percent`**（0–100），mine 没有。
- `running` 态的元素**只有前 7 个字段**（无 percent/resultPath/finishedAt）→ 解析用 `.get()`，别直接索引。
- 已观察状态包含 `queued/dispatching/running/succeeded/failed`；在途不以 `ok:false` 判失败，终态校验同 mine。
  `active` 别名可能漏 `dispatching`，需全量分页任务台交叉核对，不把该别名当完整枚举。
- `resultPath` 不是本地可读文件；结果仍走 `experiment`。
- **`experiment list` 的 `id` 就是 `explore start` 返回的 `fcRunId`**，可直接 join。

**`experiment list` 单条（实测）**：

```json
{"id":"673afc59...","problem_code":"FTS_PROBLEM_041F82","problem_name":"豆油/棕榈油·日线时序",
 "problem_type":"TimeSeriesProblem","dataset":"future","freq":"日线","symbols_count":2,
 "route":null,"run_at":"2026-09-17T16:04:40+08:00","status":"ok",
 "scanned":7,"passed":0,"failed":7,"skipped":0,"n_backtests":7,"pass_rate":0.0,
 "n_strategies":7,"strategy_count":0,"elapsed_s":0.902,"total_elapsed":176.6,"errors":[]}
```

- ⚠️ **`route` 可能为 `null`**（实测这条就是 null）→ 报告里别拿它当路线名，回退用
  `explore runs` 的 `routeCode` 或本轮自落的 state 映射。
- 通过率**直接用平台的 `pass_rate`，不要自己除**。
- `strategy_count` 只统计尚未登记、仍可评审的候选（promote/删除后会减少）；
- 历史回测总量看 `n_backtests`，不随候选消费改写；
- `passed + failed ≠ n_backtests`（有一批"未评级"策略），**别据此推失败率**；
- 耗时看 `total_elapsed`（探索全流程），不是 `elapsed_s`（只是复审）。

**`experiment get <id>`**：结论字段在**顶层 `overview`** 下——
`json.loads(out)["overview"]["passed"]`。直接取顶层 `passed` 会**静默得 `None`**（表现为"通过率算成 0%"）。
exit 5 / `code=42201` = 数据尚未就绪，稍后重试。

**`review-matrix` 的展开结构**：`items[]` 是 **strategy × segment 展开行**
（如 18 候选 × 5 段 = 90 行），不是嵌套表。用 `defaultdict(dict)` 按 strategy 分组后取
`segment_name == "后置验证"` 的行。行上**没有** `passed`/`verdict` → 候选集合必须先取
`experiment strategies` 的 `code` 集，再筛矩阵。

指标字段**都是中文**：`夏普比率 / 年化收益 / 年化波动率 / 最大回撤 / 卡玛比率 / 交易胜率 /
交易次数 / 持仓K线数`。

**分工铁律**：
- `experiment strategies` 的 `metrics` = **全周期口径**（如 2017-01→2024-12），**不是样本外**。
- 样本外**只能**取 `review-matrix` 的「后置验证」行。
- `metrics.夏普比率`（策略净值夏普）≠ `verdict.history_alpha_sharpe`（超额夏普，平台闸门用它），能差一倍。

## 5. 因子库存与选型（读）

```bash
skz factor summary    # ⭐ 选型首选口径
# → {total_routes, total_factors, deleted_factors, total_evaluations,
#    engine_distribution, route_distribution[], tag_distribution[], generated_at}
# route_distribution[]: {route_code, route_name, engine, factor_count, total,
#                        avg_sharpe, top_factors:[{factor_name, sharpe}]}
```

实测（2026-09-19）：`total_routes` 67 / `total_factors` 200963（约 20 万）。

- `factor_count` = 产能；`top_factors[].sharpe` = 该路线出过的最好因子（**看质量主要看这个**）。
- `avg_sharpe: null` ≠ 质量差：多半是**从未进入评估**的"待变现库存"，这类路线性价比往往最高。
- **`factor_count = 0` 或查不到 → 直接 explore 必然失败**：
  `errorCode: SKZ_LOGIC_ERROR / strategy exploration failed: status=no_valid_factors`。
  新建 route 一定没有因子，必须先挖（`mine start`）。

### ⭐ 推荐探索方向的四步法（场景 B1 用）

1. `skz factor summary` → 路线级产能 + 质量。
2. `skz explore runs --page N --size 100`（翻页覆盖到 total）→ 按 `routeCode` 计数 = 已探索次数。
3. `skz experiment list` → 唯一同时带 `problem_code` + `pass_rate` + `strategy_count` 的记录。
4. 与新问题（`problem get` 的 dataset/freq/symbols_count）求交叉缺口：
   - `因子量高 × 探索次数低` = 库存待变现（最高性价比）；
   - 同 `problem_type`（同 dataset + freq）上 `strategy_count > 0` 的路线 = 已验证可迁移；
   - `pass_rate` 极低且 `strategy_count = 0` 的问题/路线 = 已验证低效，别再投。

⚠️ `explore runs` 的 `problemCode` 按实际响应处理：非空匹配才是归属证据；缺失/null/空值为未知，可按可信 ID poll 补证，仍不足或冲突则暂停。`experiment list` 可补覆盖统计，不能替代回执认领任务。统一规则见 [运行契约](operating-contract.md#研究任务键与可信接管)。

## 6. 钱包（读）

```bash
skz wallet balance                            # 现金 + 额度口袋 + 合计可用
skz wallet costs                              # 当前 CLI 内置价目（mine/explore/refresh 单价）
skz wallet check <mine|explore|refresh> --qty N   # 可负担性检查，触发前必跑
```

费用与余额闸门统一按 [billing.md](billing.md)，不另维护固定金额或历史报价响应。
按已确认轮次与探索组合数量运行当前 `wallet costs` / `wallet check`，展示实际
`unitPriceCent`、`requiredCent`、`availableCent`、`affordable` 和 `shortfallCent`；
`affordable:true` 不是用户许可，查询失败或费用无法核实仍须按共同契约暂停。
`wallet balance` 的可用额是 `totalAvailableCent`（含 `gift_credit` 口袋，注意 `expireAt` 到期日）。
向用户报价时将实时返回的分除以 100 换算成人民币，不套用案例数字。

## 7. 坑位表（都是踩过的）

| 坑 | 现象 | 正确做法 |
|---|---|---|
| **zsh 不做单词分割** | `IDS="a b c"; skz explore poll $IDS` → CLI 收到一个非法 id，**返回 `[]`**，脚本以为"还没开始"一直空转 | id **逐个显式列出**，或用 Python `subprocess` 传 list 参数；zsh 里用 `${=IDS}` |
| **poll 返回裸数组** | `d.get('items')` 对 list 调 `.get` → AttributeError | 一律 `items = d if isinstance(d, list) else d.get('items', [])` |
| **`--status` 写错不报错** | `Active` / `done` → exit 0 + `total:0`，跟"真的没有在跑"分不开 | 在飞集合只用 `active`（小写） |
| **策略面空 ≠ 没资产** | `route adopted` → `[]`、`mine runs` → `total:0`，而 `factor-routes list` / `mining runs` 有货 | 重建位置以**研究面**（`/research/*`）为准 |
| **`experiment get` 少一层** | 取顶层 `passed` 静默得 `None`，通过率算成 0% | 取 `["overview"]["passed"]` |
| **`experiment list` 的 route 可为 null** | 报告里路线列空白 | 回退用本轮 state 里的 routeCode |
| **`mining runs` 无分页** | 传 `--page/--size` 报参数错 | 只有 `--route`；用 `fcRunId == run_id` 精确匹配，时间窗只辅助 |
| **`mine runs` 默认 size=5** | 只看 5 条就以为"没有在跑" | 显式 `--page/--size` 翻页 |
| **`experiment list` 无分页** | 传分页参数报错 | 一次全返回，本地筛 |
| **成果柜缺 run** | 任务台 10 条、成果柜 9 条 | 可能是失败、未就绪或读取缺失；按精确 ID 核验，不能仅用差额推断失败 |
| **`n_backtests ≠ passed + failed`** | 差一批"未评级" | 别当漏算或 bug 追 |
| **写超时≠失败** | 超时后重发 → 双倍扣费 | 回执可信 ID + 任务台/poll 核验；无法证明落库则暂停，不自动重发 |
| **后台停止原因未明** | 收到 killed、本地状态陈旧 | 优先前台收集结果并支持断点续跑；遵循宿主生命周期，不假定有后台完成唤醒 |
| **skz 网络会自愈** | 连续 5 次 90s 超时，几分钟后同命令 4s 返回 | 别一次超时就断言通道挂了 |

## 8. 实测耗时基准（排期用）

| 操作 | 量级 |
|---|---|
| `mine start` 单条 | 中位 **26.5 分钟**；近期集中 **174–206 分钟**；极值 1552/5114 分钟（疑似卡死） |
| 批 10 条 mine | 排队**串行**，≈ 10 × 单条（实测 7 小时跑完 10 条） |
| `explore start` 单条 | 日线 ~5min / 60分钟 ~8min / 15分钟 ~15min（按因子库存放大：11238 因子 ~23min） |
| 批 10 条 explore | **真并发**，20 分钟内全部 succeeded；实测 59 条 39 分钟、58 条 27 分钟 |
| 触发限流 | 分批 ≤8 条、批间 sleep 3s；实测未见 429 |
| 并发触发撞车 | 挖掘逻辑键为 route，探索为 (problem, route)；409 / `exit 7 check_existing` 后停止触发，仅按可信 ID 核对归属并 poll；不猜任务、不重发。服务端实际并发锁粒度待验证，见 [运行契约](operating-contract.md#研究任务键与可信接管) |

## 9. 安全全流程入口（1.1.0）

优先 `bash scripts/run_pipeline.sh`。由 Python argparse 解析全部参数；sh 不拼 CLI 字符串、不后台脱离。
`--help` 无网络调用。未加 `--yes-paid` 时只读预检，可保存本地状态/事件，不做任何云端 write。

### 计划 JSON

场景 A（既有路线；全部值由用户确认，不固定问题数量、轮次、日期或单价）：

```json
{"scenario":"A","route":"<已确认路线code>","problems":["<问题code>","<另一问题code>"],
 "times":3,"threshold":50,"max_total_cent":12000,
 "poll_seconds":60,"max_wait_minutes":1500,"batch_size":8,"theme":"scarlet","title":"本次研究标题"}
```

场景 A 新路线：删掉 `route`，换成 `route_json` 对象；必须完整提供七字段
`name/key_inspect/economic_logic/why_effective/market_mechanism/failure_scenarios/tags`，
只可额外提供 `compute_engine`（共最多 8 字段），不允许其他字段；`describe`/`description` 会被本 driver 的创建校验拒绝。
提交/readback 只比较上述合法提交字段（不填默认、不静默忽略七字段），研究要求放入 `key_inspect`。
已有同机制路线优先复用，不要为了参数变化新建。`times/threshold` 必须先说明并确认。
场景 A 方法建议至少 5 个问题；driver 支持用户明确批准的较小实验，不自动补建问题或扩大费用。

场景 B：`{"scenario":"B","routes":["<已确认路线>"],"problems":["<已确认问题1>","<已确认问题2>"],"max_total_cent":<预算分>}`。
至少一条既有路线 × 至少一个既有问题，每个笛卡尔积组合探索一次；不建路线、不 mine，不接受 A 的挖掘参数。
`preflight.explore_combinations` 明示组合总数。可选 `max_submissions` 必须等于组合数且批准记录同名字段一致，累计尝试不得超限；
可选 `expected_unit_prices:{"explore":<批准单价分>}` 固定本批报价，变化即暂停（降价也不自动提交）。
已确认 problem 必须预先建好；driver 不自动 problem create。预算整数单位为分；实际报价来自动态 `wallet check`。

### 人审记录（不由脚本生成许可）

先跑默认预检。读取 `<workdir>/preflight.json` 的摘要与问题/路线 readback，向用户展示完整计划、
七字段（新路线）、标的/频率/分段、机制去重和动态费用，拿到明确许可之后才保存批准 JSON：

```json
{"approved":true,"authorization_source":"<用户原话或会话确认凭据>","confirmed_at":"<确认时间>",
 "scope_workdir":"<解析后的绝对workdir>","user_id":"<whoami用户标识>",
 "config_digest":"<preflight的config_digest>","selection_digest":"<preflight的selection_digest>",
 "max_total_cent":12000}
```

`selection_digest` 覆盖完整 problem get 与所选路线 readback；重启时内容变化需重新人审。
新建路线额外要求 `route_create_approved:true`、`seven_fields_shown:true`、
`route_json_digest:<preflight同名摘要>`、`mechanism_review:{duplicate_found:false,
 routes_snapshot_digest:<preflight同名摘要>,rationale:<人工机制对比结论>}`。
人工机制去重覆盖当前路线全集，不仅名字或枚举相同就自动判重复；快照变化需重新审查。
sh 只核验人工记录，无法证明用户亲自签署；Agent 严禁在用户未同意时把 approved 改为 true。

实际启动（优先前台阻塞等待；宿主明确保证持久托管/完成唤醒才可受管执行，不加 `&`，不后台启动后退出回合）：

```bash
bash scripts/run_pipeline.sh --config <计划.json> --workdir <独占目录> \
  --yes-paid --approval <批准.json> --budget-cent <同意的cap>
# 新路线另加 --create-route；已建 route 直接复用。
```

同一配置/目录重复调用先核验状态与已有 ID，接管 queued/dispatching/running；不会重发旧任务。
新入口不自动导入旧账本，发现旧账本但无新 driver 状态即暂停；本包不分发旧 orchestrate。
未归属本账本的云端在途任务暂停，而非按时间接近猜它属于本次；人工移交需可靠 ID/审批/费用记录。
工作目录使用 flock 排他，锁失败不杀现有 worker、不删除锁文件。锁范围为单目录，
没有平台级全局原子锁；跨工作目录并发仍存在检查到提交之间的竞态，禁止多个 Agent 并发启动同账号写流水线。

### 独立付费补跑授权

不提供 `--retry-authorization` 即不补跑。明确同意后提供：

```json
{"authorized":true,"authorization_source":"<用户对补跑规则的明确授权>","confirmed_at":"<确认时间>",
 "scope_workdir":"<绝对workdir>","user_id":"<用户标识>","config_digest":"<计划摘要>",
 "max_total_cent":12000,"max_retries_per_logical_task":1,"preserve_original_plan_budget":true,
 "execution_failure_codes":["TASK_EXECUTION_FAILED","EXECUTION_FAILED","SKZ_EXECUTION_ERROR"]}
```

只接受上述已确认的执行失败码子集，不接受泛化 `SKZ_LOGIC_ERROR`。对每个 mine 逻辑轮次与
explore(route,problem) 最多一次；先再次 poll 验证 `failed/done=true/ok=false`，任务台 failed，
同路线无 queued/dispatching/running/timeout。poll done=false、还在途不补跑。
权限/配置/无因子/扣费未知/timeout/未知码不补跑。timeout 即使 done=true 也不等于计算已停止。
写超时或响应缺 ID：先持久化 intent/回执、读任务台；仅原回执有可信 ID 且 poll 逻辑匹配才接管。
缺 ID 无法证明无落库就暂停；云端查空不释放费用，不自动重写。

每次写前动态 `wallet check` 检查余下原计划与拟补跑，预算 = 历史全部保留（失败/未知也算）+
原未提交任务预算 + 本次补跑。不得砍后续任务来挤出补跑预算；低留存正常早停后才释放未挖的未来计划数量。
单价变化或余额不足暂停，cap 不自动抬高。只读调用最多 3 次有限退避；write 传输只有一次。

### 完成与通知契约

状态：`pipeline_state.json`，原始回执 `raw/`，事件 `events.ndjson`，取数 `fetched/summary.json`，报告 `report.html`。
流程必须 await 全部终态，并显式看 ok；读回错误失败关闭，不让 shell `&&` 错接探索。
零库存产出失败报告、`final_completed outcome=failed`、退出 1；暂停退出 2，不假装取消云端任务。
成功退出 0 且 `final_completed outcome=success` 后还需调用方校验 summary、report SHA256 并通过宿主附件/文件展示机制交付。
报告验证包含 doctype/UTF-8、全部章节、目录锚点、尾部、无脚本（本模板纯 HTML/CSS）；
若未来加入 JS，driver 会暂停，必须另做运行时测试才可放行，不能冒称浏览器渲染验证。
事件通知由 Agent 主动读取增量 NDJSON 发给用户；脚本不具备直接聊天或离线 push 能力。
前台收集任务结果；仅在宿主支持持久托管时使用其完成结果机制。跨会话从状态和事件恢复，不假定存在特定 TaskOutput 工具。

验证：`bash -n scripts/run_pipeline.sh`、Python 编译、`python3 -m unittest discover -s scripts -p 'test_pipeline.py' -v`。
模拟测试替换所有 CLI，并默认禁止外部 subprocess；只复用本地纯 Python 报告库，不调用云端 write。

## 10. 接入边界与已知限制

- 历史样例是原作者的观察，不是当前接口保证；命令以当前 CLI --help 与源码为准。价格动态读取，不固定案例单价。
- 只读调用最多重试三次，异常分类较宽；写调用只传输一次。driver 无云端取消、全局账号原子锁或跨目录去重能力。
- 本地状态支持断点续跑，不支持旧账本自动迁移、独立 fetch-only 子命令或单轮 first 接口；不得伪造这些能力。
- 只按精确 ID 抓取实验；清单缺行时尝试 get.overview 的真实同名字段，记录来源，不映射或推算不同统计口径。
- 缺字段、接口读取失败、候选缺后置夏普为 completed_partial，退出 1；显示缺失，不称完整成功，不重发付费任务。
- 后置验证是历史分段，写清实际日期和信息可见性；自然语言路线与高留存不证明因子实现忠实度，无独立消融则不下效果结论。
- 报告不含脚本，结构检查不等于视觉或浏览器验证；事件日志不保证离线推送。
- 配置/审批/原始 JSON 可包含用户资产和账号标识，只保存到独占目录；公开 PR、报告附件与日志摘要不要暴露敏感账号信息。
- 当前 wallet check 的 `pricingSource` 是 `cli`，单价来自 CLI 固定价目，余额来自钱包；这不是服务端实时扣费凭证。升级 CLI 后报价变化须重新确认。
