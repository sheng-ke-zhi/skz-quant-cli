---
title: 新增自定义大模型配置
description: 胜可知开放平台 POST /strategy/llm-configs：保存一条 Anthropic 协议兼容的大模型配置。
source: https://docs.shengkezhi.com/api/strategy/llm-config-create
---
# 新增自定义大模型配置

**`POST /strategy/llm-configs`** — 保存一条 Anthropic 协议兼容的大模型配置。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求

`name`、`apiKey`、`baseUrl`、`model` 均为必填。API Key 至少 5 个字符。

## 响应

返回新配置：`id`、`name`、`apiKeySuffix`、`baseUrl`、`model`、`createdAt`、`updatedAt`。不会返回 API Key 明文或密文。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/llm-configs" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"name":"Claude Compatible","apiKey":"sk-provider-key","baseUrl":"https://llm.example.com","model":"model-pro"}'
```

```json
{
  "id": "1b91e8e7-84ed-4f03-99ea-97f850555a40",
  "name": "Claude Compatible",
  "apiKeySuffix": "8xYz",
  "baseUrl": "https://llm.example.com",
  "model": "model-pro",
  "createdAt": "2026-09-13T10:00:00+08:00",
  "updatedAt": "2026-09-13T10:00:00+08:00"
}
```
