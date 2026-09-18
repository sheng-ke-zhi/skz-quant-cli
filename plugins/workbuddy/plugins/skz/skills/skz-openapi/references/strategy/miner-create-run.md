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
| `llm` | object | 否 | 大模型选择；不传或 `mode=platform` 使用平台模型 |
| `llm.mode` | string | 否 | `platform` 或 `byok` |
| `llm.configId` | UUID | 否 | 引用[自定义大模型配置列表](llm-config-list.md)中的配置 |
| `llm.apiKey` | string | 否 | 临时使用的明文 API Key；与 `configId` 二选一 |
| `llm.baseUrl` | string | 否 | 临时配置的 Anthropic 兼容 API 地址 |
| `llm.model` | string | 否 | 临时配置的模型标识 |
| `llm.saveAs` | string | 否 | 使用临时配置时，同时保存为账户配置 |

## 响应

`{ fcRunId, taskId, status, routeCode }`；成功入队时 `status` 为 `queued`。

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
  -d '{"routeCode":"RT_20260724_001","llm":{"mode":"byok","configId":"1b91e8e7-84ed-4f03-99ea-97f850555a40"}}'
```

```json
{"fcRunId":"fc-run-20260724-abc123","taskId":"fc-run-20260724-abc123","status":"queued","routeCode":"RT_20260724_001"}
```
