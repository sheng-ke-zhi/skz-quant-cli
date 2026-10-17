---
title: 测试自定义大模型连接
description: 胜可知开放平台 POST /strategy/llm-configs/probe：使用已保存配置或临时凭证测试模型连接。
source: https://docs.shengkezhi.com/api/strategy/llm-config-probe
---
# 测试自定义大模型连接

**`POST /strategy/llm-configs/probe`** — 使用已保存配置或临时凭证测试模型连接。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 副作用与授权

使用用户模型凭据并可能消耗模型额度，不是普通只读查询。调用前确认精确配置 ID 或临时配置及影响；只读策略下停止，不自动重试，见 [运行契约](../operating-contract.md#直接-api-调用边界)。

## 请求

可传 `configId`；也可直接传 `apiKey`、`baseUrl`、`model`。使用 `configId` 时可覆盖 `baseUrl` 或 `model`。

## 响应

成功时返回 `ok`、`message`、`model`、`reply`、`stopReason`、`inputTokens`、`outputTokens`。连接失败返回 `LLM_CONNECTIVITY_FAILED`。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/llm-configs/probe" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"configId":"1b91e8e7-84ed-4f03-99ea-97f850555a40"}'
```

```json
{
  "ok": true,
  "message": "Claude 协议请求成功（model-pro）",
  "model": "model-pro",
  "reply": "ok",
  "stopReason": "end_turn",
  "inputTokens": 8,
  "outputTokens": 2
}
```
