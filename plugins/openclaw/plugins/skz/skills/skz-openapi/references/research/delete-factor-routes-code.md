---
title: 删除因子路线
endpoint: DELETE /research/factor-routes/{code}
source: https://docs.shengkezhi.com/api/research/delete-factor-routes-code
---

# 删除因子路线

`DELETE /research/factor-routes/{code}` — 删除因子路线。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/factor-routes/{code}`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 研究路线编码 |
| force | query | boolean | 否 | - | 越过两条软护栏：「名下仍有因子」与「挖掘执行目录最近仍有写入」。 |
| dry_run | query | boolean | 否 | - | 只预演不写：返回将删除的挖掘执行数与将变孤儿的因子数，不做任何修改。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| deleted | boolean | 是 | 是否已实际删除；dry_run=true 时恒为 false。路线为物理删除，不做软删。 |
| dry_run | boolean | 是 | 本次是否为预演。 |
| failed_mining_runs | string[] | 是 | 未能删除的挖掘执行编号。非空表示部分失败，重试同一请求可续删（删除幂等）。 |
| mining_runs | integer | 是 | 挖掘执行数：预演时是「将删除」，实删时是「已成功删除」。 |
| orphaned_factors | integer | 是 | 该路线名下的因子数（含已软删因子）。这些因子不被级联删除，只是此后路线名回落显示为 route_code——因子是沉淀进主库的成果，挖掘执行只是过程记录。 |
| route_code | string | 是 | 被删除的研究路线编码，回显路径参数。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/factor-routes/STRAT_MOMENTUM_001" \
  -H "Authorization: Bearer sk_xxx"
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted": true,
    "dry_run": false,
    "failed_mining_runs": [],
    "mining_runs": 1,
    "orphaned_factors": 1,
    "route_code": "STS_BJ60MIN_LEADERS"
  }
}
```
