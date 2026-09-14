---
title: 批量刷新实盘策略
description: 胜可知开放平台 POST /strategy/realtime/strategies/refresh：批量刷新最多允许数量的实盘策略数据。
source: https://docs.shengkezhi.com/api/strategy/realtime-refresh
---
# 批量刷新实盘策略

**`POST /strategy/realtime/strategies/refresh`** — 批量刷新最多允许数量的实盘策略数据。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

HTTP 202 时返回 `{code:0,msg,data}`；`data` 包含 `runId`、`status`、`strategyCount`、`startedAt`、`finishedAt` 和 `message`。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/realtime/strategies/refresh" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"strategies":["STS_001","STS_002"]}'
```

```json
{
  "code": 0,
  "msg": "更新已开始",
  "data": {
    "runId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
    "status": "running",
    "strategyCount": 2,
    "startedAt": "2026-09-13T10:00:00+08:00",
    "finishedAt": null,
    "message": "正在更新 2 个策略"
  }
}
```
