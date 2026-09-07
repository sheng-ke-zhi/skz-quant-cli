---
title: 当前用户信息
endpoint: GET /research/whoami
source: https://docs.shengkezhi.com/api/research/get-whoami
---

# 当前用户信息

`GET /research/whoami` — 当前用户信息。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/whoami`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| user_id | string | 是 | 当前用户 id，来源于网关注入的 X-Skz-User-Id，是后端唯一可信的身份上下文。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/whoami" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "user_id": "01EXAMPLE000000000000000000"
  }
}
```
