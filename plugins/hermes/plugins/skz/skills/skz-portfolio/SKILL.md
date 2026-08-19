---
name: skz-portfolio
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上的组合（portfolio）资产——把多个实盘策略按权重打包成一个组合、查组合净值/回撤/持仓/再平衡权重。当用户提到「我的组合」「策略组合」「多策略打包/配置权重」「组合净值/回撤」「再平衡」，或要把几个实盘策略组合到一起时使用。不负责单个策略的实盘运营（那是 skz-strategy）。
---

# skz 技能 · portfolio（组合管理）

组合 = 把若干个已经在**实盘**的策略，按权重打包成一个整体，在若干个再平衡节点上重新配一次权重，作为一条单独的净值曲线运营。资产链条上，组合建在 `strategy`（实盘策略库）**之上**：先有能打包的实盘策略，才有能建的组合。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。需要解释结果、申请确认或给下一步时，再读取 [references/communication.md](references/communication.md)。不要把两个 reference 的正文复制回本文件。

可执行工具均为只读或纯校验：

- `scripts/preflight.py --operation read|write|paid`：检查 CLI、身份和本地写策略。
- `scripts/resume.py`：跨会话重建在途挖矿、探索和组合任务。
- `scripts/validate_plan.py`：校验付费计划；只返回 `approved:false`，绝不代替用户批准。
- `scripts/verify_write.py`：写超时后读回确认；绝不重放写命令。

六册分工：`skz-guide` 负责研究导航和付费触发；`skz-create-problem` 负责定义和创建研究问题；`skz-factor` 负责因子资产；`skz-candidate` 负责实验、候选和保存入库；`skz-strategy` 负责已入库策略；`skz-portfolio` 负责组合。任务跨边界时切换到对应技能，不要在当前册猜另一册的契约。

安装用 `skz plugin install <claude|codex|openclaw|hermes|all>`，状态以 `skz plugin status <target>` 的 `needs_install` 为准；升级后若报告 stale，重新安装。`skz --version` 输出 CLI 与 plugin contract，命令参数以 `skz --help` 为准。

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

只有 `portfolio_code`、`candidate_strategies`、`rebalance_dates`、`base_market` 是必需的；`description`/`base_freq`/`price_field`/`rebalance_method` 缺省分别是 `""`/`"1d"`/`"close"`/`"equal_weight"`。CLI 会本地校验 code/候选结构，并在付费 POST 前自动读取组合库和实盘策略库做预检；其余字段合法性交后端。

**⚠️ HITL：调它之前先跟你的人确认。** 判据是「付费触发」——建组合会异步下发一次 Function Compute 组合优化（建历史持仓 → 回测 → 报告 → 最新目标持仓一条龙），跟 `mine/explore start`、`promote start`（保存入库）是同一档花钱操作。

三个容易踩的坑，写脚本前先确认：

- **`base_market` 是英文小写枚举**：`binance` / `etf` / `future` / `index` / `stock` 五选一。别从 `strategy` 侧的中文市场（`A股`）或 `problem` 侧的 `dataset`（`future`/`etf`/`stock`）抄错值域——三处长得像，其实是三套枚举。传错直接 422 fix_params，不会静默通过。
- **`candidate_strategies` 必须是「实盘」状态的策略代码**：CLI 会在 POST 前用 `strategy list --status 实盘` 自动核对；打错、暂停或废弃都会立即 `fix_params`，不触发组合优化。申请付费许可前仍应把候选清单展示给人复核。
- **组合代码禁止复用**：后端原本会重新触发 FC 并覆盖旧组合，CLI 现在会先查 `portfolio list`，发现同名 code 就立即 `fix_params`。想改一版权重/再平衡节点，使用新 `portfolio_code`。

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
# 1. 人工复核候选；CLI 在提交时还会自动执行同一项校验
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

**结论**：组合是站在实盘策略之上的第二层封装。CLI 会在付费提交前核实候选和 code 冲突；提交后仍必须用 `portfolio list` 轮询，避免把“还在生成”误报成“建失败了”。
