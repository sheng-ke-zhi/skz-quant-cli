---
title: 创建投研资产赠予
description: 胜可知开放平台 POST /research/gifts：创建投研资产赠予。
source: https://docs.shengkezhi.com/api/research/post-gifts
---
# 创建投研资产赠予

**`POST /research/gifts`** — 创建投研资产赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `max_claims` | integer | 是 | — |
| `ttl_days` | integer | 否 | — |
| `asset_codes` | string[] | 是 | — |
| `asset_type` | `GiftAssetType` | 是 | — |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `asset_codes` | string[] | 是 | — |
| `asset_type` | `GiftAssetType` | 是 | — |
| `claim_records` | `GiftClaimRecord`[] | 是 | 已成功领取该码的明细（发出方视角）。 |
| `claimed` | integer | 是 | — |
| `created_at` | string | 是 | — |
| `expires_at` | string | 是 | — |
| `gift_code` | string | 是 | — |
| `max_claims` | integer | 是 | — |
| `status` | `GiftOutStatus` | 是 | active / revoked / expired（expired 由 expires_at 读时推导）。 |
| `ttl_days` | integer | 是 | — |
| `unavailable_asset_codes` | string[] | 是 | — |

### GiftAssetType 取值

| 取值 | 说明 |
|---|---|
| `problem` | - |
| `factor_route` | - |
| `strategy` | - |


### GiftClaimRecord 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `asset_type` | `GiftAssetType` | 是 | — |
| `claimed_at` | `string` | 是 | — |
| `from_user_id` | `string` | 是 | — |
| `gift_code` | `string` | 是 | — |
| `items` | `GiftClaimItem[]` | 是 | — |
| `recipient_user_id` | `string` | 是 | — |


### GiftOutStatus 取值

| 取值 | 说明 |
|---|---|
| `active` | - |
| `revoked` | - |
| `expired` | - |


### GiftClaimItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `inserted` | `boolean` | 是 | — |
| `origin_code` | `string` | 是 | — |
| `renamed` | `boolean` | 是 | — |
| `target_code` | `string` | 是 | — |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"asset_type":"strategy","asset_codes":["STS_60M_2GPCQGXW"],"max_claims":10,"ttl_days":3}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "asset_type": "strategy",
    "asset_codes": [
      "STS_60M_2GPCQGXW"
    ],
    "claimed": 0,
    "created_at": "2026-07-01T08:00:00Z",
    "expires_at": "2026-07-01T08:00:00Z",
    "gift_code": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
    "max_claims": 1,
    "status": "active",
    "ttl_days": 3,
    "unavailable_asset_codes": [],
    "claim_records": []
  }
}```
