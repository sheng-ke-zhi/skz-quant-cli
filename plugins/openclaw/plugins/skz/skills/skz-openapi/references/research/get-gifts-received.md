---
title: 查询我领到的赠予
description: 胜可知开放平台 GET /research/gifts/received：查询我领到的赠予。
source: https://docs.shengkezhi.com/api/research/get-gifts-received
---
# 查询我领到的赠予

**`GET /research/gifts/received`** — 查询我领到的赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | `GiftClaimRecord`[] | 是 | — |

### GiftClaimRecord 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `asset_type` | `GiftAssetType` | 是 | — |
| `claimed_at` | `string` | 是 | — |
| `from_user_id` | `string` | 是 | — |
| `gift_code` | `string` | 是 | — |
| `items` | `GiftClaimItem[]` | 是 | — |
| `recipient_user_id` | `string` | 是 | — |


### GiftAssetType 取值

| 取值 | 说明 |
|---|---|
| `problem` | - |
| `factor_route` | - |
| `strategy` | - |


### GiftClaimItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `inserted` | `boolean` | 是 | — |
| `origin_code` | `string` | 是 | — |
| `renamed` | `boolean` | 是 | — |
| `target_code` | `string` | 是 | — |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/gifts/received" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "asset_type": "problem",
        "claimed_at": "2026-09-13T10:00:00Z",
        "from_user_id": "01EXAMPLE000000000000000000",
        "gift_code": "GIFT-8K3M2P",
        "items": [
          {
            "inserted": true,
            "origin_code": "STS_MOMENTUM_001",
            "renamed": false,
            "target_code": "STS_MOMENTUM_001"
          }
        ],
        "recipient_user_id": "01EXAMPLE000000000000000000"
      }
    ]
  },
  "msg": "ok"
}```
