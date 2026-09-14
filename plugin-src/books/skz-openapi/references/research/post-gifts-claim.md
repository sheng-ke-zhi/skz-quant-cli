---
title: 领取投研资产赠予
description: 胜可知开放平台 POST /research/gifts/claim：领取投研资产赠予。
source: https://docs.shengkezhi.com/api/research/post-gifts-claim
---
# 领取投研资产赠予

**`POST /research/gifts/claim`** — 领取投研资产赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `gift_code` | string | 是 | — |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `asset_type` | `GiftAssetType` | 是 | — |
| `from_user_id` | string | 是 | — |
| `items` | `GiftClaimItem`[] | 是 | — |

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
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts/claim" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"gift_code":"GIFT-8K3M2P"}'
```

```json
{
  "code": 0,
  "data": {
    "asset_type": "problem",
    "from_user_id": "experiment_20260701_001",
    "items": [
      {
        "inserted": true,
        "origin_code": "STRAT_MOMENTUM_001",
        "renamed": true,
        "target_code": "STRAT_MOMENTUM_001"
      }
    ]
  },
  "msg": "ok"
}```
