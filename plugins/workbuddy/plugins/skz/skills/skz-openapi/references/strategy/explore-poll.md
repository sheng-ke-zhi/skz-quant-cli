---
title: 批量查询探索进度
description: 胜可知开放平台 POST /strategy/explore/poll：批量查询策略探索任务的状态与进度。
source: https://docs.shengkezhi.com/api/strategy/explore-poll
---

# 批量查询探索进度

**`POST /strategy/explore/poll`** — 传入 `fcRunId` 数组，批量取回策略探索任务的状态与进度。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `runIds` | array[string] | 是 | 运行标识数组，单次最多 `100` 个 |

## 响应

`[{ fcRunId, problemCode, routeCode, status, statusText, done, ok, percent, step, message, errorCode, errorMessage, resultPath, createdAt, finishedAt }]`

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/explore/poll" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"runIds":["fc-run-20260724-explore001"]}'
```

```json
[
  {
    "fcRunId": "fc-run-20260724-explore001",
    "problemCode": "PRB_20260724_001",
    "routeCode": "RT_20260724_001",
    "status": "succeeded",
    "statusText": "已完成",
    "done": true,
    "ok": true,
    "percent": 100,
    "step": "策略筛选完成",
    "message": "策略探索完成，产出策略 3 个",
    "errorCode": null,
    "errorMessage": null,
    "resultPath": "/results/fc-run-20260724-explore001",
    "createdAt": "2026-07-24T09:10:00+08:00",
    "finishedAt": "2026-07-24T09:20:00+08:00"
  }
]
```

