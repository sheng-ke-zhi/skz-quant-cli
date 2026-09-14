---
title: 批量删除因子
description: 胜可知开放平台 DELETE /research/factors：批量删除因子。
source: https://docs.shengkezhi.com/api/research/delete-factors
---
# 批量删除因子

**`DELETE /research/factors`** — 批量删除因子。重复名称只处理一次；部分项目失败时仍返回 HTTP 200，应逐项检查 `items`。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `factor_names` | string[] | 是 | 要删除的因子名称，1～1000 个；重复名称只处理第一次。 |
| `reason` | string | 否 | 本批次共用的删除原因，省略时为空字符串。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `failed_count` | integer | 是 | — |
| `items` | `FactorDeleteResult`[] | 是 | — |
| `succeeded_count` | integer | 是 | — |

### FactorDeleteResult 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | `integer` | 是 | 0 on success; otherwise the existing API error code. |
| `factor_name` | `string` | 是 | — |
| `msg` | `string` | 是 | — |
| `success` | `boolean` | 是 | — |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/factors" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"factor_names":["alpha_momentum_20"],"reason":"逻辑审核未通过"}'
```

```json
{
  "code": 0,
  "data": {
    "failed_count": 0,
    "items": [
      {
        "code": 0,
        "factor_name": "alpha_momentum_20",
        "msg": "ok",
        "success": true
      }
    ],
    "succeeded_count": 1
  },
  "msg": "ok"
}```
