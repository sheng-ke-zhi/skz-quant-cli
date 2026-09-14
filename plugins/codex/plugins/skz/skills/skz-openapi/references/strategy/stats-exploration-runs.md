---
title: 工作台策略探索统计
description: 胜可知开放平台 GET /strategy/research-stats/exploration-runs：查询策略探索执行统计。
source: https://docs.shengkezhi.com/api/strategy/stats-exploration-runs
---
# 工作台策略探索统计

**`GET /strategy/research-stats/exploration-runs`** — 查询策略探索执行统计。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `{items,total}`。每项包含运行 ID、问题、路线、频率、状态、扫描因子数、策略数、耗时和运行时间。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/exploration-runs" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "items": [
    {
      "id": "run-001",
      "problem_code": "PRB_001",
      "route": "RT_001",
      "status": "succeeded",
      "strategy_count": 5
    }
  ],
  "total": 1
}
```
