---
title: 创建因子挖掘任务
description: 胜可知开放平台 POST /strategy/miner/runs：创建因子挖掘任务。
source: https://docs.shengkezhi.com/api/strategy/miner-create-run
---

# 创建因子挖掘任务

**`POST /strategy/miner/runs`** — 创建因子挖掘任务。同一用户同一路线编码存在执行中任务时，将返回 `409`。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 计费

接口调用成功并返回 `fcRunId` 后会产生费用，计费价格见[产品定价](https://docs.shengkezhi.com/pricing)。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `routeCode` | string | 是 | 路线编码，由[创建因子路线](routes.md)接口返回 |

## 响应

`{ fcRunId, status, routeCode }`

## 特殊状态码

| 状态码 | 说明 |
| --- | --- |
| 402 | 余额不足或扣费被拒 |
| 409 | 该路线正在执行中 |
| 503 | 扣费服务不可用或研究数据后端未就绪 |

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/miner/runs" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"routeCode":"RT_20260724_001"}'
```

```json
{"fcRunId":"fc-run-20260724-abc123","status":"running","routeCode":"RT_20260724_001"}
```
