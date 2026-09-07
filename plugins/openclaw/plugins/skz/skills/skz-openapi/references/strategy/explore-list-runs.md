---
title: 策略探索记录列表
endpoint: GET /strategy/explore/runs
source: https://docs.shengkezhi.com/api/strategy/explore-list-runs
---

# 策略探索记录列表

`GET /strategy/explore/runs` — 分页列出当前账户下的策略探索任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

Query 参数：

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| status | 否 | 状态过滤，支持精确状态或 active，active 表示运行中与排队中 |
| page / size | 否 | 分页，默认 1 / 20，size 最大 100 |

## 响应

```json
{
  page, size, total,
  items: [{
    fcRunId, problemCode, routeCode, status, statusText, done, ok,
    errorCode, errorMessage, resultPath, createdAt, finishedAt
  }]
}
```

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/strategy/explore/runs?page=1&size=2" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "page": 1,
  "size": 2,
  "total": 3,
  "items": [
    {
      "fcRunId": "fc-run-20260724-explore001",
      "problemCode": "PRB_20260724_001",
      "routeCode": "RT_20260724_001",
      "status": "running",
      "statusText": "执行中",
      "done": false,
      "ok": false,
      "errorCode": null,
      "errorMessage": null,
      "resultPath": null,
      "createdAt": "2026-07-24T09:10:00+08:00",
      "finishedAt": null
    }
  ]
}
```
