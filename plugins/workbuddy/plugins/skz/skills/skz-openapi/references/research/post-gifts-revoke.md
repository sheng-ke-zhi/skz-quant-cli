---
title: 撤回投研资产赠予
description: 胜可知开放平台 POST /research/gifts/revoke：撤回投研资产赠予。
source: https://docs.shengkezhi.com/api/research/post-gifts-revoke
---
# 撤回投研资产赠予

**`POST /research/gifts/revoke`** — 撤回投研资产赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `gift_code` | string | 是 | — |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `revoked` | boolean | 是 | — |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts/revoke" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"gift_code":"GIFT-8K3M2P"}'
```

```json
{
  "code": 0,
  "data": {
    "revoked": true
  },
  "msg": "ok"
}```
