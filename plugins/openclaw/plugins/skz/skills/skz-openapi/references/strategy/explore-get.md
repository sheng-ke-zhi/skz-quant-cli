---
title: 查询策略探索进度
endpoint: GET /strategy/explore/{fcRunId}
source: https://docs.shengkezhi.com/api/strategy/explore-get
---

# 查询策略探索进度

`GET /strategy/explore/{fcRunId}` — 按运行标识查询单次策略探索任务的状态与进度。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 路径参数

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| fcRunId | 是 | 策略探索任务运行标识 |

## 响应

```json
{
  fcRunId, status, statusText, done, ok, percent, step, message,
  errorCode, errorMessage, resultPath, createdAt, finishedAt
}
```

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/strategy/explore/fc-run-20260724-explore001" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "fcRunId": "fc-run-20260724-explore001",
  "status": "running",
  "statusText": "执行中",
  "done": false,
  "ok": false,
  "percent": 35,
  "step": "回测中",
  "message": "正在对候选策略进行回测验证",
  "errorCode": null,
  "errorMessage": null,
  "resultPath": null,
  "createdAt": "2026-07-24T09:10:00+08:00",
  "finishedAt": null
}
```
