---
title: 撤销策略赠予
description: 胜可知开放平台 DELETE /research/gifts/{gift_code}：撤销策略赠予。
source: https://docs.shengkezhi.com/api/research/delete-gifts-gift-code
---

# 撤销策略赠予

**`DELETE /research/gifts/{gift_code}`** — 撤销策略赠予。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `gift_code` | path | string | 是 | `-` | 32 位十六进制赠予码 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `revoked` | boolean | 是 | 固定为 `true`，表示当前用户发出的赠予码已经撤回，后续不能再被新用户领取。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/gifts/GIFT-8K3M2P" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "revoked": true
  }
}```
