---
title: quant.shengkezhi.com 站点地图
source:
  - https://docs.shengkezhi.com/
  - https://docs.shengkezhi.com/getting-started
  - https://docs.shengkezhi.com/new-user-roadmap
  - https://docs.shengkezhi.com/install
  - https://docs.shengkezhi.com/account
  - https://docs.shengkezhi.com/strategy
  - https://docs.shengkezhi.com/problem
  - https://docs.shengkezhi.com/factor
  - https://docs.shengkezhi.com/portfolio
  - https://docs.shengkezhi.com/live
  - https://docs.shengkezhi.com/faq
  - https://docs.shengkezhi.com/pricing
  - https://docs.shengkezhi.com/chat
  - https://docs.shengkezhi.com/custom-llm-config
  - https://docs.shengkezhi.com/performance-metrics
  - https://docs.shengkezhi.com/strategy-states
  - https://docs.shengkezhi.com/strategy-position-weights
  - https://docs.shengkezhi.com/research-methodology
  - https://quant.shengkezhi.com/（SPA 主包与懒加载 chunk 内的路由表，2026-09 提取）
---

# quant.shengkezhi.com 站点地图

Web App 是 https://quant.shengkezhi.com 的 SPA（React + 路由懒加载），无公开 sitemap.xml，且对任意路径均返回 200 首页 HTML，无法用 HTTP 状态码验证路由。本文路由表提取自站点实际发布的 JS bundle 路由定义，可信度高；页面渲染需登录态，未逐一做浏览器级验证。

## 公开页（未登录可访问）

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `https://quant.shengkezhi.com/` | 平台首页/工作台入口，未登录时跳转登录页 | 用户问「平台地址 / 怎么进平台」 |
| `/login` | 登录页，支持「账号密码」与「手机号验证码」两种方式，可切换到注册 | 用户要登录、注册账号（注册需手机号 + 邀请码 + 短信验证码） |
| `/forgot-password` | 忘记密码入口 | 用户忘记密码 |
| `/reset-password` | 重置密码页 | 重置流程中跳转，一般不直接给 |
| `/pay/result` | 支付结果页 | 充值支付后跳转，一般不直接给 |
| `/auth/feishu/callback` | 飞书登录/OAuth 回调 | 仅集成用，不主动给用户 |

## 工作台与对话（/app 下，需登录）

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app` | 登录后的工作台主页 | 用户问「登录后去哪」 |
| `/app/chat` | AI 对话工作台，自然语言发起研究；`/app/chat?thread=<会话ID>` 直达某个会话 | 用户想发起研究对话、找回某个研究会话 |
| `/app/notifications` | 通知中心 | 用户问任务完成通知在哪看 |

## 策略相关页

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/strategy` | 策略管理页（标签组织多视角） | 用户要看策略库、策略列表 |
| `/app/strategy?tab=prob` | 策略管理 → 研究问题标签；`/app/problem` 是它的重定向别名 | 用户要管理/新增研究问题 |
| `/app/strategy?tab=exp` | 策略管理 → 实验（策略研究/探索记录）标签；`/app/experiments` 是重定向别名 | 用户要查策略探索/回测实验记录、进入策略审核 |
| `/app/strategy/<code>` | 策略详情页：构成、回测/实盘绩效、持仓权重、状态操作（暂停/开启实盘/废弃） | 用户要看某只具体策略的详情、净值、持仓、实盘分析 |

策略 `code` 形态：API 样本为 `STS_1D_O3FSW2D0`、`STS_60M_1I7G6TS4`——前缀 `STS` + 周期编码（`1D` 日线、`60M` 60 分钟线）+ 8 位随机串。文档站示例出现过 `ETS_1D_A3N8JWDB`，前缀不同（可能是旧版/实验态前缀），给链接时以 API 返回的 code 为准，不要自行拼造。

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/experiment/<id>` | 实验详情页（一次策略探索执行记录，id 为 32 位十六进制 run id，如 `a79dfc93b7e64a6cbbe26f2a787a6bad`） | 用户要复盘某次策略探索的执行记录与候选列表 |

## 因子与挖掘页

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/factor-management` | 因子管理页，默认「因子库」标签：浏览已入库因子、质量指标 | 用户要看因子库、因子表现 |
| `/app/factor-management?tab=mining` | 因子管理 → 挖掘执行历史标签；`/app/mining` 是重定向别名 | 用户要追溯历次因子挖掘任务的执行记录 |
| `/app/factor-management/factors/<factorName>` | 因子详情页 | 用户要看某个因子的定义与表现 |

因子名形态：`TSA_260820_N26M85RL`（TSA + 日期 + 随机串）。因子路线与挖掘记录请配合 API（`/strategy/routes`、`/strategy/miner/runs`）取 code。

## 组合页

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/portfolio` | 我的组合列表：多条策略组成的整体视图、组合层面净值收益 | 用户要看/建组合 |
| `/app/portfolio/<code>` | 组合详情页：构成、历史持仓、整体表现报告 | 用户要看某个组合的详情 |

## 实盘页

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/live` | 实盘运行页：观察实盘策略的真实运行状态、持仓与净值 | 用户问「我实盘最近怎么样」「实盘在哪看」 |

## 账户与开放平台页

| 路径 | 用途 | 何时给用户这个链接 |
| --- | --- | --- |
| `/app/account` | 账号中心：账户信息、改密码、大模型配置（自定义 LLM，用于因子挖掘） | 用户要改账户信息、配置自定义大模型 |
| `/app/funds` | 充值与订单：套餐充值、订单与支付记录 | 用户问充值、发票、订单、额度钱包 |
| `/app/open-platform` | 开放平台页：一键生成/刷新 API Key（`sk_` 开头，每账户一把，刷新即旧钥失效） | 用户要配置 skz CLI / API Key 时——`skz auth add` 前引导来此取 Key |
| `/app/register-success` | 注册成功落地页 | 注册流程自动跳转 |
| `/app/forbidden` | 无权限页（菜单/权限拦截） | 不主动给 |

## 备注

- 已验证：以上 `/app/*`、`/login` 等路径均直接提取自 SPA 发布的路由定义（bundle 内 `path:` 表），非猜测；因页面需登录态，未做逐页渲染验证。
- 重定向别名（真实路由 → 目标）：`/app/problem` → `/app/strategy?tab=prob`；`/app/experiments` → `/app/strategy?tab=exp`；`/app/mining` → `/app/factor-management?tab=mining`。给用户链接时优先用目标地址。
- 会话详情 ID（`/app/chat?thread=...`）不称「code」，由平台生成；研究问题唯一编码形如 `XXX_PROBLEM_XXXX`（文档说明），研究问题当前在策略管理页的「研究问题」标签下管理，未见独立详情路由。
- API Key 仅在 `/app/open-platform` 生成；API Base 为 `https://api.shengkezhi.com/open/v1`，详见 `references/overview.md`。
