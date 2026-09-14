---
title: 修改组合生命周期状态
description: 胜可知开放平台 PATCH /research/portfolios/{code}/status：修改组合生命周期状态。
source: https://docs.shengkezhi.com/api/research/patch-portfolios-code-status
---
# 修改组合生命周期状态

**`PATCH /research/portfolios/{code}/status`** — 修改组合生命周期状态。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 组合编码。 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `expected_status` | string | 是 | 用户看到的原状态；不一致时返回409，避免覆盖其它会话的操作。 |
| `status` | string | 是 | 目标状态：`实盘`、`暂停` 或 `废弃`。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | string | 是 | 被更新的组合编码。 |
| `status` | string | 是 | 更新后的组合生命周期状态。 |

## 调用示例

```bash
curl -X PATCH "https://api.shengkezhi.com/open/v1/research/portfolios/PORTFOLIO_DEMO_001/status" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"expected_status":"实盘","status":"暂停"}'
```

```json
{
  "code": 0,
  "data": {
    "code": "PORTFOLIO_DEMO_001",
    "status": "暂停"
  },
  "msg": "ok"
}```
