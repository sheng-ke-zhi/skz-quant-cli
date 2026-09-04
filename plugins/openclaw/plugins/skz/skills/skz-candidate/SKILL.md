---
name: skz-candidate
description: 用 skz CLI 评审和处置胜可知（Shengkezhi）量化平台上的探索实验与候选策略——查看实验通过率和评审矩阵、比较候选的跨时段表现、删除候选或整次探索、把选中的候选保存入库（promote）。当用户提到「候选策略」「实验结果」「探索跑完了」「评审/筛选候选」「删除候选/实验」「保存入库」，或要在策略探索完成后挑选成果时使用。不负责触发探索（那是 skz-guide），也不负责已入库策略的实盘查询和运营（那是 skz-strategy）。
---

# skz 技能 · candidate（候选评审与入库）

策略探索的产出在这册收尾：**评审候选 → 处置落选结果 → 保存选中候选入库**。

资产链条：一次探索产出一个**实验**（含多个候选策略）→ 挑中的候选**保存入库**（命令是 `promote`，进库即 `暂停` 态）→ 交给 `skz-strategy` 观察和运营。

> **跟人说话时这个动作叫「保存入库」，不叫 promote，也不叫「上线」。** 入库和真上场是两个决定、两笔风险；用户在这里确认保存入库，不代表允许切到实盘。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。需要解释结果、申请确认或给下一步时，再读取 [references/communication.md](references/communication.md)。不要把 reference 的正文复制回本文件。

可执行工具均为只读或纯校验：

- `scripts/preflight.py --operation read|write|paid`：检查 CLI、身份和本地写策略。
- `scripts/resume.py`：跨会话重建在途的因子挖掘、探索和组合任务。
- `scripts/validate_plan.py`：校验付费计划；只返回 `approved:false`，绝不代替用户批准。
- `scripts/verify_write.py`：写超时后读回确认；绝不重放写命令。

七册分工：`skz-wallet` 负责资金和费用；`skz-guide` 负责研究导航、因子挖掘与策略探索；`skz-create-problem` 负责定义研究问题；`skz-factor` 负责因子资产；`skz-candidate` 负责实验、候选和保存入库；`skz-strategy` 负责已入库策略与实盘更新；`skz-portfolio` 负责组合。任务跨边界时切换到对应技能，不要在当前册猜另一册的契约。

安装用 `skz plugin install <claude|codex|openclaw|hermes|dsh|all>`，状态以 `skz plugin status <target>` 的 `needs_install` 为准；升级后若报告 stale，重新安装。`skz --version` 输出 CLI 与 plugin contract，命令参数以 `skz --help` 为准。DSH 网页版默认关闭 skill，装完后到 Settings → Plugins 确认 `skill-filesystem` 与 `tool-skill` 为 Enabled（CLI/headless 默认已开启）。

## 1) 评审候选

```bash
skz experiment list                          # 实验列表；计数字段见下（别猜字段名）
skz experiment get <id>                      # 概览 {overview}：通过率、回测数、problem、耗时、errors
skz experiment strategies <id>               # 候选清单（只有通过的）
skz experiment review-matrix <id>            # 评审矩阵：全部回测 x 各时段指标
```

`experiment get` 若 exit 5 / `code=42201`（数据尚未就绪）：产物还没落地，或 id 对不上。稍后重试，或回 `experiment list` 核对 id，不要当成 internal。列表里的 `total_elapsed` 是探索全流程耗时，`elapsed_s` 只是复审耗时；有前者时优先用前者。

这两个结果的范围不同：

- `experiment strategies` 只给通过的候选，每条带 `passed`。列表项主键是 `code`，而 `experiment list` 主键是 `id`。`experiment list` 没有分页 flag。
- `experiment review-matrix` 包含全部回测，行上没有 `passed` / `verdict`。行数等于回测数乘以 problem 自己定义的时段数，不能把某次的乘数当常量。

所以必须先从 `experiment strategies` 取得通过的 code 集合，再筛选 review matrix。直接读矩阵会把落选回测当成候选。

`experiment list` 的真实计数字段是 `scanned` / `passed` / `failed` / `skipped` / `pass_rate` / `n_backtests` / `n_strategies` / `strategy_count`：

- 通过率直接使用平台的 `pass_rate`，不要自己除。
- `strategy_count` 只统计尚未登记、仍可评审的候选；保存入库或删除后会减少。
- 历史回测总量看 `n_backtests`，不会随候选消费而改写。
- `passed + failed` 不一定等于 `n_backtests`，不要据此推导失败率。
- 先查看 `.items[0]` 的真实 schema 再写 `jq`，猜错字段名可能静默得到 `null`。

`experiment strategies` 的单条还包含 `verdict`（`is_good` / `reason` / `cond_*_passed` / `yearly_metrics[]`）、`model`、`route`、`symbol_count`、`weight_type` 和 `metrics`。评审时至少核对：

1. `verdict.yearly_metrics` 是否存在单年整体崩掉。
2. review matrix 各训练段是否稳定，不能只看总指标。
3. `metrics` 的多头/空头占比是否符合预期构造。
4. 指标的实际窗口，不能混用同名但不同窗口的夏普或回撤。

### 评审原则（四条）

1. **样本内外分开比，主看收益风险比。** 候选阶段拿不到 `oos_start`——它只在入库后由 `strategy nav` 提供，真正的样本外判断在入库后由 `skz-strategy` 按 `nav.oos_start` 执行。候选阶段的样本内外对比用可得的 `time_segments` 和 review matrix 的时段划分来做：训练段之外的时段按样本外对待，但「后置验证」段仍可能是样本内（见下）。样本外显著失效是过拟合的强信号，但要结合有效样本量判断——样本外段太短不足以定论，别直接否。比指标时先看夏普、卡玛这类收益风险比，不看绝对收益。
2. **多头超额、空头超额是评价策略能力的重要依据。** 两腿相对基准各自的超额分开看：两腿都有正超额是很强的能力证据，只有一腿赚钱说明能力边界窄。但它不替代对策略单边定位与市场环境的判断——单边构造碰上不配合的市场，超额照样难看，别只看组合总收益下结论。
3. **年化收益、最大回撤只作参考，不做绝对评判。** 这两类指标与杠杆使用强相关——杠杆会显著改变年化收益和回撤；在线性、低成本的简化口径下才可近似视为同比放大。跨策略比较先对齐杠杆口径；最终拍板的尺子始终是收益风险比。
4. **策略横向对比先归一到同一风险水平。** 比如统一按年化波动率 20% 归一后，再比较收益风险比、夏普、卡玛与多空超额；同时保留实际杠杆与未归一的原始指标，避免归一化掩盖可实现性或尾部风险。

> **「后置验证」不等于真正样本外。** 将该段的 `edt` 与 `nav.oos_start` 比较；`edt <= oos_start` 的段仍按样本内处理。真正样本外表现要在入库后由 `skz-strategy` 从 `strategy nav` 的 `oos_start` 之后计算。

候选侧的风险指标也可能与入库侧口径不同。例如候选 `history_alpha_max_drawdown` 是相对 alpha/基准回撤，入库 `recent-eval.history.全样本回撤` 是绝对回撤。结论冲突时先核对定义，不要直接说策略入库后恶化。

## 2) 删除候选或实验（写 · 必须先问人）

```bash
skz experiment delete <experiment_id> <strategy_code>
skz experiment delete-run <experiment_id>
```

`experiment delete` 永久删除单个尚未入库的候选回测产物。`delete-run` 删除整次探索的所有候选和执行目录，不可恢复；它不是省略 strategy code 的同一命令。

两级护栏：

- 有实盘更新任务正在运行：硬拒绝，`--force` 无效，只能等待。
- 执行目录最近仍有写入：软护栏。先用 `experiment list` 查证，把结果交给用户并取得第二次明确确认，才可加 `--force`。

已保存入库的策略是自包含资产，不会因删除来源实验而停止运行。删除后候选会从 `experiment strategies` 和 review matrix 消失，但实验原始汇总可能保持不变。

写不重试。结果不确定时先读回：单个候选查 `experiment strategies <id>`，整次实验查 `experiment list`。目标仍存在，且用户再次确认后，才允许重试一次。

## 3) 保存入库（写 · 不花钱 · 必须先问人）

```bash
skz promote start <experiment_id> <strategy_code>   # -> {promotion_id,status:"running",phase:"queued",...}
skz promote get <promotion_id>                      # 只按 status 轮询到 succeeded / failed
```

保存入库不收费，但会消费候选并创建暂停态策略。调用前把候选的关键证据、核心假设和失败信号摆给用户；确认必须绑定本次 experiment id 和 strategy code。

`status=running` 表示请求已经受理；`phase=queued|dispatching|realtime_running` 都是非终态，不能据此重提同一个 promote。只以 `status=succeeded|failed` 判断终态。`42905` 是当前用户 promotion 队列已满，`50301` 是任务状态存储不可用；两者都按 `retry_later` 处理，但写请求不自动重放。

向用户明确说明：这一步不收费，会把候选保存进实盘库并预热实时结果；入库后固定是 `暂停` 态，不会自动交易。切 `实盘` 是之后由 `skz-strategy` 处理的独立决定，需要再次确认。

命令受理后会消费候选回测产物：该 code 会立即从候选详情和 review matrix 消失，`experiment list.strategy_count` 同步减少。这是成功受理的正常生命周期，不是候选丢失。即使后台任务最终失败，已登记的暂停态策略仍可能在策略库，按 `promote get.error` 处理，不能重发原候选。

保存前先记下候选阶段独有的信息：experiment id、指标窗口、关键夏普/回撤、逐年表现、多头/空头占比和入库理由。`promote get` 成功后切换到 `skz-strategy`：

1. 用 `strategy get/list` 确认资产存在。
2. 把刚才保留的候选证据追加到 memo。
3. 检查策略构造、持仓方向和 `recent-eval`。
4. 保持暂停观察；不要把本次入库确认延伸成实盘许可。

不要依赖 `promote start --memo` 完成交接：后端只在首次插入时写 memo，复用已有策略记录时会静默忽略。默认在 promotion 成功后由 `skz-strategy` 单独追加 memo。

## 一个典型任务

「探索跑完了，帮我看看有没有能保存的」：

```bash
skz experiment list
skz experiment get <id>
skz experiment strategies <id>
skz experiment review-matrix <id>
# 只对通过候选比较逐年、跨时段、多空占比和指标窗口
# 永久删除候选或实验前，展示精确范围并取得确认
# 保存前展示候选证据、代价及“入库仍暂停”，取得确认后：
skz promote start <id> <strategy_code>
skz promote get <promotion_id>
# succeeded 后切换到 skz-strategy，确认入库、追加 memo、开始暂停观察
```
