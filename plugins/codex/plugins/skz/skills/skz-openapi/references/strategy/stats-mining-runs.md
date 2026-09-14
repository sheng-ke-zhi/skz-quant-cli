---
title: 工作台因子挖掘统计
description: 胜可知开放平台 GET /strategy/research-stats/mining-runs：查询因子挖掘执行统计。
source: https://docs.shengkezhi.com/api/strategy/stats-mining-runs
---
# 工作台因子挖掘统计

**`GET /strategy/research-stats/mining-runs`** — 查询因子挖掘执行统计。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `{items,total}`。每项包含运行及路线信息、状态、候选和保留数量、各阶段淘汰数量、保留率、平均 Sharpe/Calmar、计算引擎、耗时和开始时间。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/mining-runs" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "items": [
    {
      "run_id": "run-001",
      "route_code": "RT_20260913_001",
      "route_name": "动量路线",
      "status": "succeeded",
      "total_candidates": 120,
      "retained": 18,
      "retain_rate": 0.15
    }
  ],
  "total": 1
}
```
