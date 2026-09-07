---
title: 删除策略研究产出
description: 胜可知开放平台 DELETE /research/experiments/{id}/strategies/{code}：删除策略研究产出。
source: https://docs.shengkezhi.com/api/research/delete-experiments-id-strategies-code
---

# 删除策略研究产出

**`DELETE /research/experiments/{id}/strategies/{code}`** — 删除策略研究产出。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |
| `code` | path | string | 是 | `-` | 候选策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `deleted` | boolean | 是 | 删除是否完成；成功响应恒为 true。 |
| `experiment_id` | string | 是 | 被删除候选所属的策略探索编号。 |
| `strategy_code` | string | 是 | 被删除候选的策略编号。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/experiments/experiment_20260701_001/strategies/STRAT_MOMENTUM_001" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted": true,
    "experiment_id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
    "strategy_code": "STRAT_MOMENTUM_001"
  }
}```
