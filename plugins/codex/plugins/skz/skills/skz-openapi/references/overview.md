---
title: 开放平台概述
description: 胜可知开放平台 API 概述：Base URL、Bearer Token 认证、JSON 数据格式、限流与每日 IP 上限。
source: https://docs.shengkezhi.com/api/overview
---

# 开放平台概述

胜可知开放平台提供一组 HTTPS + JSON 接口，用于访问国内金融市场基础数据、创建策略研究任务以及查询账户下的投研任务。所有接口共用同一套认证与调用限制规则。

## 基本信息

| 项 | 值 |
| --- | --- |
| Base URL | `https://api.shengkezhi.com/open/v1` |
| 协议 | HTTPS |
| 数据格式 | JSON，UTF-8 |
| 认证 | Bearer Token |

## 接口前缀

开放平台按接口类型分为三个前缀：

| 前缀 | 说明 |
| --- | --- |
| `/market/*` | 市场基础数据 |
| `/strategy/*` | 策略研究任务，包括因子路线、研究问题、因子挖掘与策略探索 |
| `/research/*` | 投研接口 |

## 认证方式

认证请求头固定使用：

```text
Authorization: Bearer sk_xxx
```

- Key 在「开放平台」页一键生成，每个账户一把，可随时刷新；刷新后旧 Key 立即失效。

> **密钥安全（警告）**
> Key 仅限本人使用；多人或多设备共用同一把 Key 会达到「每日 IP 上限」，超限请求将被拒绝。

## 调用限制

| 限制 | 默认 | 超限响应 | 说明 |
| --- | --- | --- | --- |
| 每分钟限流 | 500 次/分钟 | `429 RATE_LIMITED` | 每把 Key 独立计数，非全局累计 |
| 每日 IP 上限 | 5 个不同 IP/天 | `403 TOO_MANY_IPS` | 防止密钥共享；**北京时间每日 0 点重置**。当天出现第 N+1 个新 IP 即被拒 |

## 在线调试

左侧每个接口页都内置了**在线调试台**：填入你的 Key、参数与 JSON 请求体，点「发起调试」即可看到真实响应，并可一键复制等效 curl 命令。Key 仅用于当前页面的请求，不会写入浏览器本地存储。

## 接下来

- 市场数据 → [市场列表](market/markets.md)、[标的分页查询](market/symbols.md)、[交易日历](market/trading-calendar.md)、[批量解析期货当前合约](market/post-market-data-future-contracts-resolve.md)
- 策略接口 → [创建因子路线](strategy/routes.md)、[创建研究问题](strategy/problems.md)、[已采用路线列表](strategy/routes-adopted.md)
- 因子挖掘 → [创建因子挖掘任务](strategy/miner-create-run.md)、[因子挖掘任务列表](strategy/miner-list-runs.md)、[批量查询挖掘进度](strategy/miner-poll.md)
- 策略探索 → [创建策略探索任务](strategy/explore-create.md)、[查询策略探索进度](strategy/explore-get.md)、[策略探索记录列表](strategy/explore-list-runs.md)、[批量查询探索进度](strategy/explore-poll.md)
- 投研接口 → [研究问题列表](research/get-problems.md)、[因子列表](research/get-factors.md)、[因子挖掘产出记录](research/get-mining-runs.md)、[策略研究执行列表](research/get-experiments.md)、[实盘策略列表](research/get-strategies.md)、[任务列表](research/get-worker-tasks.md)
- 排查报错 → [错误码](errors.md)
- 完整调用示例 → [使用示例](examples.md)
