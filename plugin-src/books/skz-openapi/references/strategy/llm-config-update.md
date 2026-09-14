---
title: 更新自定义大模型配置
description: 胜可知开放平台 PUT /strategy/llm-configs/{id}：修改指定配置的名称、密钥、地址或模型。
source: https://docs.shengkezhi.com/api/strategy/llm-config-update
---
# 更新自定义大模型配置

**`PUT /strategy/llm-configs/{id}`** — 修改指定配置的名称、密钥、地址或模型。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求

至少提供 `name`、`apiKey`、`baseUrl`、`model` 中的一项。未提供 `apiKey` 时保留原密钥。

## 响应

返回更新后的配置：`id`、`name`、`apiKeySuffix`、`baseUrl`、`model`、`createdAt`、`updatedAt`。不会返回 API Key 明文或密文。

## 调用示例

```bash
curl -X PUT "https://api.shengkezhi.com/open/v1/strategy/llm-configs/1b91e8e7-84ed-4f03-99ea-97f850555a40" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"model":"model-pro-v2"}'
```

```json
{
  "id": "1b91e8e7-84ed-4f03-99ea-97f850555a40",
  "name": "Claude Compatible",
  "apiKeySuffix": "8xYz",
  "baseUrl": "https://llm.example.com",
  "model": "model-pro-v2",
  "createdAt": "2026-09-13T10:00:00+08:00",
  "updatedAt": "2026-09-13T10:10:00+08:00"
}
```
