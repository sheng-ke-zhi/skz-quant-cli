---
name: skz-openapi
description: 胜可知（Shengkezhi）开放平台 API 参考——直接调用 api.shengkezhi.com/open/v1 的 HTTPS+JSON 接口（市场数据 /market/*、策略任务 /strategy/*、投研 /research/*），以及给用户返回 quant.shengkezhi.com Web App 页面链接。当用户问「开放 API」「直接调接口」「Bearer Token sk_xxx」「API Key 限流/多 IP」「帮我拼一个 curl / 请求体」「给我策略页/组合页链接」时使用。走 skz CLI 能做的事优先用对应 skz-* 技能；本技能面向 API 级操作与链接直达。
---

# skz 技能 · openapi（开放 API 与站点地图）

胜可知开放平台的 HTTP 接口参考与 Web App 页面地图。按需加载，不要整包读入。

## 通用规则（调用任何接口前）

Base URL `https://api.shengkezhi.com/open/v1`，HTTPS + JSON（UTF-8），认证头 `Authorization: Bearer sk_xxx`。限流 500 次/分钟（429 RATE_LIMITED），每 Key 每日最多 5 个不同 IP（403 TOO_MANY_IPS，北京时间 0 点重置）。Key 在 Web App「开放平台」页（`/app/open-platform`）生成，刷新即旧 Key 失效。

首次使用先读 [references/overview.md](references/overview.md)；报错排查读 [references/errors.md](references/errors.md)；端到端调用样例读 [references/examples.md](references/examples.md)。

## 路由表

references/ 下每个接口一个文件，frontmatter 标注 `endpoint` 与原文 `source`。按任务域选读：

| 任务域 | 目录 | 典型场景 |
|---|---|---|
| 市场基础数据 | `references/market/` | 市场列表、标的分页查询、交易日历、期货当前合约批量解析 |
| 策略任务 | `references/strategy/` | 创建因子路线 / 研究问题、因子挖掘任务（创建/列表/轮询）、策略探索（创建/查进度/列表/批量轮询） |
| 投研读写 | `references/research/` | 因子、策略、组合、实验、任务、gift 等全部读写接口（文件名前缀 get/post/patch/delete 对应 HTTP 方法） |

research 域常用入口：实盘与实验策略族 `research/get-strategies*.md` 与 `research/get-experiments*.md`；因子族 `research/get-factors*.md`、`research/get-mining-*.md`；组合族 `research/get-portfolios*.md`；赠予 `research/*gifts*.md`；写操作（创建/导入/promote/打标/memo/状态/删除）一律 `post-*` / `patch-*` / `delete-*` 文件。

## 站点地图（给用户页面链接时）

读 [references/sitemap.md](references/sitemap.md)。路由提取自 Web App JS bundle，含公开页与 `/app/*` 全部路由（如 `/app/strategy/<code>` 策略详情、`/app/experiment/<id>`、`/app/portfolio/<code>`、`/app/chat?thread=<id>` 等）及重定向别名。策略 code 形如 `STS_1D_O3FSW2D0`（前缀 + 周期 + 随机串），以 API 返回值为准拼链接。

## 边界

- Key 是机密：不写入日志、不在给用户的示例中回显完整 Key。
- 写操作（POST/PATCH/DELETE）有真实副作用（扣费任务、删除资产、领取 gift），拼好请求后先向用户确认再执行。
- 用户只是要用 CLI 功能时，改用对应 skz-* 技能，不走裸 API。
