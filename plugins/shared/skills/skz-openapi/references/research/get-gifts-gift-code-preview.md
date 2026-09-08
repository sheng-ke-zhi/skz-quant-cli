---
title: 预览策略赠予
description: 胜可知开放平台 GET /research/gifts/{gift_code}/preview：预览策略赠予。
source: https://docs.shengkezhi.com/api/research/get-gifts-gift-code-preview
---

# 预览策略赠予

**`GET /research/gifts/{gift_code}/preview`** — 预览策略赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `gift_code` | path | string | 是 | `-` | 32 位十六进制赠予码 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `already_claimed` | boolean | 是 | 本人是否已领取过；已领取时再次领取会原样回放，不重复拷贝。 |
| `claimable` | boolean | 是 | 整体是否可领：全部策略可用、名额未尽、且不是自己发的码。 |
| `expires_at` | string | 是 | 过期时间，RFC3339。 |
| `from_user_id` | string | 是 | 赠予方 user_id。 |
| `items` | `GiftPreviewItem`[] | 是 | 逐条策略的可领取状态。 |
| `remaining_claims` | integer | 是 | 剩余可领人数。 |

### GiftPreviewItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `strategy_code` | `string` | 是 | 赠予方侧的策略编号。 |
| `description` | `string` | 是 | 策略描述，赠予方未填时为空串。 |
| `available` | `boolean` | 是 | 该条是否可领取。 |
| `reason` | `string` \| null | 否 | 不可领取的原因，可领取时为 `null`。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/gifts/GIFT-8K3M2P/preview" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "already_claimed": true,
    "claimable": true,
    "expires_at": "2026-07-01T08:00:00Z",
    "from_user_id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
    "items": [
      {
        "strategy_code": "STRAT_MOMENTUM_001",
        "description": "示例Description",
        "available": false,
        "reason": "示例Reason"
      }
    ],
    "remaining_claims": 1
  }
}```
