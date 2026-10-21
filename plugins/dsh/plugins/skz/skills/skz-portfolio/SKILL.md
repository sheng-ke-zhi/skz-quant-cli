---
name: skz-portfolio
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上的组合（portfolio）资产——把多个实盘策略按权重打包成一个组合、查组合净值/回撤/持仓/再平衡权重。当用户提到「我的组合」「策略组合」「多策略打包/配置权重」「组合净值/回撤」「再平衡」，或要把几个实盘策略组合到一起时使用。不负责单个策略的实盘运营（那是 skz-strategy）。
---

# skz 技能 · portfolio（组合管理）

组合 = 把若干个已经在**实盘**的策略，按权重打包成一个整体，在若干个再平衡节点上重新配一次权重，作为一条单独的净值曲线运营。资产链条上，组合建在 `strategy`（实盘策略库）**之上**：先有能打包的实盘策略，才有能建的组合。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。需要解释结果、申请确认或给下一步时，再读取 [references/communication.md](references/communication.md)。不要把两个 reference 的正文复制回本文件。

公共安装、工具和技能职责见 [运行契约](references/operating-contract.md#公共安装工具与职责)。任务跨边界时切换技能，不猜另一册的契约。

## 前端同款 Agent 新建组合

用户提供自然语言目标，希望选择研究问题/策略范围、平台或 BYOK 模型，由 Agent 探索并生成配置时，先读 [Agent 构建流程](references/agent-build.md)，使用 `skz portfolio build catalog/start/latest/get/history/notice/read`。费用检查为 `wallet check portfolio-build` 或 `portfolio-build-byok`，仅绑定这一条 Agent 构建链路。已知配置直接导入仍用下文的 `portfolio create`。

## 1) 建组合（写 · 可能付费 · 先核价再授权）

**执行门槛：** 先按 [references/billing.md](references/billing.md) 核验组合专属费用，再展示本次完整 JSON 参数、范围、费用依据与金额、当前余额、预计耗时和失败信号，取得绑定本次参数及费用的明确许可。当前 CLI 没有组合报价项，服务端实际计费尚未核实，必须暂停创建；不得用 `wallet check refresh` 的策略更新价替代，也不得为核价试跑。以下写命令仅说明接口，不是当前可直接执行的工作流。

```bash
skz portfolio create   # 从 stdin 读完整 JSON 或 TOML，POST /research/portfolios
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

`portfolio_code`、`candidate_strategies` 必填；显式 `rebalance_dates` 与 `rebalance_date_rules` 至少一种，`base_market` 可省略。完整配置支持范围见 CLI `skz-openapi` 中的创建组合参考；`description`/`base_freq`/`price_field`/`rebalance_method` 缺省分别是 `""`/`"1d"`/`"close"`/`"equal_weight"`。CLI 会本地校验 code/候选结构，并在提交 POST 前自动读取组合库和实盘策略库做预检；其余字段合法性交后端。该预检不核验组合费用，也不代替授权。

`validate_plan.py` 的组合计划还要求 `operation:"portfolio.create"`、`assumption` 和 `failure_signal`。离线要求四个创建必填字段类型正确，候选为非空且无首尾空格的字符串数组，再平衡日期为非空 `YYYY-MM-DD` 有效日历日期数组；不推测服务端市场枚举或日期排序限制。在线读取完整实盘策略分页，以顶层 `code` 精确核对；分页失败、总数变化或响应结构异常报告 `state:"unknown"`，不据此宣称候选不存在。

`valid:true` 仅表示当前校验范围通过：`validation_scope:"schema_only"` 是离线格式校验，`"read_only_preflight"` 是格式及只读预检，均不是服务端最终验收或可提交保证。输出始终保留 `approved:false`、`approval_required:true`、`billing_verified:false`、`ready_to_submit:false`；研究计划元数据不是创建 body 字段，实际提交体应按上方示例另行构造。组合核价与本次用户授权未完成前仍须暂停。

**⚠️ HITL：核价后仍须先跟用户确认。** 建组合会异步下发一次 Function Compute 组合优化（建历史持仓 → 回测 → 报告 → 最新目标持仓一条龙），按可能付费的写操作管理，不能从 `mine/explore start` 推断其价格；`promote start` 保存入库不收费，也不能据此推断组合免费。

三个容易踩的坑，写脚本前先确认：

- **`base_market` 是英文小写枚举**：`binance` / `etf` / `future` / `index` / `stock` 五选一。别从 `strategy` 侧的中文市场（`A股`）或 `problem` 侧的 `dataset`（`future`/`etf`/`stock`）抄错值域——三处长得像，其实是三套枚举。传错直接 422 fix_params，不会静默通过。
- **`candidate_strategies` 必须是「实盘」状态的策略代码**：CLI 会在 POST 前用 `strategy list --status 实盘` 自动核对；打错、暂停或废弃都会立即 `fix_params`，不触发组合优化。申请付费许可前仍应把候选清单展示给人复核。
- **组合代码禁止复用**：后端原本会重新触发 FC 并覆盖旧组合，CLI 现在会先查 `portfolio list`，发现同名 code 就立即 `fix_params`。想改一版权重/再平衡节点，使用新 `portfolio_code`。

## 2) 绩效产物和刷新

`skz portfolio list` 和 `skz portfolio get <code>` 都读取已保存的组合定义。响应中的 `has_performance` 表示是否存在可展示的绩效 MsgPack：

- `has_performance: true` → 可以直接查看组合绩效、净值、回撤和持仓。
- `has_performance: false` → 组合定义存在但绩效产物缺失，详情仍可读；只报告产物缺失，不自动刷新。用户只要求查看组合不代表同意后台计算。可只读查询 `skz portfolio refresh-status <code>` 了解已有刷新任务。

刷新必须先按 [references/billing.md](references/billing.md) 核验组合专属费用，再展示组合 code、刷新范围与当前配置、费用依据与金额、当前余额、预计耗时和失败信号，取得绑定本次参数及费用的明确许可。当前无组合报价能力时必须暂停刷新；仅告知用户、用户同意查看或允许跳过余额检查都不满足门槛。以下仅为核价和授权完成后的命令说明：

```bash
skz portfolio refresh <code>
skz portfolio refresh-status <code>   # 轮询最近一次刷新任务
```

刷新会重新触发一次 Function Compute 组合优化，按可能付费的写操作管理，实际计费待核实，不能自动重试。刷新完成后再次执行 `skz portfolio get <code>` 读取新产物。

## 3) 组合库与详情（读 · 可自主）

```bash
skz portfolio list           # 全部组合，**没有任何筛选/分页参数**——传了也不认，后端 handler 本身不收
skz portfolio get <code>     # 详情：meta + 持仓权重 + 净值/回撤/月度/多空归因 + 判定
skz portfolio report <code>  # 返回 {contentType,body}；body 是完整 HTML，通常很大
```

`list` 每一项的核心指标（`annual_return`/`sharpe`/`max_drawdown`/`abs_return`）在组合还没生成好之前是 `null`，不是 `0`——别把 `null` 当成"表现是 0"。

`get` 返回的详情里，除了 `meta`（组合配置）、`strategies`（`strategy_id`+`weight`）、`latest_weights`（最新一期目标持仓，`symbol`+`weight`）这几个 ASCII 字段是强类型的，剩下几块是**松散 map，原样透传**（跟 `strategy metrics`/`strategy definition` 同一套做法）：

- `metrics` / `compare_metrics` —— 全样本核心指标 + 多空/多头/基准/超额的对比指标，**键是中文**（`夏普比率`/`最大回撤`…）
- `compare.series` —— 多空/多头/基准/超额四条累计收益曲线，键也是中文 leg 名
- `positions.weights` —— `symbol -> 每日权重数组`（对齐 `positions.dates`）
- `verdict` —— `history`/`recent` 两段的判定明细

**中文键在 jq 里要用 bracket 记法**（跟 `strategy.md` 里的坑同一个成因）：`jq '.metrics["夏普比率"]'`，不是 `jq '.metrics.夏普比率'`。

`has_report` 标记后端是否生成了 HTML 回测报告。需要原始报告时用 `portfolio report`；普通分析优先使用 `get` 的结构化数据，避免把数 MB HTML 整体塞进上下文。

## 4) 生命周期与删除

```bash
skz portfolio status <code> --expected-status <实盘|暂停|废弃> --status <实盘|暂停|废弃>
skz portfolio delete <code>
```

`status` 要求显式给出用户看到的原状态，后端用它防止覆盖其他会话的并发变更。切到 `暂停` 可自主降风险；切到 `实盘`、`废弃` 或物理删除前必须展示组合 code、当前状态和影响并取得确认。写结果不确定时分别用 `portfolio get` 或 `portfolio list` 读回，禁止直接重发。

## 一个典型任务（照着改）

「把这几个实盘策略打包成一个组合，季度再平衡」：

先只读复核候选，再按费用契约核价。当前没有组合报价能力时到此暂停，说明缺少费用依据，不执行下方创建命令；不能用策略刷新报价代替。以下第 2—4 步仅描述未来费用可核实且用户已明确授权后的流程，不可整段直接运行。

```bash
# 1. 人工复核候选；CLI 在提交时还会自动执行同一项校验
skz strategy list --status 实盘 | jq '.items[].code'

# 2. 仅在组合费用已核实、完整参数及费用获明确许可后创建
skz portfolio create <<'EOF'
{"portfolio_code":"PF_MOMENTUM_CLUSTER","description":"动量策略组合，季度再平衡",
 "candidate_strategies":["STS_1D_DSKCIB7M","FTS_1D_9ZWKQUKE"],
 "rebalance_dates":["2025-01-01","2025-04-01","2025-07-01","2025-10-01"],
 "base_market":"stock"}
EOF

# 3. 读取组合列表，确认 has_performance
skz portfolio list | jq '.items[] | select(.code=="PF_MOMENTUM_CLUSTER") | .has_performance'

# 4. has_performance 为 true 后查看详情
skz portfolio get PF_MOMENTUM_CLUSTER
```

**结论**：组合是站在实盘策略之上的第二层封装。CLI 会在提交前核实候选和 code 冲突；费用核验及明确授权仍是提交前提。提交后仍必须用 `portfolio list` 轮询，避免把“还在生成”误报成“建失败了”。
