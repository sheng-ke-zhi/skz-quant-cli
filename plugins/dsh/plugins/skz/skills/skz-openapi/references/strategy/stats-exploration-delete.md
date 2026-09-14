---
title: 删除策略探索统计记录
description: 胜可知开放平台 DELETE /strategy/research-stats/exploration-runs/{runId}：删除当前账户的一条策略探索统计记录。
source: https://docs.shengkezhi.com/api/strategy/stats-exploration-delete
---
# 删除策略探索统计记录

**`DELETE /strategy/research-stats/exploration-runs/{runId}`** — 删除当前账户的一条策略探索统计记录。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

成功返回 HTTP `204 No Content`。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/strategy/research-stats/exploration-runs/01K5D7M8T9ABCDEFGHJKMNPQRS" \
  -H "Authorization: Bearer sk_xxx"
```

```json
null
```
