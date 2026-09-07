---
title: 批量查询挖掘进度
endpoint: POST /strategy/miner/poll
source: https://docs.shengkezhi.com/api/strategy/miner-poll
---

# 批量查询挖掘进度

`POST /strategy/miner/poll` — 传入 fcRunId 数组，批量取回因子挖掘任务的状态与进度。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| runIds | array[string] | 是 | 运行标识数组，单次最多 100 个 |

## 响应

```json
[{
  fcRunId, routeCode, status, statusText, done, ok, percent, step, message,
  errorCode, errorMessage, resultPath, createdAt, finishedAt
}]
```

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/miner/poll" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"runIds":["fc-run-20260724-abc123"]}'
```

```json
[
  {
    "fcRunId": "fc-run-20260724-abc123",
    "routeCode": "RT_20260724_001",
    "status": "succeeded",
    "statusText": "已完成",
    "done": true,
    "ok": true,
    "percent": 100,
    "step": "质检通过",
    "message": "因子挖掘完成，构建因子 12 个",
    "errorCode": null,
    "errorMessage": null,
    "resultPath": "/results/fc-run-20260724-abc123",
    "createdAt": "2026-07-24T09:00:00+08:00",
    "finishedAt": "2026-07-24T09:05:00+08:00"
  }
]
```
