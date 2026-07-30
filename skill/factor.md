---
name: skz-factor
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上的因子资产——浏览/筛选/排序因子库、看某次挖掘 run 挖出了什么、查单因子跨时段评估、软删不成立的因子。当用户提到「我的因子库」「挖出来的因子」「因子表现/夏普」「这次挖矿结果」「清理/删因子」，或要在挖矿跑完后查看成果时使用。不负责触发挖矿本身（那是 skz-guide）。
---

# skz 技能 · factor（因子管理）

挖矿的**产出**在这册看。两层，别混：

- **成果柜** = 某一次挖掘 run 挖出了哪些因子（`skz mining *`，按 `run_id` 索引）
- **因子库** = 跨所有 run 沉淀下来的全部因子资产（`skz factor *`，按 `factor_name` 索引）

除软删外全是只读。

<!-- COMMON -->

## 因子库：跨 run 的因子资产

```bash
skz factor summary                           # 概览：总数/已删/评估数 + 引擎、路线、标签分布（含各路线 TOP 因子）
                                             # 注意 top_factors[].sharpe 其实是 mean_sharpe（实测核对过），不是 best
                                             # total_routes 可能大于 route_distribution 的元素数——差额是
                                             # 「已建但还没挖出因子」的路线（刚建的 route），不是字段错乱
skz factor list \
  [--q 关键词] [--route <code>] [--engine <e>] [--tag <t>] \
  [--sort 夏普比率] [--order desc] [--include-deleted] [--page 1] [--page-size 50]
skz factor get <factor_name>                 # 详情：factor_code 表达式 + tags(含 QC 明细) + 64 条 evaluations
skz factor-routes list                       # 因子路线（挖矿方向）清单，供 --route 取 code
```

`factor list` 一条 item 长这样（真实字段）：

```json
{"factor_name":"TSA_260718_O599IPR2",
 "factor_code":"ma = Mean($close, 120)\nbias = Div(Sub($close, ma), Add(ma, 1e-6))\n…",
 "compute_engine":"TSA","engine_full":"TimeSeriesAstEngine","description":"乖离…",
 "agg":{"best_sharpe":0.957,"mean_sharpe":0.155,"median_sharpe":0.090,
        "median_calmar":0.040,"problem_count":64,
        "pos_sharpe_ratio":null,"best_problem":null}}   // ← 这两个在 factor list 里恒 null
```

**`problem_count: 64` 不是 64 个问题**——是 **16 个问题 × 4 个方法**（`MA001`/`TS001`/`TS002`/`TS003`）。所有 agg 统计都摊在这 64 行上。这条很重要：**同一个问题下换个方法，夏普能从 -0.7 摆到 +1.5**，所以 `pos_sharpe_ratio` 低不必然等于"跨问题不稳"，也可能是方法间分歧。别把小幅差异过度解读成不稳定。

> **`best_problem` / `evaluations[].problem` 里那种 `FTS_PROBLEM_D_xxxxxxxx` 编码查不到**：它们是挖矿内部做跨问题验证的**基准问题**，跟 `problem list` 里你自己建的研究问题（`STS_QS_LEADERS` 这类）是**两套命名空间**。拿它去 `skz problem get` 必回 **404 / exit 2 fix_params**——那不是你参数写错，是这套编码根本没暴露查询入口，**别在这上面反复试**。只能从前缀认品类：`F`=期货 / `S`=股票 / `E`=ETF。

- **⚠️ `metrics` 不是"单次评估"——它就是 `agg` 的中文键别名。** 实测全库 2029 个因子：`metrics["夏普比率"] == agg.mean_sharpe`，**零个不相等**。所以别指望从 `metrics` 拿到"某一次评估"的值——**真正的单次评估只在 `factor get` 的 `evaluations[]` 里**（64 条，每条带 `problem`/`method`）。`metrics` 是中文键松散 map，CLI 原样透传。
- **两个端点的 `agg` 形状不同,不是父子集**：`factor list` 的 agg 有 7 个键（含真值 `median_calmar`，但 `pos_sharpe_ratio`/`best_problem` 为 null）；`mining factors` 的 agg 只有 6 个键（**没有 `median_calmar`**，但那两个是真值）。缺字段时换另一个端点试。
- `--sort` 收中文指标键（`夏普比率`）或字段名（`best_sharpe`）；`--order` 缺省 `desc`；`--page-size ≤ 200`。
- **别只看 `best_sharpe`**：它是该因子在 64 次评估里的**峰值单次**，天然有挑选偏差——`best_problem` 也只是"峰值出自哪个问题"，**不代表它在那个问题上稳定地好**（实测有因子的 best_problem 按该问题多次评估平均是负的）。判稳定性看 `mean_sharpe` / `median_sharpe` / `pos_sharpe_ratio`（正夏普占比）。

> **⚠️ `factor list` 的 `agg.pos_sharpe_ratio` 与 `agg.best_problem` 恒为 `null`**（实测；`best/mean/median_sharpe` 有值）。要这两个字段只能**按 run 问**：`mining factors <run_id> [--q <name>]`。给 `factor list --sort pos_sharpe_ratio` 不报错（exit 0）但**静默不起作用**，因为字段全空。
> **而且它比"无效"更坏**：实测 `--sort pos_sharpe_ratio` 的 **`asc` 与 `desc` 返回完全相同**，且**跟默认序也不同**——落到了第三种确定性顺序。单看一次调用会以为排序生效了。**怀疑某个 `--sort` 没生效时，对拉一次 `asc` 与 `desc`：结果相同 = 这个键没起作用。**
> 库级排序可用的是 **`mean_sharpe` / `median_sharpe` / `best_sharpe`**——`factor list --sort mean_sharpe --order asc` 是找"跨评估最差尾部"最省事的一条命令。

**`factor get` 没有 `agg`**——它只给 64 条原始 `evaluations`（每条 `problem/method/sharpe/calmar/segments/status`）。要 mean/median/正比例得自己从这 64 条算，别指望它回 `factor list` 里那份摘要。另外 agg 只有 `best_problem`、**没有对称的 `worst_problem`**，最差情况得自己扫 evaluations。

`factor get` 的 **`tags` 里藏着平台自己的质检明细**（`detect_passed`/`positive_passed`，含各训练段与人类可读原因）——判稳健性很好用的第二意见。但有三个坑：

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
  [--pos-min 0.5] [--page 1] [--page-size 20]
```

> **`run_id` 的两种形态**：新挖的 run，成果柜里的 `run_id` **就是 `fcRunId` 本身**（32 位 hex，如 `ad3907d6c59b43c4be7a29546c978335`）；早期 run 是 `<route>_<n>_<日期>_<时间>` 格式。两种都能直接喂给 `mining overview/factors`，**别去拼格式**——从 `mining runs` 拿现成的 `run_id`。

> **⚠️ `mine` 和 `mining` 是两块完全不同的后端，名字却几乎撞车——这是最容易叉出去的地方。**
> `skz mine *`（动词，`/strategy/miner/*`）= **任务台**：`mine start` 触发挖矿（扣费）、`mine runs`/`mine poll` 看**进度**。
> `skz mining *`（动名词，`/research/mining/*`）= **成果柜**：`mining runs`/`overview`/`factors` 看**挖出了什么**，只有这边带 `agg` 跨问题统计。
> 陷阱：猜着用 `mine runs` 会拿到一个**看起来完全合理**的运行列表（不报错），但那条路上没有 `overview`/`factors` 这层数据，很容易拿着错的列表还不自知。**要看成果，一律走 `mining`。**

**两个实测过的坑：**

1. **分页三个端点三种上限**：`factor list --page-size` 最大 200；`mining factors --page-size` 最大 100，CLI 会在请求前拦下 101 以上；`mining runs` 没有分页 flag，它无条件全量返回。**永远比对 `len(items)` 与 `total`**，不等就翻页。
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

skz factor get <factor_name>                      # 4. factor_code 表达式 + tags 里的平台质检 + 64 条 evaluations
# 5. 判据（几条同时成立才算"该删"，别单条定罪）：
#    · mean_sharpe 与 median_sharpe **都为负**（不是偶尔差，是中心趋势就不行）
#    · pos_sharpe_ratio 明显偏低（正夏普占比小；只有 mining factors/factor get 有这个字段）
#    · tags 里平台质检的 ratio_train_full 没过（阈值 0.618；比自己拍阈值可靠）
#    再回到 factor_code 看逻辑站不站得住——数据难看 + 逻辑也说不通 → 才提议删
# 6. ⚠️ 列出短名单和理由给人，问过之后再 delete；别自己删
```

**参照系比阈值重要**：先看一眼头部因子（`--sort pos_sharpe_ratio --order desc`）长什么样，再判断尾部差到什么程度算异常。硬套一个绝对阈值容易在不同 route 上失准。

> **⚠️ 别拿 `best − mean` 差距当过拟合证据**（两次独立实测都否掉了它）：
> - 它**结构性地对每个因子都很大**（`best` 是 64 次里的峰值），全库中位数就在 0.78 左右——区分不出任何东西。
> - 更要命的是**方向会骗你**：差距小往往不是"更少挑肥拣瘦"，而是**这批的天花板更低**（`best` 拉不上去）。把它读成利好正好读反。
> - 而且 `evaluations[].segments` 里全是训练集段，**没有任何 held-out 段**——所以这个差距反映的是训练期内 64 次相关抽样的离散度，跟"训练 vs 样本外"的泛化差距根本不是一回事。
> 判过拟合要用 `median_sharpe` / `pos_sharpe_ratio` / QC 的 `ratio_train_full`。

## 常见状态

- **新库/刚挖完索引未生成** → 研究面可能回 **exit 5 retry_later**（净值/统计还没算完），退避再来，别当失败。
- **`factor get` 一个不存在的名字** → **exit 2 fix_params**（改 name，别重试）。
- **空结果不是错误**：`{"total":0,"items":[]}` + exit 0 意味着这个账号确实还没有因子——去 `skz skills guide` 先挖。
