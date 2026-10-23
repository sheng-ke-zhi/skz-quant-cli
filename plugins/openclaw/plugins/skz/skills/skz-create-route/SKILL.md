---
name: skz-create-route
description: 把投资想法定义并创建为胜可知（Shengkezhi）因子路线：打磨七字段、按机制查重、选择创建入口并读回 code。当用户说「创建因子路线」「新建 route」「把这个投资逻辑建成路线」「这条路线是否建重」时使用。研究方向取舍及付费挖掘归 skz-guide，标的、市场、频率与时间分段归 skz-create-problem，既有路线赠予和删除归 skz-factor。
---

# skz 技能 · create-route（定义并创建因子路线）

本册把已经选定的投资想法落成一条可复用路线，终点是**建好的路线与它的 code**。后端接受格式不代表逻辑成立；空洞正文照样可能被后续几十分钟的付费挖掘反复使用。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。展示路线、申请确认和解释结果前读取 [references/communication.md](references/communication.md)。

公共安装、工具和技能职责见 [运行契约](references/operating-contract.md#公共安装工具与职责)。任务跨边界时切换技能，不猜另一册的契约。

**该不该建、建哪条**由 `skz-guide` 判断；本册负责**怎么建对**。不追问标的、市场、频率或时间分段，那些属于 `skz-create-problem`。不触发 `mine start` / `explore start`，不删除旧路线；既有路线的日常资产查询、赠予与处置交给 `skz-factor`；本册可只读查重与写后核验。建路线免费，不需要 wallet check。

## 两个写入口，一份账本

| 命令 | 端点 | CLI 返回 | 什么时候用 |
|---|---|---|---|
| `skz factor-routes create` | `POST /research/factor-routes` | `{code, route_code}`，两者同值 | 默认入口，按内容哈希幂等 |
| `skz route create` | `POST /strategy/routes` | `{routeCode}` | 需要与 guide 既有流程逐字一致，或现有脚本按 `routeCode` 取字段 |

两者使用相同字段正文。依据 issue #20 记录的**单账号实测**：`route adopted` 有 11 条，`factor-routes list` 有 68 条，前者 11 个 code 全在后者里，交集 11/11，**差集为空**。该观察支持两个入口写入**同一份路线账本**、code 共用的结论，不是对后端实现的审计；创建接口的幂等语义与返回字段名仍有差异，不能互相推断。

`mine start --route <code>` 可以使用任一入口建出的 code；CLI 的 `preflight_route` 统一用 `factor-routes list` 免费预检。`route adopted` 只是已采用子集，不能用它判定账号有没有某条路线。

**参与内容哈希的 5 个字段**是 `name`、`key_inspect`、`economic_logic`、`why_effective`、`market_mechanism`，仅当它们完全一致时，研究面入口的内容幂等才成立。**改了其中任何一个就是新路线**，后端会给新 code；同一套逻辑换个说法也不会被幂等挡住，仍须按机制查重。`failure_scenarios` / `tags` 不参与哈希，可后续补充不等于 CLI 提供 update 命令；不要为补字段自动重发创建。`compute_engine` 可选，默认 `TimeSeriesAstEngine`。

## 七字段成色标准

七字段都要准备完整；这比后端最低必填要求更严格，目的是在花钱前把研究逻辑讲清楚。

| 对人说 | 字段 | 完成标准 |
|---|---|---|
| 名字 | `name` | 优先用市场现成的叫法，让人能直接判断是否研究过；别用长标题掩盖逻辑 |
| 看什么 | `key_inspect` | 可观察、可计算的现象，写明信号、比较对象与约束；「动量有效」不是观察对象 |
| 钱从哪来 | `economic_logic` | 回答「这现象为什么存在」：谁因什么约束或行为站在对手盘，谁付出成本 |
| 为什么还在 | `why_effective` | 回答「这钱为什么没被套利掉」：持续存在的成本、风险、制度或反应时滞是什么 |
| 哪类机制 | `market_mechanism` | 使用下节分类，不能拿自创的行为描述当枚举 |
| 什么时候失效 | `failure_scenarios` | 非空字符串数组；每条写出可识别的反例、机制消失或成本反超条件 |
| 标签 | `tags` | 字符串数组，可空；用于检索，不能替代正文逻辑 |

`economic_logic` 与 `why_effective` 是**两问，不是一句拆两半**。例如「机构调仓分批执行，价格缓慢反映资金流」解释现象；「风控限额与成交冲击让套利盘无法当天抹平」才解释为什么还能赚。漏掉后半问最常见，不能把「资金推动价格」在两个字段换词复述。

`failure_scenarios` 是在花钱前救你的机会：想不出它怎么死，等于还没理解它为什么活。「震荡市失效」太泛；「突破后量能快速萎缩，价格三日内跌回突破位」才可核查。

## `market_mechanism` 白名单与归并

**`market_mechanism` 必须逐字等于这 10 个值之一**，禁组合、禁加后缀、禁自创、禁近义改写：

> `错误定价` · `风险补偿` · `行为偏差` · `流动性溢价` · `制度性套利` · `自我实现预言` · `微观结构` · `信息扩散` · `趋势跟踪` · `套保压力`

真实失败案例：❌ `投资者反应不足`（后端 `INPUT_VALIDATION` 拒绝）→ ✅ `行为偏差`。常见归并：`散户追涨` → `行为偏差`；`机构调仓被迫接盘` → `流动性溢价`；`套保盘必须出场` → `套保压力`。归并是建议，最终要由人判断，不能悄悄改动哈希字段。

**route 没有 meta 发现端点**，不能照抄 `skz problem meta` 的发现方式。这份清单是写死在技能正文里的，后端加了值它不会自己更新。三条出路：

- 别自创：写错由后端 400 挡下 → `fix_params`，不扣费；确认是明确拒绝后，按共同契约修正正文并重新展示、重新确认。
- 怀疑过期时反查账号实际在用值，不能用样本更小的 `route adopted`：

  ```bash
  skz factor-routes list | jq -r '.items[].market_mechanism' | sort -u
  ```

- 出现了表里没有的值，**以真实返回为准**；离线脚本的拒绝不证明线上值非法，暂停核对并报告清单过期，不能随意找一个近义值替代。账号没出现某值也不证明该值非法。

## 建前查重：比机制，不比名字

```bash
skz factor-routes list
```

先按 `market_mechanism` 对照，再并排比较 `key_inspect`：**同机制下是否真的观察到了不同的东西**。名字不同不代表收益来源不同；只是周期或参数不同，复用已有 code，后续是否再挖由 `skz-guide` 另行取得付费许可。

疑似重复时，把已有与拟建路线的机制、观察对象、对手盘和失效条件并排给人定夺。不要自作主张删旧建新；删除会级联处置执行目录，属于 `skz-factor` 的独立确认范围。

## 离线预检

准备好待展示的 `route.json` 后，可运行：

```bash
python3 scripts/route_precheck.py --file route.json
# 或：python3 scripts/route_precheck.py < route.json
```

脚本只做离线静态校验：七字段及类型、当前机制清单、可选引擎；未知字段与过短的逻辑、失效场景和过长名称给 warning。归并建议不会自动改正文。输出始终包含 `approved:false` 与 `approval_required:true`，**绝不代替用户批准**；只有 `valid:true` 才有 `normalized`，仍须核对 warning 与实际提交字段，不能把未知字段直接带入请求。

脚本不联网、不写盘，非法 JSON 或静态校验失败退出 2。它检查格式和明显薄弱处，不能证明经济逻辑成立，也不能证明没有重复路线。

## 创建命令（写 · 不花钱 · ⚠️ 必须先问人）

**⚠️ HITL：调用 `skz factor-routes create` 或 `skz route create` 前，用人话完整展示本次待建的每条路线，七个字段（名字、看什么、钱从哪来、为什么还在、哪类机制、什么时候失效、标签）都要有，并说明查重结果；取得明确许可后才调用。内容有修改后，必须重新展示并重新确认。即使用户说「直接建」或「别确认了」也不得跳过展示与许可；guide 的逃生门只能省第①步的追问。**

确认是因为路线会成为反复复用的资产：糊弄正文不会立刻报错，会让后续挖掘持续产生看起来正常、实际无意义的结果。

默认入口，提交人已确认的原文：

```bash
skz factor-routes create < route.json
```

若选择兼容 `routeCode` 的入口，**只执行这一种入口**，不能两个都执行：

```bash
skz route create < route.json
```

可用的七字段正文示例（只作打磨参考，不能代替本次展示与确认）：

```json
{"name":"放量突破","key_inspect":"价格突破前20期高点，成交量高于前20期均量，观察突破后量价是否持续确认",
 "economic_logic":"机构分批建仓形成持续资金需求，信息逐步反映到价格上，而非单日完成定价",
 "why_effective":"成交冲击与机构风控限额限制资金一次到位，套利者承担假突破和信息反转风险",
 "market_mechanism":"趋势跟踪","failure_scenarios":["突破后量能快速萎缩，价格三期内跌回原突破位"],"tags":["量价","突破"]}
```

## 写后确认与未知结果

保存真实回执中的 `code`（研究面 `code` / `route_code` 同值，策略面是 `routeCode`），再用 `skz factor-routes list` 按 code 精确找回本次路线，核对七字段；只看到同名路线不够。字段缺失或正文不一致时暂停人审，不能自动接受或删后重建。

**写命令一律不自动重试，不要重发。** 超时、中断或回执缺失代表结果未知，CLI 已用 `into_write_unknown("skz factor-routes list")` 给出只读核验方式（exit 7 / `check_existing`）。即使研究面有内容幂等，也不是重发许可证；先照 `remediation.verifyWith` 查询。查不到、读失败或拿不准就把正文原样报给人，同时说明哪些证据缺失，保留未知，不尝试另一个入口补建。

## 创建前自检三问

1. 看什么、谁付钱、为什么没被套利掉，是否分别答清？
2. 失效场景能否被识别，同机制现有路线是否已经覆盖？
3. 人是否看过最终七字段并明确批准，写后如何按真实 code 核对？

本册面向时序量价 CTA / 择时研究；股票截面选股与截面多空不在此流程内。
