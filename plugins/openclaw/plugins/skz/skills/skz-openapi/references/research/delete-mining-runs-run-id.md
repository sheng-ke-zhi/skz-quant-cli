---
title: 删除单次挖掘执行
endpoint: DELETE /research/mining/runs/{run_id}
source: https://docs.shengkezhi.com/api/research/delete-mining-runs-run-id
---

# 删除单次挖掘执行

`DELETE /research/mining/runs/{run_id}` — 删除单次挖掘执行。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/mining/runs/{run_id}`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| run_id | path | string | 是 | - | 挖掘执行 ID |
| force | query | boolean | 否 | - | 越过“目录最近仍有写入”的软护栏。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| deleted | boolean | 是 | 删除是否完成；成功响应恒为 true。 |
| run_id | string | 是 | 被删除的因子挖掘执行 ID。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/mining/runs/run_20260701_001" \
  -H "Authorization: Bearer sk_xxx"
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted": true,
    "run_id": "a79dfc93b7e64a6cbbe26f2a787a6bad"
  }
}
```
