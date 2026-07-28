---
name: skz-portfolio
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上的组合（portfolio）资产——把多个实盘策略按权重打包成一个组合、查组合净值/回撤/持仓/再平衡权重。当用户提到「我的组合」「策略组合」「多策略打包/配置权重」「组合净值/回撤」「再平衡」，或要把几个实盘策略组合到一起时使用。不负责单个策略的实盘运营（那是 skz-strategy）。
---

# skz 技能 · portfolio（组合管理）

组合 = 把若干个已经在**实盘**的策略，按权重打包成一个整体，在若干个再平衡节点上重新配一次权重，作为一条单独的净值曲线运营。资产链条上，组合建在 `strategy`（实盘策略库）**之上**：先有能打包的实盘策略，才有能建的组合。

<!-- COMMON -->

## 1) 建组合（写 · 花钱 · 必须先问人）

```bash
skz portfolio create   # 从 stdin 读一份 JSON body，POST /research/portfolios
```

stdin body 示例：

```json
{
  "portfolio_code": "PF_MOMENTUM_CLUSTER",
  "description": "动量策略组合，季度再平衡",
  "candidate_strategies": ["STS_1D_DSKCIB7M", "FTS_1D_9ZWKQUKE"],
  "rebalance_dates": ["2025-01-01", "2025-04-01", "2025-07-01", "2025-10-01"],
  "base_market": "stock",
  "base_freq": "1d",
  "price_field": "close",
  "rebalance_method": "equal_weight"
}
```

只有 `portfolio_code`、`candidate_strategies`、`rebalance_dates`、`base_market` 是必需的；`description`/`base_freq`/`price_field`/`rebalance_method` 缺省分别是 `""`/`"1d"`/`"close"`/`"equal_weight"`。CLI 只本地校验"是合法 JSON object"，字段合法性交后端（不变量 4）。

**⚠️ HITL：调它之前先跟你的人确认。** 判据是「付费触发」——建组合会异步下发一次 Function Compute 组合优化（建历史持仓 → 回测 → 报告 → 最新目标持仓一条龙），跟 `mine/explore start`、`promote start`（保存入库）是同一档花钱操作。

三个容易踩的坑，写脚本前先确认：

- **`base_market` 是英文小写枚举**：`binance` / `etf` / `future` / `index` / `stock` 五选一。别从 `strategy` 侧的中文市场（`A股`）或 `problem` 侧的 `dataset`（`future`/`etf`/`stock`）抄错值域——三处长得像，其实是三套枚举。传错直接 422 fix_params，不会静默通过。
- **`candidate_strategies` 必须是「实盘」状态的策略代码，但后端建组合时不校验这条**——只查非空，不查状态、不查代码是否存在。传了打错的代码或还在 `暂停`/`废弃` 的策略，一样先 202 受理，几分钟后组合优化才在后台异步失败（`job_status` 变 `failed`）。跟 `explore start --problem <不存在的>` 是同一个陷阱、同一条防线：**建组合前先 `skz strategy list --status 实盘` 确认每一个候选代码真实存在且在实盘**。
- **组合代码复用会重新触发一次 FC，且不报冲突**：同一个 `portfolio_code` 若还在 `pending`，重复提交会被去重复用；但只要**曾经成功过**，任务记录已被清理，同名代码再提交就是一次全新的组合优化（覆盖旧的），不会像 `mine`/`explore` 那样撞 409。想改一版权重/再平衡节点，直接换个新 `portfolio_code`，别复用旧的。

## 2) 生成中怎么知道好了没

**⚠️ 这条是本册最容易踩空的一个坑：`skz portfolio get <code>` 在生成中、以及生成失败之后，一样回 404 / exit 2 `fix_params`。** 组合详情端点只读磁盘上已落地的产物，产物还没生成完（或者生成失败、根本没落地）时目录不存在，跟"code 打错了"是同一个错误——**不能靠它判断"是不是还在跑"**。

正确做法是 **`skz portfolio list`**：新建组合任务的状态会被 merge 进列表项：

```json
{"code":"PF_MOMENTUM_CLUSTER","description":"动量策略组合，季度再平衡","status":"生成中",
 "base_market":"stock","base_freq":"1d","symbol_count":0,"strategy_count":2,
 "sdt":"","edt":"","annual_return":null,"sharpe":null,"max_drawdown":null,"abs_return":null,
 "job_status":"pending","job_error":null}
```

- `job_status:"pending"` → 还在跑，过一会再 `list` 一次。
- `job_status:"failed"` → 生成失败，`job_error` 给失败文案（已脱敏，不含内部堆栈）；这个 code 已经可以复用重建（旧任务记录已清）。
- `job_status` **缺席**（字段不出现或为 `null`）→ 已经落地成真实组合，这时候 `skz portfolio get <code>` 才会正常返回，且 `status` 会变回正常值。

**⚠️ 顺带一个不直观的地方：任务态占位行的 `status` 不是组合的正常状态值,而是 `"生成中"` / `"生成失败"` 这两个专门的人话标签**——跟 `strategy status` 那套「实盘/暂停/废弃」中文枚举是两码事，别当成同一个字段的同一套取值来解析。而且**目前没有组合层面的状态切换或删除端点**：一个组合一旦生成成功，`status` 就固定是 `"实盘"`，平台没给 CLI 暴露改它的入口。

## 3) 组合库与详情（读 · 可自主）

```bash
skz portfolio list           # 全部组合，**没有任何筛选/分页参数**——传了也不认，后端 handler 本身不收
skz portfolio get <code>     # 详情：meta + 持仓权重 + 净值/回撤/月度/多空归因 + 判定
```

`list` 每一项的核心指标（`annual_return`/`sharpe`/`max_drawdown`/`abs_return`）在组合还没生成好之前是 `null`，不是 `0`——别把 `null` 当成"表现是 0"。

`get` 返回的详情里，除了 `meta`（组合配置）、`strategies`（`strategy_id`+`weight`）、`latest_weights`（最新一期目标持仓，`symbol`+`weight`）这几个 ASCII 字段是强类型的，剩下几块是**松散 map，原样透传**（跟 `strategy metrics`/`strategy definition` 同一套做法）：

- `metrics` / `compare_metrics` —— 全样本核心指标 + 多空/多头/基准/超额的对比指标，**键是中文**（`夏普比率`/`最大回撤`…）
- `compare.series` —— 多空/多头/基准/超额四条累计收益曲线，键也是中文 leg 名
- `positions.weights` —— `symbol -> 每日权重数组`（对齐 `positions.dates`）
- `verdict` —— `history`/`recent` 两段的判定明细

**中文键在 jq 里要用 bracket 记法**（跟 `strategy.md` 里的坑同一个成因）：`jq '.metrics["夏普比率"]'`，不是 `jq '.metrics.夏普比率'`。

`has_report` 只是一个信号位，标记后端是否生成了 HTML 回测报告——**本册不提供拉取报告的命令**。`get` 返回的结构化数据（`nav`/`monthly`/`drawdowns`/`compare`/`verdict`/`positions`）已经覆盖了报告里的同一份数据，agent 用得上的都在这些字段里。

## 一个典型任务（照着改）

「把这几个实盘策略打包成一个组合，季度再平衡」：

```bash
# 1. 确认每个候选真的在实盘——建组合不校验这条，传错要等异步失败才知道
skz strategy list --status 实盘 | jq '.items[].code'

# 2. 向人说清楚要花钱、异步生成、得到同意后再建
skz portfolio create <<'EOF'
{"portfolio_code":"PF_MOMENTUM_CLUSTER","description":"动量策略组合，季度再平衡",
 "candidate_strategies":["STS_1D_DSKCIB7M","FTS_1D_9ZWKQUKE"],
 "rebalance_dates":["2025-01-01","2025-04-01","2025-07-01","2025-10-01"],
 "base_market":"stock"}
EOF

# 3. 轮询用 list 的 job_status，不要用 get（生成中 get 会 404/exit 2，别误判成失败）
skz portfolio list | jq '.items[] | select(.code=="PF_MOMENTUM_CLUSTER") | .job_status'

# 4. job_status 缺席（已就绪）之后，再看详情
skz portfolio get PF_MOMENTUM_CLUSTER
```

**结论**：组合是站在实盘策略之上的第二层封装，建之前的唯一防线是自己先核实候选代码，建之后的唯一防线是别用错端点轮询——这两条不对，agent 要么会花冤枉钱建一个注定失败的组合，要么会把"还在生成"误报成"建失败了"。
