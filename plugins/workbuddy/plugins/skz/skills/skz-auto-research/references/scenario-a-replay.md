# 场景 A · 交易经验 / 交易策略复现

**输入**：一段交易经验或一个已有策略（可能是别人写的、或用户自己总结的规律）。
**产出**：一条因子路线 + 若干次挖掘结果 + 在 ≥5 个研究问题上的探索结果 + 一份 HTML 汇总报告。

```
A0 前置检查(只读)
 → A1 选定 ≥5 个研究问题          [用户确认]  ← 不够则现场建 problem（HITL）
 → A2 把交易经验写成因子路线 route  [用户确认]  ← 七字段完整展示后建
 → A3 串行挖掘 ×10，有效因子数 <50 即停（自动段）
 → A4 在选定的每个问题上发起 explore [付费许可]
 → A5 抓取 → HTML 报告 → 指到策略审核页
```

**两条硬闸门**：A2 建路线前展示七字段；A3/A4 付费前先 `wallet check` + 三句话说明 + 拿许可。
**执行优先脚本**：A1/A2 人审完成后用 `scripts/run_pipeline.sh` 贯通 A3→A4→A5。
参数、审批记录、预算及可选一次补跑规则见 [cli-contract.md](cli-contract.md) 第 9 节。
既有同机制 route 直接续跑；仅已确认新 route_json 才加 --create-route，七字段/readback 不能静默放宽。
默认预检不付费；不分发旧 orchestrate，不把下面教学命令拼成无人审的 shell 链。

**一个自动段**：A3 一旦启动就自己判阈值、自己决定要不要继续，**不要每轮都来问用户"还挖吗"**——
用户的意图就是"跑到低产为止"。但要**在每轮结束后给他一行进度**（第 k/10 轮、本次留存多少、是否继续）。

---

## A0 前置检查（只读，30 秒）

```bash
skz auth status                   # 本地身份与只读策略
skz whoami                        # 身份
skz wallet balance                # 余额 → 决定 10 次挖掘 + N 次探索够不够
skz mine runs --status active     # 有在跑的挖掘 → 先接管轮询，别重复触发
skz explore runs --status active
skz problem list                  # 现有研究问题（A1 的候选池）
skz factor-routes list            # 现有路线（A2 去重用）
```

把结果压成 4 行给人看：身份 OK / 余额 X（够 Y 次挖掘 + Z 次探索）/ 在途任务 / 现有问题数 & 路线数。

---

## A1 选定研究问题（≥5 个）

### 怎么筛

拿交易经验里的**品种 + 频率**当筛子，从 `problem list` 里过滤：

| 交易经验的属性 | 对应的问题字段 |
|---|---|
| 品种（股票 / ETF / 期货，哪个板块） | `dataset` = `stock` / `etf` / `future` |
| 交易周期（短线/日内/波段） | `freq` = `日线` / `60分钟` / `15分钟` / … |
| 标的集合与相关性强弱 | `symbols[]`（时序要求 ≤10 个且高相关） |
| 策略形态（择时 / 多空） | `problem_type` = `TimeSeriesProblem` |

**筛选原则**：

1. **同 dataset + 同 freq 优先**——只有同族问题的结果才可比，跨频率混在一张报告里没法横向读。
2. **时序标的池遵循 ≤10 个且高相关的契约**。数量不是样本质量保证；根据机制、周期与样本长度说明适配性，不为凑数自动扩池。
3. **优先复用**账号里已有的问题（复用 = 免费 + 已有历史实验可做基准）。
4. 至少 5 个；**上限不必卡死**，但每多一个问题就多一次 explore 扣费，先跟用户对齐 N。

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

### 不够 5 个时现场建（HITL）

```bash
skz problem meta                  # 必查：合法 type/dataset/freq + default_time_segments + max_time_segment_date
skz symbols --market future --keyword 焦煤 --size 200    # 确认标的代码（带 999 后缀的主力连续）
skz problem create < problem.json # 展示后确认再建
```

`problem.json` 骨架（`time_segments` 必须 5 段齐全，照抄 meta 的 `default_time_segments` 改日期）：

```json
{"problem_type":"TimeSeriesProblem","dataset":"future","freq":"日线",
 "name":"焦煤·日线时序","description":"<目标 / 口径 / 成功标准>",
 "symbols":["JM999.DCE"],
 "time_segments":[{"name":"训练集A段","sdt":"20170101","edt":"20190101"},
                  {"name":"训练集B段","sdt":"20190101","edt":"20210101"},
                  {"name":"训练集C段","sdt":"20210101","edt":"20230101"},
                  {"name":"训练集","sdt":"20170101","edt":"20230101"},
                  {"name":"后置验证","sdt":"20230101","edt":"20250101"}]}
```

**先把待建问题完整展示（标的集合 / 市场 / 频率 / 五段时间分段 / 描述），取得明确许可后才 create。
内容有修改要重新展示重新确认。**（用户说"直接建"也不得跳过展示。）

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

⚠️ `route` **没有 update**，写错只能 delete 重建 → 创建前逐字核对 9 个提交字段。
⚠️ 新建的 route **因子数为 0**，必须先挖才能 explore。

---

## A3 串行挖掘 ×10（自动段 · 核心）

### 为什么串行

见 SKILL.md「铁律/为什么必须串行」：**CLI 无取消命令**，批量触发后撤不回来，
"发现低产就停止后续"只能靠"一次只在途 1 条"。实测批量触发本就是排队串行（不损失吞吐）。

### 状态机

```
for round in 1..10:
    ① wallet check mine --qty 1（余额不足 → 停手问人）
    ② 先 fsync 写前 intent/费用保留，再 mine start --route R → fcRunId，立即落盘响应
    ③ 轮询 mine poll <fcRunId>（60s/轮，判 ok）
    ④ 取 retained（见下"取数"）
    ⑤ 判定：
        ok=true 且 retained >= 50  → 继续下一轮
        ok=true 且 retained <  50  → ⭐ 停止本批次，带已挖到的因子进 A4
        failed/done=true/ok=false → 默认暂停；单独授权且白名单执行失败才核验后补跑一次
        done=false / timeout / 长时间 queued 不动 → 不补跑；窗口耗尽暂停（不等于停止云端计算）
```

每轮结束给人一行：

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

### 阈值 50 的校准（别把它当普适真理）

```bash
skz mine runs --size 50      # 任务台：看本账号历史的 status 分布（含失败/超时）
skz mining runs              # 成果柜：看本账号历史的 retained / retain_rate 分布
```

实测样例：3000 候选 → retained 36（1.2%）、另一路线 2482 → 206（8.3%）。
**50 这个阈值处在"偏低"的区间**，用途是"筛掉明显异常低产的轮次"，不是"质量门槛"。
如果本账号常态留存就该是 100+，把阈值调高；如果常态是 20–40，阈值 50 会让第 1 轮就停——
**这件事必须在开始前跟用户确认一句**，别闷头按 50 跑完发现第一轮就停。

### 耗时与轮询

- 单条中位 ~26 分钟、近期 174–206 分钟 → **串行 10 次要做好跨天准备**。
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

- **explore 真并发**：10 条 20 分钟内全部完成，别按 mine 的串行口径排期。
- ⚠️ 因子库存为 0 → 必失败（`no_valid_factors`）。**先查再投。**
- ⚠️ 幂等：按 `problem` 去重；用户重复下指令时先查 `explore runs` / `experiment list`
  的 `createdAt` 确认已触发，**接管轮询，绝不再 start**。
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
2. 挖掘侧：10 轮里跑了几轮、为什么停、留存中位多少、跟历史比如何。
3. 探索侧：几个问题上出了候选、哪些问题的候选后置验证表现较好；仅有该数据时，尚不能确认真正样本外。
4. 局限（照报告写全）：全周期口径 ≠ 样本外、现金仅为参考基准且低净敞口不证明风险中性、多重比较、成本口径未确认；分列验证区间、训练/筛选使用情况、入库后的 `nav.oos_start`，缺失写未知，日期新旧不证明独立性。
5. 下一步：去审核页看哪几条；要调路线参数就回去改 key_inspect 再挖（**新建 route，不能 update**）。
