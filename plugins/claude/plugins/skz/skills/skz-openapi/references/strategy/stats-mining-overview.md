---
title: 工作台挖掘运行概览
description: 胜可知开放平台 GET /strategy/research-stats/mining-runs/{runId}/overview：查询单次挖掘的 KPI、筛选漏斗和淘汰原因。
source: https://docs.shengkezhi.com/api/strategy/stats-mining-overview
---
# 工作台挖掘运行概览

**`GET /strategy/research-stats/mining-runs/{runId}/overview`** — 查询单次挖掘的 KPI、筛选漏斗和淘汰原因。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回运行 ID、路线信息、`kpi`、筛选漏斗、淘汰原因、问题分组，以及 `overview_ready`、`funnel_complete` 完整性标识。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/research-stats/mining-runs/01K5D7M8T9ABCDEFGHJKMNPQRS/overview" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "run_id": "01K5D7M8T9ABCDEFGHJKMNPQRS",
  "route": {
    "code": "RT_001",
    "name": "动量路线"
  },
  "kpi": {
    "total_candidates": 120,
    "retained": 18
  },
  "funnel": [],
  "elimination_breakdown": [],
  "overview_ready": true
}
```
