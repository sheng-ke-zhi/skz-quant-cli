---
title: 工作台因子路线库存
description: 胜可知开放平台 GET /strategy/research-stats/factor-routes：查询工作台因子路线、因子数量和运行汇总。
source: https://docs.shengkezhi.com/api/strategy/stats-factor-routes
---
# 工作台因子路线库存

**`GET /strategy/research-stats/factor-routes`** — 查询工作台因子路线、因子数量和运行汇总。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `{items,total}`。每项包含路线定义、标签、失败场景、因子库存、质量指标、运行汇总和最近运行信息。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/factor-routes" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "items": [
    {
      "code": "RT_001",
      "name": "动量路线",
      "alive_factor_count": 42,
      "total_factor_count": 45,
      "mining_run_count": 3
    }
  ],
  "total": 1
}
```
