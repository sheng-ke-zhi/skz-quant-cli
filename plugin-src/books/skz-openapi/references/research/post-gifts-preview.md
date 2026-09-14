---
title: 预览投研资产赠予
description: 胜可知开放平台 POST /research/gifts/preview：预览投研资产赠予。
source: https://docs.shengkezhi.com/api/research/post-gifts-preview
---
# 预览投研资产赠予

**`POST /research/gifts/preview`** — 预览投研资产赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `gift_code` | string | 是 | — |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `already_claimed` | boolean | 是 | — |
| `asset_type` | `GiftAssetType` | 是 | — |
| `claim_reason` | string \| null | 否 | — |
| `claim_status` | `GiftClaimStatus` | 是 | — |
| `claimable` | boolean | 是 | — |
| `expires_at` | string \| null | 否 | — |
| `from_user_id` | string | 是 | — |
| `items` | `GiftPreviewItem`[] | 是 | — |
| `remaining_claims` | integer | 是 | — |
| `resumable` | boolean | 是 | — |

### GiftAssetType 取值

| 取值 | 说明 |
|---|---|
| `problem` | - |
| `factor_route` | - |
| `strategy` | - |


### GiftClaimStatus 取值

| 取值 | 说明 |
|---|---|
| `new` | - |
| `pending` | - |
| `done` | - |


### GiftPreviewItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `available` | `boolean` | 是 | — |
| `description` | `string` | 是 | — |
| `name` | `string` | 是 | — |
| `origin_code` | `string` | 是 | — |
| `reason` | `string` \| null | 否 | — |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts/preview" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"gift_code":"GIFT-8K3M2P"}'
```

```json
{
  "code": 0,
  "data": {
    "already_claimed": false,
    "asset_type": "strategy",
    "claim_reason": null,
    "claim_status": "new",
    "claimable": true,
    "expires_at": "2026-09-16T10:00:00Z",
    "from_user_id": "01EXAMPLE000000000000000000",
    "items": [
      {
        "available": true,
        "description": "中频动量策略",
        "name": "动量策略",
        "origin_code": "STS_MOMENTUM_001",
        "reason": null
      }
    ],
    "remaining_claims": 9,
    "resumable": false
  },
  "msg": "ok"
}```
