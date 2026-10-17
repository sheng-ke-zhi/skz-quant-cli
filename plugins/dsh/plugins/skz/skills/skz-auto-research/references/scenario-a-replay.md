# 场景 A · 交易经验 / 交易策略复现

**输入**：一段交易经验或一个已有策略（可能是别人写的、或用户自己总结的规律）。
**产出**：一条因子路线 + 若干次挖掘结果 + 在已确认研究问题上的探索结果 + 一份 HTML 汇总报告。

建议至少 5 个同类问题、10 轮挖掘、retained 阈值 50；均为研究建议，不是平台硬限制。
用户可明确批准较小实验，以已确认的 `problems`、`times`、`threshold` 和预算为准，
不因数量不足阻断，不自动补建问题或增加轮次。下文历史数字仅用于解释，不是当前平台保证。

```
A0 前置检查(只读)
 → A1 选定研究问题                [用户确认]  ← 新建按 create-problem（HITL），不为凑数建
 → A2 把交易经验写成因子路线 route  [用户确认]  ← 七字段完整展示后建
 → A3 串行挖掘至批准轮次，低于批准的 retained 阈值即停（自动段）
 → A4 在选定的每个问题上发起 explore [付费许可]
 → A5 抓取 → HTML 报告 → 指到策略审核页
```

**两条硬闸门**：A2 建路线前展示七字段；A3/A4 付费前先 `wallet check` + 三句话说明 + 拿许可。
**执行优先脚本**：A1/A2 人审完成后用 `scripts/run_pipeline.sh` 贯通 A3→A4→A5。
参数、审批记录、预算及可选一次补跑规则见 [cli-contract.md](cli-contract.md) 第 9 节。
既有同机制 route 直接续跑；仅已确认新 route_json 才加 --create-route，七字段/readback 不能静默放宽。
默认预检不付费；不分发旧 orchestrate，不把下面教学命令拼成无人审的 shell 链。

**一个自动段**：A3 一旦启动就自己判阈值、自己决定要不要继续，**不要每轮都来问用户"还挖吗"**——
以已确认的轮次和早停阈值为限，不能把模糊意图当成无限付费许可。
但要**在每轮结束后给他一行进度**（第 k/批准轮次、本次留存多少、是否继续）。

---

## A0 前置检查（只读，30 秒）

```bash
skz auth status                   # 本地身份与只读策略
skz whoami                        # 身份
skz wallet balance                # 余额快照；费用与可负担性另按已确认数量动态检查
skz mine runs --status active     # 有在跑的挖掘 → 先核对可信 ID 和路线，别自动接管或重复触发
skz explore runs --status active
skz problem list                  # 现有研究问题（A1 的候选池）
skz factor-routes list            # 现有路线（A2 去重用）
```

把结果压成 4 行给人看：身份 OK / 余额 X（够 Y 次挖掘 + Z 次探索）/ 在途任务 / 现有问题数 & 路线数。

---

## A1 选定研究问题（数量由用户确认）

### 怎么筛

拿交易经验里的**品种 + 频率**当筛子，从 `problem list` 里过滤：

| 交易经验的属性 | 对应的问题字段 |
|---|---|
| 品种（股票 / ETF / 期货，哪个板块） | `dataset` = `stock` / `etf` / `future` |
| 交易周期（短线/日内/波段） | `freq` = `日线` / `60分钟` / `15分钟` / … |
| 标的集合与相关性强弱 | `symbols[]`（时序一般建议 ≤10 个且高相关，不是平台硬上限） |
| 策略形态（择时 / 多空） | `problem_type` = `TimeSeriesProblem` |

**筛选原则**：

1. **同 dataset + 同 freq 优先**——只有同族问题的结果才可比，跨频率混在一张报告里没法横向读。
2. **时序一般建议 ≤10 个且高相关，不是平台硬限制**。平台硬上限与定义由 `skz-create-problem` 主讲，合法枚举和时间边界以当前 `problem meta` 为准。根据机制、周期与样本长度说明适配性，不为凑数自动扩池。
3. **优先复用**账号里已有的问题（复用 = 免费 + 已有历史实验可做基准）。
4. 建议至少 5 个同类问题，但用户可明确批准较小实验；每多一个问题就多一次 explore 扣费，先跟用户对齐 N 和预算。

### 给人看的形状（别丢一串 code）

```
本次拟在 6 个研究问题上复现「焦煤日线趋势中继」：
① 焦煤·日线时序        FTS_PROBLEM_XXXX   1 标的  日线    [已探 3 次 / 曾出 1 个候选]
② 黑色系三品种·日线     FTS_PROBLEM_YYYY   3 标的  日线    [从未探索]
...
共 6 个问题 → 6 次探索（约 ¥X）
```

⚠️ 「已探 / 曾出候选」这两列来自 `skz experiment list`（按 `problem_code` 聚合
`pass_rate` / `strategy_count`），有它才能判断这问题值不值得再投。

### 确需新建问题时（HITL）

切换 `skz-create-problem`，由它负责当前 `problem meta`、标的核对、完整定义与创建后的读回。
创建前完整展示待建问题并取得明确许可；内容修改后重新展示确认，用户说“直接建”也不能跳过。
本案例不另维护问题 JSON 或历史日期模板；问题建好后再确认本轮计划，driver 不自动创建问题。

---

## A2 把交易经验写成因子路线（HITL）

### 从经验里提取七字段

| 字段 | 从用户的哪句话来 | 写不好的后果 |
|---|---|---|
| `name` | 用**市场现成的叫法**（"放量突破后的短期动量"） | 自造名字换不来"这个我试过" |
| `key_inspect` | **看什么**：具体到参数与算子链 | 抽象描述 → 引擎按形状瞎猜 |
| `economic_logic` | **这钱是谁亏的**（现象为什么存在） | 没有对手盘 = 在训练集里找巧合 |
| `why_effective` | **为什么还没被赚光**（套利为什么没抹平） | 只答前半 = 方向糊弄 |
| `market_mechanism` | 十个枚举值里挑一个，逐字照抄 | 自创值会被后端拒或丢弃 |
| `failure_scenarios[]` | **什么时候失效**（绑定本组标的的真实弱点） | 写"可能失效"= 等于没写 |
| `tags[]` | 决策/过滤类关键词 | 影响可检索性 |

`key_inspect` 写法（用户偏好）：**全中文自然语言**，先给所依据的定义/底层逻辑，再给算子链；
用方括号小标题分段（【结构层】【力度层】【环境层】【输出】）；参数写成"5日均量线上穿20日均量线"这种口吻。

### 去重（按机制，不是按名字）

```bash
skz factor-routes list | jq -r '.items[] | "\(.market_mechanism)\t\(.code)\t\(.name)"' | sort
```

同机制下已有路线 → 先问自己"看的东西是否真的不同"；只是周期/参数不同 → **别新建，拿老 code 直接挖**。

### 展示 → 确认 → 创建

```
路线：放量突破后的短期动量
看什么       突破当日的量能确认：5 日均量上穿 20 日均量、且当日量 > 20 日 90 分位...
钱从哪来     散户看到突破才追进来，把价格推过头；他们不知道量能已经透支
为什么还在   追涨的资金源源不断（新散户进场），而机构不会为了这点价差去反向套利
哪类机制     行为偏差
什么时候失效 震荡市里的假突破（放量后 3 日内跌回突破点）；或全市场缩量，量能信号失真
```

确认后：

```bash
skz route create < route.json     # → {routeCode}，务必落盘
```

⚠️ 创建前逐字核对七个核心字段；driver 至多另允许可选 `compute_engine`，不是 9 个字段。
路线内容需纠正时暂停并交给用户决定；不得自动删除重建，删除属于 `skz-factor` 的独立确认边界。
⚠️ 新建的 route **因子数为 0**，必须先挖才能 explore。

---

## A3 按批准轮次串行挖掘（自动段 · 核心）

### 为什么串行

本 driver 不调用取消接口，不能假定已提交的云端任务可撤回或退款。
“发现低产就停止后续”靠“一次只在途 1 条”；串行是费用与早停边界，不保证服务端吞吐。

### 状态机

```
for round in 1..times（已批准轮次）:
    ① wallet check mine --qty 1（余额不足 → 停手问人）
    ② 先 fsync 写前 intent/费用保留，再 mine start --route R → fcRunId，立即落盘响应
    ③ 轮询 mine poll <fcRunId>（60s/轮，判 ok）
    ④ 取 retained（见下"取数"）
    ⑤ 判定：
        succeeded/done=true/ok=true 且 retained >= threshold → 批准轮次内继续
        succeeded/done=true/ok=true 且 retained < threshold  → 停止本批次，带已挖到的因子进 A4
        failed/done=true/ok=false → 默认暂停；单独授权且白名单执行失败才核验后补跑一次
        done=false / timeout / 长时间 queued 不动 → 不补跑；窗口耗尽暂停（不等于停止云端计算）
```

每轮结束给人一行（以下为假设批准 10 轮、阈值 50 的历史数字示例，不用于覆盖实际计划）：

```
第 3/10 轮完成：3000 候选 → 留存 118（3.9%），高于阈值 50，继续第 4 轮。
```

### 取数：怎么拿到"有效因子数"

```bash
skz mining runs --route <routeCode>     # ⚠️ 无分页，只有 --route
```

- 取本批次的 run：必须按账本 `fcRunId == run_id` 精确匹配；时间窗仅辅助展示，
  不能用于接管 ID、结算单轮 retained 或核验未知提交。
- `retained` = 有效因子数（终选保留）。与 `skz mining overview <run_id>` 的 `kpi.retained` 同源，
  需要漏斗细节时再拉 overview（报告里要用）。

### 早停阈值的校准（50 仅为示例，不是普适真理）

```bash
skz mine runs --size 50      # 任务台：看本账号历史的 status 分布（含失败/超时）
skz mining runs              # 成果柜：看本账号历史的 retained / retain_rate 分布
```

历史样例：3000 候选 → retained 36（1.2%）、另一路线 2482 → 206（8.3%）。
早停阈值用于限制后续低产轮次的投入，不是因子质量门槛；50 不能无条件套用于当前账户。
如果本账号常态留存就该是 100+，把阈值调高；如果常态是 20–40，阈值 50 会让第 1 轮就停——
**这件事必须在开始前跟用户确认一句**，别闷头按 50 跑完发现第一轮就停。

### 耗时与轮询

- 历史观察单条中位 ~26 分钟、某批 174–206 分钟，不保证当前耗时；按已批准轮次估计排期，可能跨天。
- 轮询窗口可按已确认排期配置到 1500 分钟；优先前台等待，受管任务仅在宿主保证持久托管与完成唤醒时使用。
- 脚本自身做断点续跑：独占目录 `pipeline_state.json` 保存每轮及 attempts 中的云端 ID、回执与费用，
  重启先核验并恢复，**已触发的轮次绝不重发**。

---

## A4 在选定的研究问题上批量探索

```bash
skz factor summary                                  # ⚠️ 确认该 route 的 factor_count > 0
skz wallet check explore --qty <N>                  # N = 问题数
# 教学命令；实际执行用 driver 分批 ≤8，在 pipeline_state.json 中落盘 intent 与 ID
skz explore start --problem <code_i> --route <routeCode>
skz explore poll <id1> <id2> ...                    # 60s/轮，≤100 id
```

- **探索排期**：历史观察为 10 条约 20 分钟完成，不构成并发能力或锁粒度保证；按实际状态跟进，不保证不同键一定可并发。
- ⚠️ 因子库存为 0 → 必失败（`no_valid_factors`）。**先查再投。**
- ⚠️ 去重与接管：挖掘按路线，探索按 `(problemCode, routeCode)` 识别逻辑任务。用户重复下指令时只用可信回执/账本 ID 查任务台并 poll，核对问题与路线；缺字段先按可信 ID 补证，错配或证据不足则暂停。`createdAt` 仅供展示和辅助排查，时间相近不能证明本次提交已受理；无可信 ID 不自动认领任何一项，也不重新付费提交。详见 [运行契约](operating-contract.md#研究任务键与可信接管)。
- 判成败看 `ok`。`experiment list` 的 `id` 就是 `fcRunId`，可直接 join。

---

## A5 抓取 → 报告 → 指到审核页

### 抓取清单（只读，可并发 4）

```bash
skz mining runs --route <R>                 # 本轮每轮：total_candidates / retained / retain_rate / status / elapsed_s
skz mining overview <run_id>                # 漏斗 + 淘汰分解（报告第 3 章要用）
skz factor summary                          # 路线累计 factor_count + top_factors[].sharpe
skz experiment list                         # 本批每个 exp：passed / n_backtests / pass_rate / strategy_count / symbols_count
skz experiment get <exp_id>                 # ⚠️ 取 ["overview"]
skz experiment strategies <exp_id>          # 通过候选明细 + verdict.yearly_metrics[]
skz experiment review-matrix <exp_id>       # 后置验证表现取 segment_name=="后置验证" 行，不代表真正样本外
```

⚠️ 把失败的 id 记进 `miss` 列表**别静默跳过**；skz 网络会抖，单次超时常见，加 2 次重试。

### 报告

按 [report-spec.md](report-spec.md) 出**单文件 HTML**（左栏 sticky 目录 + 深酒红侧栏 + 浅色正文 + 红涨绿跌），
落到 `reports/` 或用户指定目录。

### 结尾必须写的审核指引

> 到胜可知平台「策略管理 → 策略审核」页逐条审核候选：https://quant.shengkezhi.com/
> （功能说明：https://docs.shengkezhi.com/strategy）
> 审核要点：别只挑历史收益最高的——核对交易成本、验证期稳定性、回撤，以及表现是否**符合原路线的逻辑**。
> 审核通过后需要**单独**保存入库；入库即 `暂停` 态，切实盘是再之后的独立决定。

**不要替用户做审核，也不要自动 promote。** 报告只指路 + 给出"哪些值得优先看"的判断依据。

---

## 交付话术（结论先行）

1. 一句话结论：这条经验在本次研究窗口复现出来了吗？（看 A3 的留存趋势 + A4 的通过率与后置验证表现，不宣称独立验证通过）
2. 挖掘侧：批准轮次中跑了几轮、为什么停、留存中位多少、跟历史比如何。
3. 探索侧：几个问题上出了候选、哪些问题的候选后置验证表现较好；仅有该数据时，尚不能确认真正样本外。
4. 局限（照报告写全）：全周期口径 ≠ 样本外、现金仅为参考基准且低净敞口不证明风险中性、多重比较、成本口径未确认；分列验证区间、训练/筛选使用情况、入库后的 `nav.oos_start`，缺失写未知，日期新旧不证明独立性。
5. 下一步：去审核页看哪几条；要调路线参数就回去改 key_inspect 再挖（**新建 route，不能 update**）。
