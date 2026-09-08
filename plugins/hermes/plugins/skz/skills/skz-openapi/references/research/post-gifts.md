---
title: 创建策略赠予
description: 胜可知开放平台 POST /research/gifts：创建策略赠予。
source: https://docs.shengkezhi.com/api/research/post-gifts
---

# 创建策略赠予

**`POST /research/gifts`** — 创建策略赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `max_claims` | integer | 是 | 允许领取的**去重人数**上限，1～100。同一用户重复领取幂等回放，不重复占名额。 |
| `strategy_codes` | string[] | 是 | 要赠予的实盘库策略编号，1～10 条；重复项会去重后校验。 |
| `ttl_days` | integer | 否 | 有效期天数，仅接受 1 / 3 / 7；缺省 3。到期后码及其计数器一起消失。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `claimed` | integer | 是 | 已领取人数。 |
| `created_at` | string | 是 | 创建时间，RFC3339。 |
| `expires_at` | string | 是 | 过期时间，RFC3339。 |
| `gift_code` | string | 是 | 赠予码，32 位十六进制。**它本身就是策略的访问凭证**，泄露即等于把策略给出去。 |
| `max_claims` | integer | 是 | 允许领取的去重人数上限。 |
| `strategy_codes` | string[] | 是 | 码内打包的策略编号（赠予方侧编号）。 |
| `ttl_days` | integer | 是 | 有效期天数。 |
| `unavailable_strategy_codes` | string[] | 是 | 码内已失效（赠予方已删除或已废弃）的策略编号；非空时该码整体不可领取。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"max_claims":10,"strategy_codes":["TS_1D_A96ACBB3"],"ttl_days":3}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "claimed": 1,
    "created_at": "2026-07-01T08:00:00Z",
    "expires_at": "2026-07-01T08:00:00Z",
    "gift_code": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
    "max_claims": 1,
    "strategy_codes": [
      "示例Strategy codes"
    ],
    "ttl_days": 1,
    "unavailable_strategy_codes": [
      "示例Unavailable strategy codes"
    ]
  }
}```
