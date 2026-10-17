---
name: skz-openapi
description: 胜可知（Shengkezhi）开放平台 API 参考——直接调用 api.shengkezhi.com/open/v1 的 HTTPS+JSON 接口（市场数据 /market/*、策略任务 /strategy/*、投研 /research/*），以及给用户返回 quant.shengkezhi.com Web App 页面链接。当用户问「开放 API」「直接调接口」「Bearer Token sk_xxx」「API Key 限流/多 IP」「帮我拼一个 curl / 请求体」「给我策略页/组合页链接」时使用。走 skz CLI 能做的事优先用对应 skz-* 技能；本技能面向 API 级操作与链接直达。
---

# skz 技能 · openapi（开放 API 与站点地图）

胜可知开放平台的 HTTP 接口参考与 Web App 页面地图。按需加载，不要整包读入。

先读 [沟通约定](references/communication.md)，按任务需要加载接口参考。

九册分工：`skz-wallet` 负责资金和费用；`skz-guide` 负责研究导航、因子挖掘与策略探索；`skz-auto-research` 负责已确认计划的端到端批量编排与报告；`skz-create-problem` 负责定义研究问题；`skz-factor` 负责因子资产；`skz-candidate` 负责实验、候选和保存入库；`skz-strategy` 负责已入库策略与实盘更新；`skz-portfolio` 负责组合；`skz-openapi` 负责开放 API 与页面直达。任务跨边界时切换到对应技能，不要在当前册猜另一册的契约。

## 通用规则（调用任何接口前）

Base URL `https://api.shengkezhi.com/open/v1`，HTTPS + JSON（UTF-8），认证头 `Authorization: Bearer sk_xxx`。限流 500 次/分钟（429 RATE_LIMITED），每 Key 每日最多 5 个不同 IP（403 TOO_MANY_IPS，北京时间 0 点重置）。Key 在 Web App「开放平台」页（`/app/open-platform`）生成，刷新即旧 Key 失效。

首次使用先读 [references/overview.md](references/overview.md)；报错排查读 [references/errors.md](references/errors.md)；端到端调用样例读 [references/examples.md](references/examples.md)。

无论直接 API 还是 CLI，调用前都先读 [references/operating-contract.md](references/operating-contract.md)，继承身份、凭据、只读策略、人工确认及未知写结果边界；直接 API 另见其中「直接 API 调用边界」。需要向用户解释结果或申请写操作确认时再读 [references/communication.md](references/communication.md)。

因子批量软删除见 [references/research/delete-factors.md](references/research/delete-factors.md)：HTTP 200 仍可能逐项失败，必须检查批量回执。

## 路由表

references/ 下每个接口一个文件，frontmatter 标注 `description` 与原文 `source`，正文首行给出方法与路径。按任务域选读：

| 任务域 | 目录 | 典型场景 |
|---|---|---|
| 市场基础数据 | `references/market/` | 市场列表、标的分页查询、交易日历、期货当前合约批量解析 |
| 账户与计费 | `references/payment/` | 钱包余额与可用额度 |
| 策略任务 | `references/strategy/` | 创建因子路线 / 研究问题、因子挖掘任务（创建/列表/轮询）、策略探索（创建/查进度/列表/批量轮询） |
| 投研读写 | `references/research/` | 因子、策略、组合、实验、任务、gift 等全部读写接口；文件名前缀仅辅助定位，方法与路径以正文为准，读写按实际副作用判定 |

references/ 中另含全 skill 共享的 [运行契约](references/operating-contract.md) 等契约文件，直接 API 和 CLI 均适用。

research 域常用入口：实盘与实验策略族 `research/get-strategies*.md` 与 `research/get-experiments*.md`；因子族 `research/get-factors*.md`、`research/get-mining-*.md`；组合族 `research/get-portfolios*.md`；赠予 `research/*gifts*.md`。创建/导入/promote/打标/memo/状态/删除等按端点行为及运行契约处理，不用文件名推断授权。

## 按实际副作用判定

HTTP 方法不等于读写或确认策略。先查对应端点参考，再按运行契约判断是否需确认；以下是本地参考明确的反例，不是全量白名单：

| 端点参考 | 实际行为与授权边界 |
|---|---|
| [PUT 模型配置更新](references/strategy/llm-config-update.md) | 修改配置或模型凭据，属于写操作；执行前确认精确配置 ID 和本次变更 |
| [POST 任务轮询](references/strategy/tasks-poll.md) | 只读查询已有任务，不创建任务，不因 POST 要求写授权 |
| [POST 钱包查询](references/payment/wallet-summary.md) | 只读查询当前身份的钱包，不扣费，不因 POST 要求写授权 |
| [POST 任务预估](references/strategy/tasks-preview.md) | 只读预估与重复提示，不创建任务，不因 POST 要求写授权；预估成功不代表已授权创建 |
| [POST 模型连接测试](references/strategy/llm-config-probe.md) | 使用模型凭据，可能消耗用户模型额度；不是普通只读查询，调用前确认 |

只读是指不发起扣费、不修改业务资产，不承诺服务端零内部副作用（例如既有任务超时结算与退款）。其他 PUT/PATCH/POST/DELETE 或 GET 同样按实际行为判定；端点语义、费用或影响未知时先暂停核实，不按 HTTP 方法猜定，也不通过试调用探测。

## 站点地图（给用户页面链接时）

读 [references/sitemap.md](references/sitemap.md)。路由提取自 Web App JS bundle，含公开页与 `/app/*` 全部路由（如 `/app/strategy/<code>` 策略详情、`/app/experiment/<id>`、`/app/portfolio/<code>`、`/app/chat?thread=<id>` 等）及重定向别名。策略 code 形如 `STS_1D_O3FSW2D0`（前缀 + 周期 + 随机串），以 API 返回值为准拼链接。

## 边界

- Key 是机密：不写入日志、不在给用户的示例中回显完整 Key。
- 付费、不可逆资产、用户模型凭据等操作按运行契约取得相应许可；标签、正常 memo 等可自主写操作仍受只读策略与写结果核验约束，不扩大授权范围。
- 用户只是要用 CLI 功能时，改用对应 skz-* 技能，不走裸 API。
