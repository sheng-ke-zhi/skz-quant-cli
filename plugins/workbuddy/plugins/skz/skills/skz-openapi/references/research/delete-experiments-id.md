---
title: 删除策略研究执行
description: 胜可知开放平台 DELETE /research/experiments/{id}：删除策略研究执行。
source: https://docs.shengkezhi.com/api/research/delete-experiments-id
---

# 删除策略研究执行

**`DELETE /research/experiments/{id}`** — 删除策略研究执行。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |
| `force` | query | boolean | 否 | `-` | 越过「目录最近仍有写入」的软护栏。该护栏是启发式判断（后端不触发探索任务，无从确知 是否真有任务在跑），必须留逃生舱，否则会造出「明知是垃圾却删不掉」的死角。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `deleted` | boolean | 是 | 删除是否完成；成功响应恒为 true。 |
| `run_id` | string | 是 | 被删除的因子挖掘执行 ID。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/experiments/experiment_20260701_001" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted": true,
    "run_id": "a79dfc93b7e64a6cbbe26f2a787a6bad"
  }
}```
