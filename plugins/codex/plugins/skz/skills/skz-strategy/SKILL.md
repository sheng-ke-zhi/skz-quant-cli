---
name: skz-strategy
description: 用 skz CLI 管理胜可知（Shengkezhi）量化平台上已入库的策略资产——直接登记已验证策略、查询净值/持仓/回撤/交易明细、巡检健康度、切换实盘·暂停·废弃状态、维护 memo/tag，以及赠予或领取策略。当用户提到「我的策略」「实盘表现/净值/回撤/持仓」「上线/暂停/废弃某策略」「给策略记一笔/看当初为什么停」「登记或克隆现成策略」「赠予/领取策略」时使用。探索实验、候选评审、删除候选和保存入库由 skz-candidate 负责。
---

# skz 技能 · strategy（已入库策略管理）

这册从策略**已经入库**开始：先读 memo 和实时结果，再做巡检、状态运营、登记或赠予。探索实验、候选评审和保存入库属于 `skz-candidate`。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。需要解释结果、申请确认或给下一步时，再读取 [references/communication.md](references/communication.md)。不要把两个 reference 的正文复制回本文件。

可执行工具均为只读或纯校验：

- `scripts/preflight.py --operation read|write|paid`：检查 CLI、身份和本地写策略。
- `scripts/resume.py`：跨会话重建在途挖矿、探索和组合任务。
- `scripts/validate_plan.py`：校验付费计划；只返回 `approved:false`，绝不代替用户批准。
- `scripts/verify_write.py`：写超时后读回确认；绝不重放写命令。

五册分工：`skz-guide` 负责研究导航和付费触发；`skz-factor` 负责因子资产；`skz-candidate` 负责实验、候选和保存入库；`skz-strategy` 负责已入库策略；`skz-portfolio` 负责组合。任务跨边界时切换到对应技能，不要在当前册猜另一册的契约。

安装用 `skz plugin install <claude|codex|openclaw|hermes|all>`，状态以 `skz plugin status <target>` 的 `needs_install` 为准；升级后若报告 stale，重新安装。`skz --version` 输出 CLI 与 plugin contract，命令参数以 `skz --help` 为准。

## 1) 实盘富读（读 · 可自主）

```bash
skz strategy list [--status 实盘] [--q k] [--sort ..] [--with-metrics] [--page-size 5]
                                             # ⚠️ page-size 缺省 5（省上下文）；说多少给多少，扫全库自己调大
                                             # 每项 factor_route 是所属因子研究路线；当前策略没有 route 时为空
skz strategy get <code>                      # 详情（含 status、death_time、outsample_sdt、base_freq、description、memo）
skz strategy metrics <code>                  # 统计（中文键松散 map：夏普比率/卡玛比率/年化收益/…）
skz strategy nav <code>                      # {dates, nav, drawdown, oos_start}
skz strategy positions <code>                # 最新持仓 {items:[{dt,symbol,weight}]}（只有最近十来个 bar，见下方警告）
skz strategy latest-positions --weight-type ts|cs
                                             # 批量最新权重 {items:[{dt,symbol,weight,strategy,update_time}]}
skz strategy segments <code>                 # 分时段指标（带 is_live；见下方警告）
skz strategy periodic <code>                 # 月度/年度收益矩阵
skz strategy recent-eval <code>              # 健康度：{is_good, reason, recent{…}, recent_ok, history{…}, history_ok, params}
skz strategy definition <code>               # 策略定义（构成因子 + 参数）
skz strategy trades <code> [--year 2025] [--kind win|loss|all]
skz strategy kline <code> <kline_key>        # 单笔交易的出入场 K 线窗口
```

**`recent-eval` 先看 `reason`,不是只看 `is_good`。** `reason` 是人话结论，`recent` 给近一年指标（带 `sdt`/`edt`），`history` 给历史段，两边各有 `_ok` 布尔。这是巡检的第一眼。

> **⚠️ 别把"哪条不达标"猜错——`history_ok` 和 `recent_ok` 是两道独立的门。** 实测 `STS_1D_DSKCIB7M`：
> ```json
> {"is_good":false, "reason":"历史回撤或收益不达标",
>  "history_ok":false, "recent_ok":true,          ← 近期其实是过的！
>  "history":{"全样本回撤":0.215,"去尾历史回撤":0.215,"绝对收益":0.2204},
>  "params":{"max_dd_threshold":0.2,"recent_days":252,"target_vol":0.2}}
> ```
> 它挂在**历史回撤 0.215 > `params.max_dd_threshold` 0.2** 这条硬门槛上，跟"近期表现"无关。
> **规矩：看到 `is_good:false`，先读 `reason` 定位是哪道门，再拿 `history`/`recent` 的数值对 `params` 里的阈值验算**——别看到近期数字难看就当成失败原因（我自己就先错了一次）。
> 顺带一个判断回撤性质的技巧：**`去尾历史回撤` 若等于 `全样本回撤`，说明回撤是结构性的**（去掉最坏一段也降不下来）；若明显更小（例：另一策略 0.104 → 0.039），那是**单次可去除的事件**。两者严重程度完全不同。
>
> **⚠️ 候选说好、入库说不好，未必是"表现变了"——可能是两边量的根本不是同一个回撤。** 实测 `STS_1D_DSKCIB7M`：候选侧 `verdict.history_alpha_max_drawdown` = **0.195**（**相对 alpha/基准**的回撤）→ 过；入库侧 `recent-eval.history.全样本回撤` = **0.215**（**绝对**回撤）→ 超 `max_dd_threshold` 0.2 不过。**同一个策略、两种风险定义、两个相反结论。** 所以别把入库后的 `is_good:false` 直接理解成"入库后恶化了"，先看清两边各在量什么。

**`nav.oos_start`（样本外起点，如 `"2025-01-01"`）是全平台最该看的字段之一**——它把"真held-out证据"和"样本内曲线"划开。`oos_start` 之前的净值再漂亮也是拟合出来的；要判断策略行不行，只看这条线之后。`strategy get` 里同一个日期叫 **`outsample_sdt`**（同一件事、两个名字，别当成无关字段）。

> **⚠️ `segments` 只有 `is_live:true` 那段有数,其余段是**零填充**,不是真 0。** 实测（n=3，含刚入库的和一周前的，所以**不是重算延迟**）：`训练集A/B/C段`、`训练集` 全为 `0`，只有 `后置验证`（`is_live:true`）有真值。
> 成因：这个端点只在**该策略自己被跟踪的 nav 窗口**（实测都是 2023-01 起）上算，落在窗口外的命名段直接**零填充**（而不是给 null 或省略）。
> **`天数` 是唯一可靠的判据**：`天数=0` = 这段根本没算，`天数=484` = 真有数据。（值本身多数是**字面 0**、个别是 `null`，两种混着出现，所以不能靠"是不是 null"判断。）**不看 `天数` 就对各段夏普取平均，会得到一个大错的数。** 真实的历史分段指标在 `experiment review-matrix` 里。

**`strategy get` 的 `description` 是唯一能看到多空构造的地方**（自动生成的自由文本，形如 `filter = all_positive_top_n -> TS003`）。它解释策略**为什么**在赚或在亏——比如 `all_positive_top_n` 配上"正值代表高估"的因子，实际是**单边做空**，遇到该板块不配合就一路失血。**判断一个策略的死因,先读它。** 注意：候选阶段（`experiment strategies`）只暴露 `model`、**不给 `filter`**，所以多空构造 **入库之后才看得到**——这正是"入库先挂 `暂停`、立刻 `strategy get` / `positions` 验一眼构造，再谈 `实盘`"的原因。

**`--with-metrics` 会额外注入 `metrics` 和 `nav_preview`**（`dates`/`nav`/`drawdown`/`oos_start`），批量巡检时一次拿齐，省掉逐个 `nav`。**而且它给的字段比 `strategy metrics` 还多**（实测 19 vs 17，多出 `单笔收益`/`持仓K线数`）——同名 `metrics`、两个端点两套字段集，缺字段时换另一个端点试试。

> **各端点覆盖的时间窗不一样，别跨端点拼时间序列**（实测同一策略）：`segments` 实际只覆盖 2023+（更早的零填充）、`nav` 是 2023–2026、而 `trades` 只到 2024。**每次比较前先核对各自的日期范围**，否则会把不可比的切片放在一起。
>
> **⚠️ `positions` 不在上面这条里——它根本不是时间序列，只回最近固定几个 `dt`。** 2026-07-31 抽 4 个策略实测**全是 10 个 `dt`**（`ETS_60M_8V1K2X4M` 2 标的、`STS_1D_DSKCIB7M` 5 标的、`ETS_15M_HHHE2DXC` 4 标的、`ETS_1D_3RJZ2JMT` 5 标的）；更早一次 n=2 观察到的是 3 个，**这个条数别写死进脚本，读 `items` 里实际有几个 `dt` 就是几个**。
> **关键不是几个，是那是 bar 不是天**：10 个 bar 对 1D 策略约两周、对 15M 策略只有半天，跨度差三个数量级。且最新的 `dt` 等于该策略自己的 `latest_weight_date`，不是今天——**暂停的策略拿到的就是一个月前的**。
> 端点**没有日期/翻页参数**，更早的逐标的持仓从这里拿不到，别指望用它拼长期敞口曲线；要长期方向看 `experiment strategies` 的 `metrics.多头占比/空头占比`。排序是 `dt` 倒序、同日内 `symbol` 升序。
> **⚠️ `weight` 是每个标的的信号仓位，不是组合占比**——一篮子加总可达标的数倍（5 个标的合计 −400% 是常态），单标的实测上界 ±1.0。**别把它当归一化权重求和当"净敞口"，也别把 >100% 当成杠杆异常。**（官方文档只写「最新持仓权重明细列表（按标的排序）」，没说是几条，也没展开 `PositionItem` 的字段。）

> **`latest-positions` 是另一种批量读法**：`ts` 返回每个时序策略各标的自身最新的一行，所以同一策略的 `dt` 可以不同；`cs` 返回每个截面策略最新完整截面，所以同一策略各行 `dt` 相同。它返回所选类型的全部策略行，不收 strategy code，也不分页；零权重是有效状态，不能过滤。`update_time` 是写入时刻，`dt` 才是权重日期。

> **时间字段叫 `update_time`（不是 `promoted_at`/`updated_at`——那两个名字不存在）**，有真值（如 `2026-07-26T01:20:41+08:00`，已换算成东八区）。但它只是"最后一次变更"的时间戳，**不记录变更内容**，也没有 audit-log 类命令——**所以还是查不到"为何被暂停"**。看到 `暂停` 态别假设是"还没上线"，也可能是人有意停的；要切 `实盘` 前先问清当初为什么停。
> 另外 `recent_update` 是**嵌套对象**（`recent_update.last_heartbeat` / `.latest_weight_date`），不在顶层——按顶层读会静默拿到 `None`。
> **⚠️ 别假设 `暂停` 态一定还在算数据**：实测两个暂停策略的 `last_heartbeat` 停了 7 天、`latest_weight_date`/nav 停了约 30 天。**判断数据新不新，看 `recent_update.latest_weight_date` 与 `nav` 最后一个日期**，别默认它是活的——拿着一个月前的净值下结论会出事。
- `trades` 的每条带 `kline_key`（形如 `601688.SH|2016-09-28T16:00:00|2016-11-11T16:00:00`），直接喂给 `strategy kline` 看那笔交易的 K 线。
- 新用户实盘库为空时列表回 `{"items":[],…}` + exit 0（**不是错误**）；某策略净值还没算完回 **exit 5**，退避再看。

> `--status` 仅接受 **`实盘` / `暂停` / `废弃`** 三个中文值；CLI 会在请求前校验，大小写或错别字立即 `fix_params` / exit 2。
> 好在 `strategy list` 的 **`market_distribution` 不受 `--status` 影响**，永远给你各状态的真实计数——**拿它当交叉验证**：`items` 空但分布里有数，就是你的 `--status` 传错了，不是库空。
> 它是**按市场分组的数组**（别当成扁平 map，那是另一个字段 `status_counts`）：
> ```json
> "market_distribution": [{"market":"A股","total":3,"实盘":0,"废弃":0,"暂停":3}]
> ```
> `strategy trades --kind` 仅接受 `win|loss|all`，CLI 会在请求前校验；分页则正常，`strategy list --page-size` 说多少给多少。

## 2) 状态运营（写 · 不重试）

```bash
skz strategy status <code> --status <实盘|暂停|废弃>
skz strategy tag-add <code> --tag <t>        # 可自主
skz strategy tag-rm <code> <t>               # 可自主
echo "笔记正文" | skz strategy memo <code>   # 可自主
```

三个状态的自主边界**不一样**，别一视同仁：

| 切到 | 判定 | 说明 |
|---|---|---|
| `实盘` | **⚠️ 必须先问人** | 真金从这一刻开始上场——比 `废弃` 还重 |
| `暂停` | 可自主 | 降风险、可逆的安全阀。发现策略异常（`recent-eval.is_good=false`、回撤破限）可以先踩刹车再报告。**但平台不留痕**（status 不收 `reason`、无 audit log），所以你自己踩的刹车要**当场在对话里说清原因和当时的数，并同时写进 `memo`**（对话会散，笔记留在资产上，见 §5）——否则下次（可能是别人、也可能是失忆的你）看到这个 `暂停` 态，无从知道是"还没上线"还是"出事停的"，那道决定就卡住了 |
| `废弃` | **⚠️ 必须先问人** | **不可逆**：后端写 `death_time` 并进写保护，要「复活」才能恢复 |

**其他铁律：**

- 枚举**只有** `实盘 | 暂停 | 废弃`（CLI 本地校验，写错立即 exit 2 不发网络；后端判非法回 exit 7）。
- **status 没有 `reason` 字段**——后端契约不收原因，别指望在这里留痕。要记原因用 `memo`（长文，见 §5）或 `tag-add <code> --tag 废弃:过拟合`（短标签、可筛选），那都是**另一个动作**，得单独调；**改完状态顺手补上，别留一个没人知道为什么的状态。**
- 写不重试：撞 5xx（exit 5）也别盲重试，先 `strategy get` 看当前状态再决定。

### 直接登记 `register`（写 · 必须先问人）

```bash
# 正常路径：克隆一条已验证的策略，改一处，重新登记
skz strategy definition STS_1D_OLD \
  | jq '.strategy="STS_1D_NEW" | .model_config.model="TS005"' \
  | skz strategy register

skz strategy register mystrategy.toml                     # 单个 JSON/TOML 文件
skz strategy register strategy-a.toml strategy-b.toml     # 多个文件一次批量登记
skz strategy register < mystrategy.toml                   # 不传文件时从 stdin 读一份
```

**这不是研究流程的入口。** 正常做研究走 `skz-guide`，探索完成后由 `skz-candidate` 评审并保存入库；那条路上的策略带着回测证据。`register` 是直接把一份或一批定义写进实盘库，**不跑回测**，进去就是 `暂停` 态且**没有任何指标**——`strategy metrics` / `nav` / `segments` 都是空的。

所以它只有一个正当用途：**克隆或迁移一条已经验证过的策略**。要"试个新想法"就去走 explore，别用这个。

- **问人时要说清它跟保存候选入库的区别**：`skz-candidate` 的保存入库针对跑过回测的候选；register 是把定义直接塞进库、跳过全部评估。人容易以为两者等价。
- **输入 JSON 或 TOML 都行**，CLI 自动嗅探。文件参数可以给 1–100 个；不传文件时只从 stdin 读一份。`strategy definition <code>` 的输出**就是**合法的 JSON 输入形态——它返回的正好是后端要求的七个字段：`strategy` / `problem` / `runtime` / `model_config` / `post_process` / `route` / `factors`。少任何一个 CLI 本地就 exit 2，不发网络。
- **JSON 里的 `null` 会被丢弃**（TOML 表示不了空值，实测 `problem.suffix` 就是 null）。对后端无影响——它只读认识的键——但你要知道上传的内容与 `definition` 的输出不是逐字节相同。
- **批次边界**：单份转换后的 TOML 最多 1 MiB，整批最多 10 MiB。CLI 会先全量读取和校验，再发一次请求。
- **整批原子写入**：任一份定义非法或同批次出现重复策略编号，整批拒绝；所有新策略在同一个事务中登记，不会只成功前半批。
- **同名不覆盖**：库里已存在的策略在逐项回执里是 `inserted:false`，且**什么都不改**；同批次其他新策略仍正常登记。回执顶层给 `total` / `inserted` / `existing`，`items` 与输入文件顺序一致。
- **登记成功后逐条补 memo**（见 §3）：每条新策略都没有回测、没有指标、没有实验，`memo` 是它**唯一的来源说明**——不写，它在库里就是一条无从解释的资产。
- 写不重试。超时是 exit 7，照 `verifyWith` 跑 `skz strategy list`，按预期的每个策略 code 逐一核对；批量请求可能已整批落库，**没核清前别重发**。

## 3) 做笔记（`memo`）—— 默认动作，不是可选项

```bash
echo "2026-07-31 暂停：近 20 日回撤 -18%，超过预设 -15% 阈值，等下周复盘" \
  | skz strategy memo STS_1D_XXXX
skz strategy memo STS_1D_XXXX --clear      # 清除已有笔记
```

**平台不留痕**：status 不收 `reason`、`update_time` 只说"最后变过"不说变了什么、没有 audit log，策略侧还**没有任何字段指回它出自哪次探索**（`strategy get`/`list` 只有 `problem_code`，没有 experiment id——想回溯得逐个实验翻 `experiment strategies` 对 code）。而对话会散、你会失忆、下次来看这个策略的可能是别人。

`memo` 是这条缝上唯一的补法。所以规矩不是"可以写"，是**每次你对某个策略做出判断，就在它上面留一行**。

### 该写什么：判据只有一条——这条信息以后还查不查得回来

| 查得回来 → 别写 | 查不回来 → 必写 |
|---|---|
| 夏普/回撤/年化、nav、持仓、交易明细：随时可重查，而且**会变**，抄进 memo 只会过期 | **你的判断和理由**：为什么停、为什么上、为什么留着不删 |
| 平台通识（后置验证其实是样本内、同名夏普不同窗）：那是这本技能的事，别复制进每条 memo | **做判断时看的是哪个数、哪段窗口**：数会变，你当时的依据不会重现 |
| 你跑了哪几条命令的流水账 | **候选阶段独有的信息**：`verdict.yearly_metrics`、`多空占比`、出自哪个 experiment id——入库后策略侧一个都没有 |
| | **否定结论**：试过什么、为什么放弃。这是最容易被重复劳动的一类，而平台上没有任何地方记它 |

### 什么时候写（这几处是默认动作，不用问人）

| 时机 | 写什么 |
|---|---|
| `promote` 入库成功后 | 入库理由 + 出自哪个实验 + 候选侧关键数（**把窗口一起写上**，如"全样本 2017–2024 夏普 0.95"）。过了这一刻就得翻实验才找得回 |
| `register` 登记后 | 从哪克隆、改了什么、原策略验证到什么程度。它没有任何指标，这是唯一的来源说明 |
| 你自己踩 `暂停` | 触发的阈值 + 当时的数。对话里说清之外**还要写进去** |
| 切 `实盘` / `废弃` 前后 | 人拍板的理由，连同你摆给人看的那几个证据 |
| 观察期得出结论 | `recent-eval` 挂在哪道门（`reason` + 你对 `params` 阈值的验算）、回撤是结构性还是单次事件 |
| 踩到口径坑 | 这个策略的哪个数骗过你（例：`segments` 只有 `is_live` 那段是真值），免得下次再被同一个数骗 |

### 格式：一行一条、日期开头、新的追加在最后

```
2026-07-20 入库：出自实验 EXP_xxx，候选侧全样本(2017–2024)夏普 0.95、多空占比 0.43/0.50，均衡略偏空
2026-07-31 暂停：recent 夏普 0.12、历史回撤 0.215 破 max_dd_threshold 0.2，等下周复盘
```

它是这个策略的履历，正序读得下来。**日期写东八区当天的绝对日期，别写"今天""上周"**——读的人不知道你是哪天写的。

### 追加要先读回来（它是覆盖写）

```bash
{ skz strategy get STS_1D_XXXX | jq -r '.memo // ""'
  echo "2026-07-31 暂停：近 20 日回撤 -18%（阈值 -15%），等下周复盘"
} | skz strategy memo STS_1D_XXXX
```

原来为空时前面那个空行会被 trim 掉，不用特判。**别直接 `echo 新内容 | memo`**——那会把之前所有笔记一次抹掉，且不可恢复。

### 巡检第一眼读 memo，不是读指标

`strategy list` 和 `strategy get` 都返回 `memo`，扫全库一次列表就够，不用逐个 `get`：

```bash
skz strategy list --page-size 50 \
  | jq -r '.items[] | select(.memo != "") | "\(.code) [\(.status)] \(.memo)"'
```

**写的全部价值在于有人读。** 开工前先扫一遍：看到 `暂停` 先看它有没有说为什么停；看到已经否掉的方向，别再做一遍。

**其余边界：**

- **正文走 stdin，不是参数**——笔记有换行和标点，走参数要在 shell 里转义，容易被截断成半句。
- 上限 **10000 个字符**（按 Unicode 字符计，不是字节；中文一个字算一个），超了 exit 2、不发网络。**快满时压缩旧条目**——删掉那些可以重查的数、留下结论，别整段截掉。
- **stdin 为空报 exit 2，不会当成"清除"。** 清除必须显式 `--clear`，且清之前先 `strategy get` 看一眼当前内容——抹掉不可恢复。
- 写不重试。exit 7 就跑 `skz strategy get <code>` 看 `memo` 到底写没写进去，别盲重发。
- **笔记存在平台上、也可能被别人看到**：别往里写 token、账号或任何凭据。

## 4) 策略赠予（`gift`）—— 把实盘策略复制给别人 / 从别人那里领

```bash
# 送方
skz gift create --strategy STS_1D_A --strategy STS_1D_B --max-claims 3 --ttl-days 7
# → {"gift_code":"<32位小写hex>","strategy_codes":[...],"max_claims":3,"claimed":0,
#    "ttl_days":7,"created_at":"...+08:00","expires_at":"...+08:00","unavailable_strategy_codes":[]}
skz gift list                      # 我发出的、还没过期的码（claimed / unavailable 都是现算的）
skz gift revoke <gift_code>        # 撤回：只挡住还没领的人

# 收方
skz gift preview <gift_code>       # 零副作用：里面有哪几条、能不能领、剩几个名额
skz gift claim <gift_code>         # → {"from_user_id":"...","items":[{origin_strategy_code,strategy_code,inserted,renamed}]}
```

**语义是复制，不是转移。** 领方在自己库里得到一份独立副本；送方事后删除或废弃**不影响已经领走的副本**。

**⚠️ 赠予码就是策略的访问凭证。** 拿到码的人不需要别的授权就能领走这几条策略的**完整定义**。所以：

- **发码前必须问人**，且要问清四件事：给谁、给哪几条、几个人（`--max-claims`，按去重人数）、几天（`--ttl-days`，只能 1/3/7）。
- **发出即不可撤回地披露**——`revoke` 只挡得住还没领的人，已经领走的收不回来。
- **别把码贴进公开渠道、issue、日志或提交信息**。跟用户口头给码就行，不要顺手写进文件。

**领取方要知道的：**

- **先 `preview` 再问人**：`claimable` 为 false 时不要直接 `claim` 去撞（`items[].reason` 会说是哪条不可用）。`already_claimed:true` 说明自己领过了——再 `claim` 会**原样回放上次结果**，不会重复拷贝、也不会多占名额。
- **落地即在册，且删不掉**：副本进的是自己的实盘库，状态固定 `暂停`，要真上场得自己 `strategy status --status 实盘`（那是另一个必须问人的决定）。实盘库没有删除命令，进来了就只能改状态——所以 `claim` 之前要问人。
- **回执里 `strategy_code` 才是本地编号**，不是 `origin_strategy_code`。跟自己库里已有的编号撞名且内容不同时，后端会加 `_G{n}` 后缀（`renamed:true`）；内容一致则判为已有，`inserted:false`、什么都不写。**后续所有 `skz strategy *` 都用 `strategy_code`。**
- **带过来的是定义 + 实盘绩效 + 历史目标权重，不带 memo / tags**。所以领完**顺手补一行 memo**（见 §3）：写清这条是从谁那里领的、什么时候、为什么领——不写，它在库里就是一条没有来历的资产。

**整码要么全领、要么全不领**：送方在你领之前删了或废弃了其中任意一条，整个码不可领（exit 7，`message` 点名是哪条），**且不扣名额**；他把那条改回非废弃状态，码就又活了。这时正确动作是**去找送方**，不是重试。

**两个 409 长得像、动作相反**（都是 exit 7，看 `remediation`）：

- 「名额已用尽」→ 重发一万次也一样，**没有 `--force` 可越**（跟删除类命令的软护栏不是一回事）。去找送方另发一个码。
- 「正在领取中」→ 并发抢同一个码，退避几秒重发同一条命令即可，本次没落库也没占名额。

## 一个典型任务（照着改）

「看看这条暂停策略能不能上实盘」：

```bash
skz strategy get <strategy_code>             # 1. 先读 memo、状态、数据新鲜度与 description
skz strategy nav <strategy_code>             # 2. 核对窗口与真正样本外表现
skz strategy positions <strategy_code>       # 3. 看近期方向；不要把零权重算成空头
skz strategy recent-eval <strategy_code>     # 4. 按 reason 找失败门，再用 params 验算
# 5. 把结论和本次依据追加进 memo
# 6. 观察够了、人明确拍板后，才切实盘：
skz strategy status <strategy_code> --status 实盘
# 7. 状态变化后再追加一行 memo，记录谁拍板、凭哪些证据
```

`暂停` 是真金之前的观察窗口。读 `description` 的 filter、看近期持仓方向、按 `recent-eval.reason` 验算门槛，并把结论写进 memo；任何一项解释不通，就把证据交给用户，不提切实盘。
