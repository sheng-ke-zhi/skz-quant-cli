---
title: 策略赠予列表
endpoint: GET /research/gifts
source: https://docs.shengkezhi.com/api/research/get-gifts
---

# 策略赠予列表

`GET /research/gifts` — 策略赠予列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/gifts`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | GiftView[] | 是 | 本人发出、尚未过期的赠予码，按创建时间倒序。 |

### GiftView 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| claimed | integer | 是 | 已领取人数。 |
| created_at | string | 是 | 创建时间，RFC3339。 |
| expires_at | string | 是 | 过期时间，RFC3339。 |
| gift_code | string | 是 | 赠予码，32 位十六进制。它本身就是策略的访问凭证，泄露即等于把策略给出去。 |
| max_claims | integer | 是 | 允许领取的去重人数上限。 |
| strategy_codes | string[] | 是 | 码内打包的策略编号（赠予方侧编号）。 |
| ttl_days | integer | 是 | 有效期天数。 |
| unavailable_strategy_codes | string[] | 是 | 码内已失效（赠予方已删除或已废弃）的策略编号；非空时该码整体不可领取。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/gifts" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": []
  }
}
```
