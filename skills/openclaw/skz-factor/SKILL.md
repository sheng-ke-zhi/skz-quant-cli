---
name: skz-factor
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上的因子资产——浏览/筛选/排序因子库、看某次挖掘 run 挖出了什么、查单因子多问题评估、软删不成立的因子。当用户提到「我的因子库」「挖出来的因子」「因子表现/夏普」「这次挖矿结果」「清理/删因子」，或要在挖矿跑完后查看成果时使用。不负责触发挖矿本身（那是 skz-guide）。
---

# skz 技能 · factor（因子管理）

挖矿的**产出**在这册看。两层，别混：

- **成果柜** = 某一次挖掘 run 挖出了哪些因子（`skz mining *`，按 `run_id` 索引）
- **因子库** = 跨所有 run 沉淀下来的全部因子资产（`skz factor *`，按 `factor_name` 索引）

除软删外全是只读。

## 技能四册（装不了本套件的 harness：直读正文照做）

- **`skz skills show guide`** — 强引导：从一句想法走到因子/策略（建 route/problem、挖矿、探索、轮询）。
- **`skz skills show factor`** — 因子资产：因子库、某次挖掘的产出、单因子多问题评估、软删。
- **`skz skills show strategy`** — 策略资产：评审/删除候选、保存入库、实盘富读与状态运营。
- **`skz skills show portfolio`** — 组合资产：把实盘策略按权重打包成组合、查净值/回撤/持仓、再平衡。

安装：`skz skills install [--target claude|codex|openclaw|hermes|all]`（缺省 `claude`；`all` = 本机装了的那些）。装完 `skz skills status` 的 `needs_install:false` 即就绪。

更新：运行 `skz update`。它按当前二进制路径识别 Homebrew、Scoop、uv tool 或 pipx，升级后核对已安装 skill；非交互调用只报告结果，若 `skills.stale` 非空，再运行 `skz skills install --target all` 刷新。

## 前置：确认默认身份

token 不走环境变量，按命名身份存入本地受限权限文件。单账户最小配置：

```bash
echo "sk_你的key" | skz auth add personal --allow-write  # 最好让用户在自己终端里跑
skz auth use personal
skz auth status                 # 确认 active/account/writePolicy/readOnly
skz whoami                      # {"user_id":...}：确认 key 活性 + 身份（研究面读，不扣费）
```

**每条工作流开工前先跑 `skz auth status`。** `active` 是资产、余额和数据归属；不要根据 read/write 策略自行在多个身份之间选择。`readOnly:true` 表示当前身份或整台机器只读，所有写/触发都会直接被拒（exit 8），只有读能用。遇到它就停手交人，不能自行 `auth use` 切到可写身份。

没有 token 或尚未选择默认身份时，联网命令返回 exit 3 + `fix_auth`——照 `remediation.howTo` 引导用户。研究面读写需要 `research:mining:write`，`/strategy/*` 面（含 `problem create`、`strategy status`）需要 `strategy:write`；缺 scope 同样 exit 3，但不得自动换到权限更高的身份，必须交给用户决定。

## 该不该问人（HITL 底表，四册统一）

判据只有两条：**花钱** 或 **不可逆/对已有资产下处置**。命中任一 → **调命令之前**先跟你的人确认。

| 命令 | 判定 | 为什么 |
|---|---|---|
| `mine start` / `explore start` | **必须问人** | 付费触发 |
| `promote start` | **必须问人** | 付费 + 保存入库并预热实时结果；受理后候选会被消费 |
| `strategy status --status 实盘` | **必须问人** | 真金从此刻上场 |
| `strategy status --status 废弃` | **必须问人** | 不可逆：写 `death_time` + 进写保护 |
| `factor delete` | **必须问人** | 对已有资产下逻辑审核判断 |
| `experiment delete` | **必须问人** | 永久删除候选回测产物 |
| `experiment delete-run` | **必须问人** | 永久删掉**整次探索**：候选连同 run 目录一起没，不可恢复。已 promote 进实盘库的策略不受影响 |
| `factor-routes delete` | **必须问人** | 永久删掉一条研究路线，并**级联删掉它名下全部挖掘执行**。路线下的因子会保留但变孤儿（路线名回落显示为 route_code） |
| `factor-routes delete --dry-run` | 可自主，**且该先做** | 零修改，只预告将删几次执行、将留几个孤儿因子。**拿这份数字去问人**，别让人对着一个 code 拍板 |
| 任何删除的 `--force` | **必须问人（第二次）** | `--force` 越过的是后端的软护栏（怀疑还有任务在跑 / 名下还有因子）。撞到 exit 7 之后**不要自己加 --force 重试**——先按 remediation 查证，再带着查证结果重新问人 |
| `gift create` | **必须问人** | **不可撤回地把策略定义交出去**：赠予码就是访问凭证，拿到码的人不需要别的授权就能领走完整定义（`gift revoke` 只挡得住还没领的人）。要跟人确认「给谁、给哪几条、几个人、几天」 |
| `gift claim` | **必须问人** | 往自己实盘库写入最多 10 条别人的策略，落地即在册（只能改状态，删不掉）。要跟人确认这个码的来路 |
| `gift preview` | 可自主，**且该先做** | 零副作用，看清码里有哪几条、是否可领、剩几个名额。**拿这份清单去问人** |
| `gift revoke` / `gift list` | 可自主 | 撤回是收回自己的披露，方向安全、可再发新码；list 是纯读 |
| `portfolio create` | **必须问人** | 付费触发 FC 组合优化 |
| `strategy register` | **必须问人** | 往实盘库批量写入**没跑过回测**的资产（连样本内指标都没有） |
| `route create` / `problem create` | 可自主 | 不花钱、可逆；钱的关卡全在下游 |
| `problem delete` | **必须问人** | 物理删除研究问题，不可恢复；不级联删除既有实验或策略 |
| `strategy status --status 暂停` | 可自主 | 降风险、可逆的安全阀 |
| `tag-add` / `tag-rm` | 可自主 | 纯整理 |
| `strategy memo` | 可自主，**且该主动做** | 免费、可覆盖；平台上唯一的留痕手段（status 不收 `reason`、无 audit log）。**做完判断顺手写一行**，别等人要——写什么、什么时候写见 strategy 册〈做笔记〉 |
| `strategy memo --clear` | 可自主 | 同上；但会**抹掉已有笔记且不可恢复**，清之前先 `strategy get` 看一眼当前内容。**追加笔记不要用它**——先 `get` 读回原文再整体写回 |
| 一切读命令 | 可自主 | 无代价 |

**CLI 不会替你问人**——它是非交互批处理原语，不弹确认、没有 `--yes`。这条契约由你（agent）在对话里执行：**先问人，再调命令**。想要机制兜底的用户，`skz skills permissions` 会打印一份可贴进 harness 权限配置的规则（我们只提供文本，不代改任何配置文件）。

唯一的例外是**只读模式**：机器被设成只读时，上表所有写命令一律 exit 8 直接拒绝。那不是"替你问人"，是"根本不许做"——没有确认环节，也没有开关可以让你自己打开。碰到就停手交人。

## 跟人说话：字段名不出口（四册统一）

**机器层用英文键名，对人一律金融话——这条没有条件。** 别按"对面懂不懂"开关它：专业的人也不想听 `retain_rate`，他想听「三千个候选里留下几个」。英文标识符不是"够专业"，是把实现漏给了用户。方向是**金融话在前，平台词只在真要传参时出现一次**，而不是反过来给平台词加个中文括号。

| 对人说 | 机器层 |
|---|---|
| **3000 个候选里留下 226 个（7%），你之前几次在 2%~13%** | `retain_rate` / `retained` / `total_candidates` |
| 跨多次评估的平均夏普 / 最好的那一次（有挑选偏差，别单看） | `mean_sharpe` / `best_sharpe` |
| 这些评估里有多少次是正的 | `pos_sharpe_ratio` |
| 通过率（几十个回测过一两个是常态） | `pass_rate` |
| 择时（单品种判进出场）/ 选股·轮动（一篮子里比强弱） | `TimeSeriesProblem` / `CrossSectionalProblem` |
| 股票 / ETF / 期货 / 指数 | `stock` / `etf` / `future` / `index` |
| **保存入库**（入库后是暂停态，不会自动交易） | `promote start` |

- **「保存入库」别说成 promote**，那是命令名不是动作名。也别说「毕业」「上线」——**「上线」尤其危险**：入库和真上场是两个决定、两笔风险，用同一个词会让人以为点头一次就完事了。要真上场那步才说「上线」。

- **保留率、通过率三样一起说：数 + 率 + 参照系。** 「保留率 7%」单说出来像个不及格的分数，而实测该账号 11 次挖矿落在 2%~13%，7% 是并列最高的一档。这跟「止损线写成跟谁比、别拍绝对阈值」是同一个动作——被迫说出参照系，就不可能只甩一个孤立数字。分母从 `total_candidates` 读（实测 11 次都是 3000，但那是观测不是承诺），别把三千写死进话术。
- **纯机器层的词根本不出口**：`fcRunId` / 退出码 / `action` / `done` / `ok` / `job_status` / `agg` / `--status active`——它们只在你自己调命令时存在。
- **内部行话同样不出口，而且不用翻译**：漏斗 / 加速器模式 / 领路人模式 / 摊赌注 / 止损线 / 成果柜 / 任务台。它们是中文、听着像人话，所以最容易漏出去——「你今天走完一轮漏斗」要说成「你今天从头跑了一遍：定方向、挖因子、做探索，出了一个候选策略」。
- **用市场现成的叫法**：一个玩法在行业里已经有名字，就用那个（「行业轮动」，而不是「行业主题 ETF 的横截面轮动动量」）。自造名字换不来「这个我试过，不行」这句话。

## I/O 契约（照 action 分支，别解析 message）

- **成功**：stdout 一份紧凑 JSON，exit 0。空结果是成功（`{"total":0,"items":[]}` / `[]`），不是错误。
- **⚠️ 所有分页命令的 `--page-size` / `--size` 缺省只有 5 条**（`symbols`、`mine runs`、`explore runs`、`factor list`、`mining factors`、`strategy list`）。缺省小是有意的——列表项很占上下文，默认全量会把窗口烧光。所以**别把首页当全部**：先看 `total`，要更多就显式加 `--page-size`（上限各端点不同）或翻 `--page`。
- **⚠️ exit 0 = 命令成功，不等于任务成功。** `mine poll` / `explore poll` 读到一个**失败的任务**同样是 exit 0（它成功读到了"失败"这个事实）。**异步任务的成败在 body 的 `ok` 字段里，退出码不承载**——`done:true` 只说"跑完了"，`ok:false` 才是失败。实测见过 `done:true, ok:false, errorCode:"SKZ_LOGIC_ERROR"`。
- **⚠️ exit 0 也可能是"删了一半"。** `factor-routes delete` 会先删路线行、再逐个删名下的挖掘执行；个别执行没删掉时后端仍回 200（路线确实删了，残留只是磁盘垃圾）。**看 `failed_mining_runs`：非空就重发同一条命令续删**（删除幂等，续删不会多删别的）。这是本工具里唯一一处"exit 0 但事情没做完"，别只 branch 退出码。
- **失败**：stderr `{"error":{"kind","action",...}}`，只看 `action` / 退出码决定下一步：

| exit | action | 怎么办 |
|---|---|---|
| 2 | fix_params | 改参数（含 stdin JSON 非法、status 枚举错、fcRunId 超 100 个）重来 |
| 3 | fix_auth | 凭据/权限问题（含 scope 不足），找人（看 remediation） |
| 4 | give_up | 停手交人：配额/IP 超限（当天别试，0 点重置）vs 余额不足（去**前端充值**，`status:402` 带充值 remediation） |
| 5 | retry_later | 限流/临时网络/`5xx`/研究数据未就绪（净值刚建未算完）/工作区正在初始化（`40909`），退避后再来；**但写命令的 5 别盲重试** |
| 6 | internal | 内部/协议错误（含研究后端返回非成功包），上报 |
| 7 | check_existing | **别重发，先查现有状态。** ①触发撞 409「已在跑」→ 去 `mine runs`/`explore runs --status active` 轮询那个 `fcRunId`；②**写超时/连接失败 → 结果未知（可能已落库）**；③**删除撞软护栏**（`remediation` 里点名 `--force` 的那种）→ 先按它查证，再**回去问人**要不要 force，别自己加。**带 `remediation` 时一律以它为准**——它是按这条命令定制的；本表这行是泛化说法，对 `route/problem create` 这类没有 `fcRunId` 的写会指错方向 |
| 8 | not_permitted | **这台机器禁止写操作（只读模式），请求根本没发出。停手交给人**，别找别的路（换写法、改环境变量、绕开本工具直接访问平台都算绕过）。重试没意义；这**不是** key 权限问题，换 key 也没用 |

- **两个 429 相反**：RATE_LIMITED 可重试、QUOTA_EXCEEDED 不可——工具已按 errorCode 分好，你只认 action。
- **`40909` 不是普通冲突**：它只表示工作区初始化尚未结束，CLI 把它归为 `retry_later`；读命令会有限重试，写命令仍不会自动重放。
- **写不自动重试**：创建/触发/保存入库/删除/status/tag/memo 内部**不重试**（无幂等、可能重复扣费/重复处置资产）；读与 `* poll` 幂等，才自动重试。`memo` 虽然是幂等覆盖、重发无害，也照样不重试——「写一律不重试」是一条零例外的规则，例外一开，下一个新写命令就得重新判一次，判错的代价是重复扣费。
- **⚠️ 写命令拿到 exit 7（超时/连接失败）＝ 结果未知,不等于失败。** 请求可能已经到后端并落库了。**铁律：先读回来确认,再决定重不重试**（`remediation.verifyWith` 直接给了该跑哪条读命令）。

  | 超时的写 | 用什么确认它到底成没成 |
  |---|---|
  | `route create` | `skz factor-routes list` 看有没有那条 name |
  | `problem create` | `skz problem list` |
  | `problem delete` | `skz problem get <code>`（404 表示已删；仍可读表示未删） |
  | `mine start` / `explore start` | `skz mine runs --status active` / `explore runs --status active`（**别直接重触发,会重复扣费**） |
  | `promote start` | `skz strategy list`（看该 code 有没有进库）+ 已拿到 `promotion_id` 时用 `skz promote get <promotion_id>` 查任务；候选消失是受理后的正常结果 |
  | `strategy register` | `skz strategy list`（看该 code 有没有进库） |
  | `experiment delete` | `skz experiment strategies <id>`（看该 code 是否仍在候选清单） |
  | `experiment delete-run` | `skz experiment list`（看该 id 是否还在） |
  | `factor-routes delete` | `skz factor-routes list`（看该 code 是否还在）+ `skz mining runs --route <code>`（看执行清干净没） |
  | `gift create` / `gift revoke` | `skz gift list`（看那个码在不在；`create` 超时时码可能已生成并已可被领取） |
  | `gift claim` | `skz gift preview <gift_code>` 看 `already_claimed`（比翻策略库准——撞名时落地编号会带 `_G{n}` 后缀，照原编号找会找不到） |
  | `strategy status` / `tag-add` / `memo` | `skz strategy get <code>` 看当前 status / tags / memo |
  | `portfolio create` | `skz portfolio list`（看该 code 的 `job_status`；**别用 `portfolio get`**，生成中/失败它一律 404） |

  确认**没写进去**才可以重来一次；确认**写进去了**就往下走，别重复触发。

  **重来最多一次，且花钱的写（`mine/explore start`、`promote start`、`portfolio create`）重来前要再问一次人**——超时通常是平台侧持续故障（实测同一条写连撞两次 30s 超时），第三次只会再烧一次钱、不会有新信息。连撞两次就停手，把 `message` 原文交给人。
- **钱不在开放面**：余额不足（402/give_up）无法在 CLI 内自愈——引导**用户去前端充值**，别自动重试触发。
- **时间戳是东八区，日期不是时间戳**：`create_time`/`created_at`/`started_at`/`finished_at`/`run_at`/`update_time`/`last_heartbeat`/`generated_at` 这类**事件时刻**，工具已把后端的 UTC 换算成东八区、并带 `+08:00` 后缀（`2026-07-26T01:20:41+08:00`）——直接当北京时间读，别再自己加 8 小时。而 `cal_date`/`dates`/`rebalance_dates`/`sdt`/`edt`/`dt`/`latest_weight_date`/`outsample_sdt` 是**交易日/区间边界**，原样不动（移一天就换了一个交易日）。
- **⚠️ `trades` 等松散块里的时间不换算**，尤其 **`kline_key`（形如 `601688.SH|2016-09-28T16:00:00|2016-11-11T16:00:00`）必须原样喂给 `strategy kline`**——它是路径参数，改一个字符就查不到那根 K 线。
- **分页自己驱动**：读 `total` 决定翻不翻页，没有 `--all`。
- **投影字段用管道**：`skz factor list | jq '.items[] | .factor_name'`。
- **中文键在 jq 里必须用 bracket 记法**：指标键几乎都是中文（`夏普比率`/`年化收益`/`最大回撤`），`jq '.夏普比率'` 或 `jq '{夏普比率}'` 会直接报 `INVALID_CHARACTER`。写 **`jq '.["夏普比率"]'`**。这个坑实测有 agent 踩两次，别靠记性。
- **先看一眼真实 schema,别猜字段名**：`skz <cmd> | jq '.items[0]'`。猜错字段名**不报错**——`jq` 给 `null`、静默算出错答案（实测有 agent 因此把通过率算错）。这是本套件里最容易被忽略、后果最大的失败模式。
- **枚举/筛选值写错也多半不报错,只回空**（exit 0 + `items:[]`），跟"真的没有"分不开。拿到空结果先怀疑自己的参数值，再下结论。
- **每日 5 个 IP 上限**：别在会变 IP 的环境反复跑。

`skz --version` 出 `{"cli","contract"}`；`skz --help` 及各子命令 `--help` 是完整用法参考。

## 因子库：跨 run 的因子资产

```bash
skz factor summary                           # 概览：总数/已删/评估数 + 引擎、路线、标签分布（含各路线 TOP 因子）
                                             # 注意 top_factors[].sharpe 其实是 mean_sharpe（实测核对过），不是 best
                                             # total_routes 可能大于 route_distribution 的元素数——差额是
                                             # 「已建但还没挖出因子」的路线（刚建的 route），不是字段错乱
skz factor list \
  [--q 关键词] [--route <code>] [--engine <e>] [--tag <t>] \
  [--sort 夏普比率] [--order desc] [--include-deleted] [--page 1] [--page-size 5]
                                             # ⚠️ page-size 缺省 5（省上下文），最大 200——先看 total 再决定加多少
skz factor get <factor_name>                 # 详情：factor_code 表达式 + tags(含分段 QC 明细) + evaluations
skz factor-routes list                       # 因子路线（挖矿方向）清单，供 --route 取 code
skz factor-routes delete <code> --dry-run    # 删路线前的零修改预演（写侧，见〈删研究路线〉）
```

`factor list` 一条 item 长这样（真实字段）：

```json
{"factor_name":"TSA_260718_O599IPR2",
 "factor_code":"ma = Mean($close, 120)\nbias = Div(Sub($close, ma), Add(ma, 1e-6))\n…",
 "compute_engine":"TSA","engine_full":"TimeSeriesAstEngine","description":"乖离…",
 "agg":{"best_sharpe":0.957,"mean_sharpe":0.155,"median_sharpe":0.090,
        "median_calmar":0.040,"problem_count":16,
        "pos_sharpe_ratio":0.625,"best_problem":"FTS_PROBLEM_D_xxxxxxxx"}}
```

**`problem_count` 是去重后的问题数**，不是评估行数；同一问题使用多个方法时，评估总数会更大。判断稳定性时仍要同时看问题与方法，别把小幅差异过度解读成不稳定。

> **`best_problem` / `evaluations[].problem` 里那种 `FTS_PROBLEM_D_xxxxxxxx` 编码查不到**：它们是挖矿内部做跨问题验证的**基准问题**，跟 `problem list` 里你自己建的研究问题（`STS_QS_LEADERS` 这类）是**两套命名空间**。拿它去 `skz problem get` 必回 **404 / exit 2 fix_params**——那不是你参数写错，是这套编码根本没暴露查询入口，**别在这上面反复试**。只能从前缀认品类：`F`=期货 / `S`=股票 / `E`=ETF。

- **`metrics` 只包含后端预计算的夏普与卡玛指标**，不是某次评估，也不是完整 `agg` 的别名。单次评估只在 `factor get.evaluations[]` 里；CLI 对 `metrics` 原样透传。
- `factor list` 与 `mining factors` 的 `agg` 都提供 `best_problem`、`pos_sharpe_ratio` 和 `median_calmar`，不必为了补字段切换端点。
- `--sort` 收中文指标键（`夏普比率`）或字段名（`best_sharpe`）；`--order` 缺省 `desc`；`--page-size ≤ 200`。
- **别只看 `best_sharpe`**：它是该因子在多次评估里的**峰值单次**，天然有挑选偏差——`best_problem` 也只是"峰值出自哪个问题"，**不代表它在那个问题上稳定地好**。判稳定性看 `mean_sharpe` / `median_sharpe` / `pos_sharpe_ratio`（正夏普占比）。

**`factor get` 没有 `agg`**——它的 `evaluations[]` 每条只有 `problem/method/status/sharpe/calmar`。要 mean/median/正比例得自己从这些评估算，别指望它回 `factor list` 里那份摘要。另外 agg 只有 `best_problem`、**没有对称的 `worst_problem`**，最差情况得自己扫 evaluations。

`factor get` 的 **`tags` 里藏着平台自己的分段质检明细**（`detect_passed`/`positive_passed`，含各训练段与人类可读原因）——判稳健性很好用的第二意见。`evaluations[]` 已不含 `segments`，分段信息只能从 `tags[].detail` 获取。但有三个坑：

1. **粗粒度 tag 名本身没有区分度**：`factor summary.tag_distribution` 里 `detect_passed`/`positive_passed` 都是 1965/1965（100%）——**那只是"每个因子都挂着这两个 tag"，不是"100% 通过质检"**。质检好坏全在 `tags[].detail` 里。
2. **`detail` 是 JSON 字符串,要二次 `json.loads`**，不是嵌套对象。真实结构（实测）：
   ```
   tags[] = [{"tag":"detect_passed",  "detail":"{\"problems\":[…],\"checks_passed\":[…]}"},
             {"tag":"positive_passed","detail":"{\"methods\":{\"ratio_train_a\":{\"passed\":false,\"reason\":\"…0.281 <= 0.618\"},
                                                              \"ratio_train_full\":{…}, \"neutrality\":{…}}}"}]
   ```
   取质检结论要走 **`positive_passed` → `detail`(loads) → `methods` → `ratio_train_full` → `passed`/`reason`**。
   `reason` 是现成的人话（`"训练集- 正收益比例 0.508 <= 0.618"`），直接可以引给人看。
3. **⚠️ 六个子检查里通常只有 `ratio_train_full`（全周期正收益占比，阈值 0.618）有区分度。** 另外五个多半**饱和**：`ratio_train_b`/`neutrality`/`tsa_hold_monotonic` 几乎全过、`ratio_train_a`/`ratio_train_c` 几乎全不过（**A/C 段弱是全库共性**，把它当"这批因子特有的问题"是过度归因）。饱和的检查携带零信息。
   **而且 `ratio_train_full` 更像"极端尾部的绊线"而非质量梯度**：实测某批 17 个样本里它只对最差的 3 个报警，对另外 8 个轻度为负的因子全程沉默。所以它**判不出"中等偏差"**，只抓最坏的那几个。
   **方法：每次都自己现算，别套用别人的数字。** 六个子检查哪个饱和、哪个有区分度**随批次变化**——拉一组 `mean_sharpe` 中位数附近的因子当**参照系**，跟目标组逐检查对比通过率，看谁真的分得开。（我这里写的比例是某次实测的一个样本，不是常量。）

## 成果柜：某次挖掘挖出了什么

`skz mine start` 拿到 `fcRunId`、`skz mine poll` 到 **`ok:true`**（不是只看 `done`）后，**用同一个 id 当 `run_id`**：

```bash
skz mining runs [--route <routeCode>]        # 该账号所有挖掘 run（含 retained/retain_rate/elapsed_s/派生 status）
skz mining overview <run_id>                 # 漏斗/KPI：total_candidates→retained、淘汰分解、problem 分组
skz mining factors <run_id> \
  [--q k] [--route/--engine/--tag ..] [--sort best_sharpe] [--order desc] \
  [--pos-min 0.5] [--page 1] [--page-size 5]
                                             # ⚠️ page-size 缺省 5，最大 100
```

`mining overview.kpi.evaluate_methods` 是本次使用的评估方法数组；即使只有一种方法也保持数组形状。

> **`run_id` 的两种形态**：新挖的 run，成果柜里的 `run_id` **就是 `fcRunId` 本身**（32 位 hex，如 `ad3907d6c59b43c4be7a29546c978335`）；早期 run 是 `<route>_<n>_<日期>_<时间>` 格式。两种都能直接喂给 `mining overview/factors`，**别去拼格式**——从 `mining runs` 拿现成的 `run_id`。

> **⚠️ `mine` 和 `mining` 是两块完全不同的后端，名字却几乎撞车——这是最容易叉出去的地方。**
> `skz mine *`（动词，`/strategy/miner/*`）= **任务台**：`mine start` 触发挖矿（扣费）、`mine runs`/`mine poll` 看**进度**。
> `skz mining *`（动名词，`/research/mining/*`）= **成果柜**：`mining runs`/`overview`/`factors` 看**挖出了什么**，只有这边带 `agg` 跨问题统计。
> 陷阱：猜着用 `mine runs` 会拿到一个**看起来完全合理**的运行列表（不报错），但那条路上没有 `overview`/`factors` 这层数据，很容易拿着错的列表还不自知。**要看成果，一律走 `mining`。**

**两个实测过的坑：**

1. **分页三个端点三种上限**：`factor list --page-size` 最大 200；`mining factors --page-size` 最大 100，CLI 会在请求前拦下 101 以上；`mining runs` 没有分页 flag，它无条件全量返回。**两边缺省都只有 5 条**（省上下文），要多少自己传。**永远比对 `len(items)` 与 `total`**，不等就翻页。
2. **`--group` 按 run 动态校验**：CLI 会先读该 run 的 `overview.problem_groups[].prefix`，无效值立即 `fix_params` 并列出可选值，不再把参数错伪装成空结果。

另外只有 `--pos-min`、**没有 `--pos-max`**，所以"捞最不稳的那批"没法直接用阈值筛，得靠 `--sort pos_sharpe_ratio --order asc` 从头拿。

`mining runs` 的 `retain_rate` 是这次挖矿的信噪比（例：3000 候选 → 121 保留 = 4%）。`mining overview` 的 `elimination_breakdown` 会告诉你**因为什么被淘汰**（高相关、样本不足…），那是判断"这条路线值不值得再挖一次"的主要依据。

> **「上次挖矿」往往不是一个 run。** 实际常见的是一批：同一波里 3 条 route × 3 个 run = 9 个 `run_id` 连着跑几小时。**CLI 没有"上一批"的概念**，`mining runs` 只给你一列 run（带 `started_at`），要按批看就自己照时间聚。
> 别偷懒用「`factor summary.total_factors` == 这批 retained 之和」来认定"库 = 这批"——**只有账号至今只挖过这一批时才成立**，第二批一来这个等式就悄悄失效。

## 软删（写 · 必须先问人）

```bash
skz factor delete <factor_name> --reason "逻辑不成立：与已有动量因子高度共线"
```

**⚠️ HITL：调它之前先跟你的人确认。** 判据是「对已有资产下逻辑审核判断」——它不花钱，但它是一个**判断**，不是整理。写命令**不自动重试**。

**真机验过的行为**（`{"factor_name":…,"is_deleted":true}` + exit 0）：

- `factor summary` 的 `total_factors` **减 1**、`deleted_factors` **加 1**。
- 默认 `factor list` **查不到**它了；加 `--include-deleted` 就**看得回来**（带 `is_deleted:true`）。
- **`factor get <已删因子>` 仍然正常返回**（不是 404），且 `delete_reason` 字段**完整保留你写的理由**。

所以 `--reason` 要写成**能让后来人复核的证据**，而不是「效果不好」四个字——把判据的数值和参照系写进去（见下面的典型任务）。软删是**可逆的逻辑审核动作**，不是物理删除。

## 删研究路线（写 · 物理删 · 必须先问人）

```bash
skz factor-routes delete <route_code> --dry-run   # 0. 先预演：零修改，只报数
skz factor-routes delete <route_code>             # 1. 问过人之后再真删
```

**跟因子软删是两回事**：因子软删可逆、`factor get` 还查得回来；**路线删是物理删，并且级联删掉这条路线名下的全部挖掘执行**（那些 run 的漏斗、参数、挖出的中间产物都没了）。

- **因子不级联删**。路线下的因子会保留，但变成孤儿：此后它们的路线名回落显示成 route_code。`--dry-run` 的 `orphaned_factors` 就是这批因子的数量（含已软删的）。
- **先 `--dry-run` 再问人**。它零修改、不花钱、只读模式下也能用，会告诉你「将删几次执行、将留几个孤儿因子」。**拿这两个数字去问人**——让人对着一个 route_code 拍板等于没问。注意它**不绕过护栏**：护栏没过时预演一样报 exit 7，这反而是好事，你能在动手前就知道自己需要 `--force`。
- **两条软护栏，共用一个 `--force`**：「名下还有因子」、「执行目录最近仍有写入」。这个端点**没有硬拒绝那一级**——后端不触发挖矿，没有权威运行态可查，所以两条都是它的启发式怀疑。
- **撞到 exit 7 别自己加 `--force`**：先按 `remediation` 查证（`skz mining runs --route <code>` 看那些执行是不是真的还在跑），带着查证结果**再问一次人**。
- **exit 0 也可能删了一半**：看 `failed_mining_runs`。非空表示路线行已删、个别执行目录没清掉，**重发同一条命令续删**即可（删除幂等）。

打标签属于整理，**可自主**：

```bash
skz strategy tag-add / tag-rm …              # 因子侧标签见 factor get 的 tags 字段
```

## 一个典型任务（照着改）

「挖矿跑完了，帮我看看这次挖出的东西值不值得留」：

```bash
skz mining runs                                   # 1. 找最近的 run_id（含 retained/retain_rate）
skz mining overview <run_id>                      # 2. 漏斗：淘汰在哪一环、保留率正不正常

# 3. 找最不稳的尾部——用后端排序，别自己拉全量回来本地排
skz mining factors <run_id> --sort pos_sharpe_ratio --order asc --page-size 20
#    要全量时记得比对 total 与 len(items)，>100 得翻页

skz factor get <factor_name>                      # 4. factor_code 表达式 + tags 里的平台质检 + evaluations
# 5. 判据（几条同时成立才算"该删"，别单条定罪）：
#    · mean_sharpe 与 median_sharpe **都为负**（不是偶尔差，是中心趋势就不行）
#    · pos_sharpe_ratio 明显偏低（正夏普占比小；factor list/mining factors 的 agg 都有）
#    · tags 里平台质检的 ratio_train_full 没过（阈值 0.618；比自己拍阈值可靠）
#    再回到 factor_code 看逻辑站不站得住——数据难看 + 逻辑也说不通 → 才提议删
# 6. ⚠️ 列出短名单和理由给人，问过之后再 delete；别自己删
```

**参照系比阈值重要**：先看一眼头部因子（`--sort pos_sharpe_ratio --order desc`）长什么样，再判断尾部差到什么程度算异常。硬套一个绝对阈值容易在不同 route 上失准。

> **⚠️ 别拿 `best − mean` 差距当过拟合证据**（两次独立实测都否掉了它）：
> - 它会结构性地偏大（`best` 是多次评估里的峰值），单独看区分不出稳定性。
> - 更要命的是**方向会骗你**：差距小往往不是"更少挑肥拣瘦"，而是**这批的天花板更低**（`best` 拉不上去）。把它读成利好正好读反。
> - `evaluations[]` 不提供分段序列，所以这个差距只反映多问题、多方法评估的离散度，不能拿来推断"训练 vs 样本外"的泛化差距；分段质检只看 `tags[].detail`。
> 判过拟合要用 `median_sharpe` / `pos_sharpe_ratio` / QC 的 `ratio_train_full`。

## 常见状态

- **新库/刚挖完索引未生成** → 研究面可能回 **exit 5 retry_later**（净值/统计还没算完），退避再来，别当失败。
- **`factor get` 一个不存在的名字** → **exit 2 fix_params**（改 name，别重试）。
- **空结果不是错误**：`{"total":0,"items":[]}` + exit 0 意味着这个账号确实还没有因子——去 `skz skills guide` 先挖。
