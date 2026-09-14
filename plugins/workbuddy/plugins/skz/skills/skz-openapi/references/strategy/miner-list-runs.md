---
title: 因子挖掘任务列表
description: 胜可知开放平台 GET /strategy/miner/runs：分页列出当前账户下的因子挖掘任务及其执行状态。
source: https://docs.shengkezhi.com/api/strategy/miner-list-runs
---

# 因子挖掘任务列表

**`GET /strategy/miner/runs`** — 分页列出当前账户下的因子挖掘任务及其执行状态。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| Query 参数 | 必填 | 说明 |
| --- | --- | --- |
| `status` | 否 | 状态过滤，支持精确状态或 `active`，`active` 表示运行中与排队中 |
| `page` / `size` | 否 | 分页，默认 `1` / `20`，`size` 最大 `100` |

## 响应

`{ page, size, total, items: [{ fcRunId, routeCode, status, statusText, done, ok, errorCode, errorMessage, resultPath, createdAt, finishedAt }] }`

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/strategy/miner/runs?page=1&size=2" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "page": 1,
  "size": 2,
  "total": 5,
  "items": [
    {
      "fcRunId": "fc-run-20260724-abc123",
      "routeCode": "RT_20260724_001",
      "status": "running",
      "statusText": "执行中",
      "done": false,
      "ok": false,
      "errorCode": null,
      "errorMessage": null,
      "resultPath": null,
      "createdAt": "2026-07-24T09:00:00+08:00",
      "finishedAt": null
    }
  ]
}
```

## 与「因子挖掘产出记录」的区别

本接口来自任务调度侧，返回的是**任务本身的执行状态**（排队、运行中、失败原因、结果路径），支持按状态过滤与分页，适合做任务列表页与进度轮询。

若要看每次挖掘**产出了多少因子**（候选数、保留数、保留率、耗时），请用投研接口的[因子挖掘产出记录](../research/get-mining-runs.md)。
