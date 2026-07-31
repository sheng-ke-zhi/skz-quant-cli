## 技能四册（装不了本套件的 harness：直读正文照做）

- **`skz skills show guide`** — 强引导：从一句想法走到因子/策略（建 route/problem、挖矿、探索、轮询）。
- **`skz skills show factor`** — 因子资产：因子库、某次挖掘的产出、单因子跨时段评估、软删。
- **`skz skills show strategy`** — 策略资产：评审/删除候选、保存入库、实盘富读与状态运营。
- **`skz skills show portfolio`** — 组合资产：把实盘策略按权重打包成组合、查净值/回撤/持仓、再平衡。

安装：`skz skills install [--target claude|codex|openclaw|hermes|all]`（缺省 `claude`；`all` = 本机装了的那些）。装完 `skz skills status` 的 `needs_install:false` 即就绪；**升级二进制后要重装**（`status` 会报 `stale`）。

## 前置：配置 token（一次）

token 不走环境变量，存本地受限权限文件：

```bash
echo "sk_你的key" | skz auth set     # 或让用户在自己终端里跑，key 就不进对话
skz auth status                       # {"present":true}
skz whoami                            # {"user_id":...}：确认 key 活性 + 身份（研究面读，不扣费）
```

没有 token 时，任何联网命令返回 exit 3 + `{"error":{"action":"fix_auth","remediation":{...}}}`——照 `remediation.howTo` 引导用户。研究面读写需要 `research:mining:write`，`/strategy/*` 面（含 `problem create`、`strategy status`）需要 `strategy:write`；**一条完整流程横跨两个 scope**，缺哪个都拿 `INSUFFICIENT_SCOPE`（同样 exit 3）→ 找人换一把双 scope 的 key，别重试。

## 该不该问人（HITL 底表，四册统一）

判据只有两条：**花钱** 或 **不可逆/对已有资产下处置**。命中任一 → **调命令之前**先跟你的人确认。

| 命令 | 判定 | 为什么 |
|---|---|---|
| `mine start` / `explore start` | **必须问人** | 付费触发 |
| `promote start` | **必须问人** | 付费 + 触实盘部署 |
| `strategy status --status 实盘` | **必须问人** | 真金从此刻上场 |
| `strategy status --status 废弃` | **必须问人** | 不可逆：写 `death_time` + 进写保护 |
| `factor delete` | **必须问人** | 对已有资产下逻辑审核判断 |
| `experiment delete` | **必须问人** | 永久删除候选回测产物 |
| `portfolio create` | **必须问人** | 付费触发 FC 组合优化 |
| `strategy register` | **必须问人** | 往实盘库塞**没跑过回测**的资产（连样本内指标都没有）；加 `--realtime` 还额外付费 |
| `route create` / `problem create` | 可自主 | 不花钱、可逆；钱的关卡全在下游 |
| `strategy status --status 暂停` | 可自主 | 降风险、可逆的安全阀 |
| `tag-add` / `tag-rm` | 可自主 | 纯整理 |
| `strategy memo` | 可自主 | 纯整理、免费、可覆盖 |
| `strategy memo --clear` | 可自主 | 同上；但会**抹掉已有笔记且不可恢复**，清之前先 `strategy get` 看一眼当前内容 |
| 一切读命令 | 可自主 | 无代价 |

**CLI 不会拦你**——它是非交互批处理原语，不弹确认、没有 `--yes`。这条契约由你（agent）在对话里执行：**先问人，再调命令**。想要机制兜底的用户，`skz skills permissions` 会打印一份可贴进 harness 权限配置的规则（我们只提供文本，不代改任何配置文件）。

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
- **⚠️ exit 0 = 命令成功，不等于任务成功。** `mine poll` / `explore poll` 读到一个**失败的任务**同样是 exit 0（它成功读到了"失败"这个事实）。**异步任务的成败在 body 的 `ok` 字段里，退出码不承载**——`done:true` 只说"跑完了"，`ok:false` 才是失败。实测见过 `done:true, ok:false, errorCode:"SKZ_LOGIC_ERROR"`。
- **失败**：stderr `{"error":{"kind","action",...}}`，只看 `action` / 退出码决定下一步：

| exit | action | 怎么办 |
|---|---|---|
| 2 | fix_params | 改参数（含 stdin JSON 非法、status 枚举错、fcRunId 超 100 个）重来 |
| 3 | fix_auth | 凭据/权限问题（含 scope 不足），找人（看 remediation） |
| 4 | give_up | 停手交人：配额/IP 超限（当天别试，0 点重置）vs 余额不足（去**前端充值**，`status:402` 带充值 remediation） |
| 5 | retry_later | 限流/临时网络/`5xx`/研究数据未就绪（净值刚建未算完），退避后再来；**但写命令的 5 别盲重试** |
| 6 | internal | 内部/协议错误（含研究后端返回非成功包），上报 |
| 7 | check_existing | **别重发，先查现有状态。** ①触发撞 409「已在跑」→ 去 `mine runs`/`explore runs --status active` 轮询那个 `fcRunId`；②**写超时/连接失败 → 结果未知（可能已落库）**。**带 `remediation.verifyWith` 时一律以它为准**——它是按这条命令定制的验证器；本表这行是泛化说法，对 `route/problem create` 这类没有 `fcRunId` 的写会指错方向 |

- **两个 429 相反**：RATE_LIMITED 可重试、QUOTA_EXCEEDED 不可——工具已按 errorCode 分好，你只认 action。
- **写不自动重试**：创建/触发/保存入库/删除/status/tag/memo 内部**不重试**（无幂等、可能重复扣费/重复处置资产）；读与 `* poll` 幂等，才自动重试。`memo` 虽然是幂等覆盖、重发无害，也照样不重试——「写一律不重试」是一条零例外的规则，例外一开，下一个新写命令就得重新判一次，判错的代价是重复扣费。
- **⚠️ 写命令拿到 exit 7（超时/连接失败）＝ 结果未知,不等于失败。** 请求可能已经到后端并落库了。**铁律：先读回来确认,再决定重不重试**（`remediation.verifyWith` 直接给了该跑哪条读命令）。

  | 超时的写 | 用什么确认它到底成没成 |
  |---|---|
  | `route create` | `skz factor-routes list` 看有没有那条 name |
  | `problem create` | `skz problem list` |
  | `mine start` / `explore start` | `skz mine runs --status active` / `explore runs --status active`（**别直接重触发,会重复扣费**） |
  | `promote start` / `strategy register` | `skz strategy list`（看该 code 有没有进库） |
  | `experiment delete` | `skz experiment strategies <id>`（看该 code 是否仍在候选清单） |
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
