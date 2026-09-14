---
title: 读取组合数据更新任务状态
description: 胜可知开放平台 GET /research/portfolios/{code}/refresh-status：读取组合数据更新任务状态。
source: https://docs.shengkezhi.com/api/research/get-portfolios-code-refresh-status
---
# 读取组合数据更新任务状态

**`GET /research/portfolios/{code}/refresh-status`** — 读取组合数据更新任务状态。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 组合编码。 |

## 响应 data

没有刷新任务时 `data` 为 `null`；存在任务时包含 `status`、`submitted_at`、`updated_at` 和可空的 `error`。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/portfolios/PORTFOLIO_DEMO_001/refresh-status" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "data": {
    "status": "running",
    "submitted_at": "2026-09-13T10:00:00Z",
    "updated_at": "2026-09-13T10:01:00Z",
    "error": null
  },
  "msg": "ok"
}```
