---
title: 创建策略探索任务
endpoint: POST /strategy/explore
source: https://docs.shengkezhi.com/api/strategy/explore-create
---

# 创建策略探索任务

`POST /strategy/explore` — 基于研究问题编码与路线编码创建策略探索任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 计费

接口调用成功并返回 fcRunId 后会产生费用，计费价格见产品定价。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| problemCode | string | 是 | 问题编码，由创建研究问题接口返回 |
| routeCode | string | 是 | 路线编码，由创建因子路线接口返回 |

## 响应

```json
{ fcRunId, status }
```

## 特殊状态码

| 状态码 | 说明 |
| --- | --- |
| 402 | 余额不足或扣费被拒 |
| 409 | 该问题与路线对应的任务正在执行中 |
| 503 | 扣费服务不可用或研究数据后端未就绪 |

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
