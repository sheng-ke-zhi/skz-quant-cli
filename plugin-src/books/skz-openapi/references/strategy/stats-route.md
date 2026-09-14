---
title: 因子路线运行统计
description: 胜可知开放平台 GET /strategy/research-stats/route-stats：按因子路线聚合挖掘次数和产出指标。
source: https://docs.shengkezhi.com/api/strategy/stats-route
---
# 因子路线运行统计

**`GET /strategy/research-stats/route-stats`** — 按因子路线聚合挖掘次数和产出指标。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `{items,total}`。每项包含路线编码和名称、计算引擎、挖掘次数、候选及有效因子累计、平均指标和最近运行信息。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/route-stats" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "items": [
    {
      "route_code": "RT_001",
      "route_name": "动量路线",
      "mining_run_count": 3,
      "produced_factor_total": 300,
      "valid_factor_total": 42
    }
  ],
  "total": 1
}
```
