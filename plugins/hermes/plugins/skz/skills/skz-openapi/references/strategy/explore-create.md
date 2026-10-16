---
title: 创建策略探索任务
description: 胜可知开放平台 POST /strategy/explore：基于研究问题编码与路线编码创建策略探索任务。
source: https://docs.shengkezhi.com/api/strategy/explore-create
---
# 创建策略探索任务

**`POST /strategy/explore`** — 基于研究问题编码与路线编码创建策略探索任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 计费

接口调用成功并返回 `fcRunId` 后会产生费用，计费价格见[产品定价](https://docs.shengkezhi.com/pricing)。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `problemCode` | string | 是 | 问题编码，由[创建研究问题](problems.md)接口返回 |
| `routeCode` | string | 是 | 路线编码，由[创建因子路线](routes.md)接口返回 |

## 响应

`{ fcRunId, status }`

## 特殊状态码

| 状态码 | 说明 |
| --- | --- |
| 402 | 余额不足或扣费被拒 |
| 409 | 该问题与路线对应的任务正在执行中 |
| 503 | 扣费服务不可用或研究数据后端未就绪 |

409 是接口参考的冲突描述，并非已隔离验证的服务端锁保证。客户端探索逻辑键为 `(problemCode, routeCode)`，不同问题配相同路线是不同组合，但不能据此保证服务端允许并发。冲突后停止触发，仅以可信回执/账本 ID 核对问题和路线后轮询；没有可信 ID 不猜任务、不重发。字段缺失与接管规则见 [运行契约](../operating-contract.md#研究任务键与可信接管)。

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/explore" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "problemCode": "PRB_20260724_001",
    "routeCode": "RT_20260724_001"
  }'
```

```json
{"fcRunId":"fc-run-20260724-explore001","status":"running"}
```
