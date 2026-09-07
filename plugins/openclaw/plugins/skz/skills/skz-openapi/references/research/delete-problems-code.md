---
title: 删除研究问题
endpoint: DELETE /research/problems/{code}
source: https://docs.shengkezhi.com/api/research/delete-problems-code
---

# 删除研究问题

`DELETE /research/problems/{code}` — 删除研究问题。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/problems/{code}`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 研究问题编码 |

## 响应 data

删除成功时 data 恒为 null，以 code 是否为 0 判断结果。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/problems/STRAT_MOMENTUM_001" \
  -H "Authorization: Bearer sk_xxx"
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": null
}
```
