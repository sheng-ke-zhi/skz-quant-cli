# 场景 B · 用现成因子路线解决一个新研究问题

**输入**：一个新的研究问题（用户描述，或已建好的 problem code）。
**产出**：在若干条既有因子路线上的探索结果 + 一份 HTML 汇总报告。

```
B0 前置检查(只读)
 → B1 列候选路线，让用户勾选      [用户确认]  ← 按库存/历史成功率/适配度排序分档
 → B2 在选定路线上批量 explore    [付费许可]
 → B3 抓取 → HTML 报告 → 指到策略审核页
```

与场景 A 的区别：**不建路线、不挖因子**（路线已有因子），全部价值在"选哪几条路线"和"结果怎么读"。

确定性执行优先 `scripts/run_pipeline.sh`：计划设 `scenario:B`、`routes:[已确认code...]`、
`problems:[已确认code...]`，至少一条路线与一个问题，明确批准路线×问题的全部组合数量。
既支持多路线单问题，也支持一条既有路线复用至多个问题；始终 explore-only，不用 A 的 times=0 绕行。
默认只读预检；人审批准与预算齐全且 --yes-paid 才触发。
不走旧 orchestrate 付费入口。完整参数及可选一次补跑见 [cli-contract.md](cli-contract.md) 第 9 节。
await 全部探索终态→fetch→报告验证，按宿主能力展示真实进度并通过附件/文件展示机制交付，提交不等于结束。

---

## B0 前置检查（只读）

```bash
skz auth status
skz whoami
skz wallet balance
skz explore runs --status active
skz problem list                  # 确认这个新问题是否已经建过（同名/同标的族可能已存在）
skz problem get <code>            # 若已建：拿 dataset / freq / symbols[] / problem_type
```

**问题还没建？** 先按场景 A 的 A1 建（`skz problem meta` → 展示 → 确认 → `problem create`）。
新问题定边界时注意：

- `problem_type` 选 `TimeSeriesProblem`（平台主战场是时序量价择时）；
- 标的 **≤10 个且应高相关**（时序问题），标的代码带市场后缀；
- 五段时间分段齐全，且 `edt` 不晚于 `meta.max_time_segment_date`；
- description 里**显式写明"后置验证区间不参与训练/选品/调参/模型选择"**——这是研究计划，不等于实际执行证据；报告仍需核实使用情况，不能据此直接认证真正样本外。

---

## B1 列候选路线（本场景的核心判断）

### 取数四步（照顺序，别跳）

```bash
skz factor summary                       # ① 路线级：factor_count / avg_sharpe / top_factors[].sharpe
skz explore runs --page 1 --size 100     # ② 翻页到 total：按 routeCode 计数 = 已探索次数
skz experiment list                      # ③ 唯一同时带 problem_code + pass_rate + strategy_count
skz problem get <新问题code>              # ④ dataset / freq / symbols_count（做适配过滤）
```

### 打三分，分三档

对每条 `factor_count > 0` 的路线打分：

| 维度 | 判据 | 来源 |
|---|---|---|
| 产能 | `factor_count` 量级（1k 以下偏薄、1k–1w 常态、1w+ 库存厚） | ① |
| 质量 | `top_factors[].sharpe`（**该路线出过的最好因子**；`avg_sharpe: null` ≠ 差） | ① |
| 变现度 | 已探索次数（次数低 + 因子多 = 库存待变现，性价比最高） | ② |
| 适配度 | 新问题的 `problem_type` 下，该路线是否出过 `strategy_count > 0` | ③ |
| 反面信号 | 同 `type/dataset/freq` 上 `pass_rate` 极低且 `strategy_count = 0` | ③ |

分档给用户勾：

- **优先（3–5 条）**：库存厚且探索次数低；或在新问题同类（同 dataset+freq）上已出过策略。
- **可选（5–10 条）**：产能中等、从未评估（`avg_sharpe: null`）的待变现路线。
- **不建议**：`factor_count = 0`（必失败）；同类上已验证低效的。

### 给人看的形状

```
新问题：甲醇/尿素·60分钟时序（FTS_PROBLEM_ZZZZ，2 标的，60分钟）

建议优先探索这 5 条路线（勾选后一次性发起 5 次探索，约 ¥X）：
 ① 波动率调整动量        06 750d36e232  因子 12,438  top夏普 0.63  从未探索       ★库存最厚
 ② 量价背离强度          19 d0b43e8b1a  因子  1,149  top夏普 0.61  已探 12 次     ★同类出过 5 个候选
 ...
不建议：
 ③ RF4顶底背离          53 673ab8cfd4  因子 13,311  但在 60 分钟档 8 次探索 0 候选，已验证低效
```

⚠️ 路线 code 是 12 位十六进制（如 `06750d36e232`），展示时**别省略、别加空格**，
用户点选回来必须能一一对应；报告里给全。

⚠️ 一次探索 = 一条 (problem, route) = 一次扣费。
`skz wallet check explore --qty N`（N = 勾选条数）**必须跑**，并把这个数报给用户。

---

## B2 批量探索（写 · 付费 · 真并发）

```bash
# 教学命令；实际执行用 driver，在 pipeline_state.json 中落盘 intent、attempts 与费用
skz explore start --problem <新问题code> --route <route_i>       # 逐条，分批 ≤8
skz explore poll <id1> <id2> ...                                # 60s/轮，≤100 id，判 ok
```

- 只读预检：`factor summary` 里 `factor_count` 必须 > 0，否则该条直接失败（`no_valid_factors`）。
- 确定性去重：**同一 (problem, route) 在本批已触发过就跳过**。
  用户重复下指令时，按本账本可信 ID 查任务台/poll 核验并接管，不按 createdAt 猜 ID；
  无可信 ID 就暂停，不能再触发 —— 重复触发 = 双倍扣费。
- 同一条 route 同时在跑两个会撞 `exit 7 check_existing`：去 `explore runs --status active` 找它、poll 它。
- 耗时按因子库存估（129 因子 ~2min / 1239 因子 ~6min / 11238 因子 ~23min），
  但这只是历史观察（实测 59 条 39 分钟），不能保证当前耗时或费用。前台等待并支持断点续跑；受管任务遵守宿主生命周期。

---

## B3 抓取 → 报告 → 指到审核页

抓取清单同场景 A 的 A5。报告按 [report-spec.md](report-spec.md)，
本章以**路线为行**做对比表：因子库存 / top夏普 / 该次回测数 / 通过数 / 通过率 / 候选数 / 后置验证表现。

结尾同样给审核指引：

> 到胜可知平台「策略管理 → 策略审核」页审核候选：https://quant.shengkezhi.com/
> （功能说明：https://docs.shengkezhi.com/strategy）

### 结论怎么写（结论先行）

1. **一句话**：这个问题在哪些路线上出得来策略、在哪些上彻底出不来。
2. **值得进一步审核的路线**：通过率高 + 候选后置验证表现较好（取 `review-matrix` 的「后置验证」行）；尚不能确认真正样本外。
3. **可以放弃的路线**：`pass_rate = 0` 且 `strategy_count = 0`（把探索次数也写出来，
   说明是"试过多次都没出来"而不是"没试过"）。
4. **不要只看通过率**：通过率高可能只是候选靠 beta 混过去；后置验证也仅是候选比较依据，不证明独立验证通过。
5. **局限**：全周期指标 ≠ 样本外、现金仅为参考基准且低净敞口不证明风险中性、多重检验、成本口径未知；分列验证区间、训练/筛选使用情况、入库后的 `nav.oos_start`，缺失写未知，日期新旧不证明独立性。

## 一句话对比

| | 场景 A | 场景 B |
|---|---|---|
| 变量 | 1 条新路线 × N 个问题 | N 条老路线 × 1 个问题 |
| 花钱的地方 | mine ×10（贵、慢、串行）+ explore ×N | 只有 explore ×N（便宜、快、并发） |
| 最容易犯的错 | 批量触发导致早停失效（双倍扣费） | 凭感觉推路线（该用库存×历史成功率取数） |
