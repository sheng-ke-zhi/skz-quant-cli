---
title: 删除因子
endpoint: DELETE /research/factors/{factor_name}
source: https://docs.shengkezhi.com/api/research/delete-factors-factor-name
---

# 删除因子

`DELETE /research/factors/{factor_name}` — 删除因子。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/factors/{factor_name}`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| factor_name | path | string | 是 | - | 因子名称 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| reason | string | 否 | 删除理由，记录因子逻辑审核的判断依据，落库到 delete_reason，可空（默认空串）。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| factor_name | string | 是 | 被删除的因子名，回显路径参数中的因子标识。 |
| is_deleted | boolean | 是 | 删除标记，操作成功后恒为 true（库内置 is_deleted=1，因子不做物理删除，仍可用 include_deleted 查回）。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/factors/alpha_momentum_20" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"reason": "示例Reason"}'
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "factor_name": "alpha_momentum_20",
    "is_deleted": true
  }
}
```
