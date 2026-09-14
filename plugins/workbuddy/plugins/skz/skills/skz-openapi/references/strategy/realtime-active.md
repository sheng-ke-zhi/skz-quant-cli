---
title: 查询当前实盘刷新任务
description: 胜可知开放平台 GET /strategy/realtime/strategies/refresh/active：查询当前账户最近一次批量刷新状态。
source: https://docs.shengkezhi.com/api/strategy/realtime-active
---
# 查询当前实盘刷新任务

**`GET /strategy/realtime/strategies/refresh/active`** — 查询当前账户最近一次批量刷新状态。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `{code:0,msg:"ok",data}`。没有近期任务时 `data` 为 `null`；否则字段与批量刷新响应一致。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/realtime/strategies/refresh/active" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
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
