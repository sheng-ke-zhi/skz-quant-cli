---
title: 工作台因子汇总
description: 胜可知开放平台 GET /strategy/research-stats/factor-summary：查询路线数、因子数及路线分布。
source: https://docs.shengkezhi.com/api/strategy/stats-factor-summary
---
# 工作台因子汇总

**`GET /strategy/research-stats/factor-summary`** — 查询路线数、因子数及路线分布。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `total_routes`、`total_factors`、`deleted_factors` 和 `route_distribution`。路线分布包含引擎、因子数、平均 Sharpe 和领先因子。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/factor-summary" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "total_routes": 3,
  "total_factors": 120,
  "deleted_factors": 4,
  "route_distribution": [
    {
      "route_code": "RT_001",
      "route_name": "动量路线",
      "factor_count": 42
    }
  ]
}
```
