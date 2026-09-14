---
title: 自定义大模型配置列表
description: 胜可知开放平台 GET /strategy/llm-configs：列出当前账户保存的自定义大模型配置。
source: https://docs.shengkezhi.com/api/strategy/llm-config-list
---
# 自定义大模型配置列表

**`GET /strategy/llm-configs`** — 列出当前账户保存的自定义大模型配置。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回配置数组。API Key 只返回尾号 `apiKeySuffix`，不会返回明文或密文。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/llm-configs" \
  -H "Authorization: Bearer sk_xxx"
```

```json
[
  {
    "id": "1b91e8e7-84ed-4f03-99ea-97f850555a40",
    "name": "Claude Compatible",
    "apiKeySuffix": "8xYz",
    "baseUrl": "https://llm.example.com",
    "model": "model-pro",
    "createdAt": "2026-09-13T10:00:00+08:00",
    "updatedAt": "2026-09-13T10:00:00+08:00"
  }
]
```
