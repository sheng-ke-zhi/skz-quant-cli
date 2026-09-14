---
title: 刷新单个实盘策略
description: 胜可知开放平台 POST /strategy/realtime/strategies/{code}/refresh：刷新一个指定策略；用于兼容旧客户端。
source: https://docs.shengkezhi.com/api/strategy/realtime-single-refresh
---
# 刷新单个实盘策略

**`POST /strategy/realtime/strategies/{code}/refresh`** — 刷新一个指定策略；用于兼容旧客户端。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

HTTP 202 时返回 `{code:0,msg,data}`；`data` 包含运行 ID、状态、策略数量、开始及结束时间和进度消息。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/realtime/strategies/STS_001/refresh" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "更新已开始",
  "data": {
    "runId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
    "status": "running",
    "strategyCount": 1,
    "startedAt": "2026-09-13T10:00:00+08:00",
    "finishedAt": null,
    "message": "正在更新 1 个策略"
  }
}
```
