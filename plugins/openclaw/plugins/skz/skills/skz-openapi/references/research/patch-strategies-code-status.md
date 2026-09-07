---
title: 更新实盘策略状态
description: 胜可知开放平台 PATCH /research/strategies/{code}/status：更新实盘策略状态。
source: https://docs.shengkezhi.com/api/research/patch-strategies-code-status
---

# 更新实盘策略状态

**`PATCH /research/strategies/{code}/status`** — 更新实盘策略状态。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 策略编号 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `status` | string | 是 | 目标策略生命周期状态，仅接受 `实盘` 或 `暂停` 或 `废弃`（其它值返回 40901 状态非法）。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | string | 是 | 被更新状态的策略编号（形如 `TS_1D_4E70093D`，即 前缀_周期_内容哈希）。 |
| `status` | string | 是 | 更新后的策略生命周期状态，取值 `实盘` 或 `暂停` 或 `废弃`。 |

## 调用示例

```bash
curl -X PATCH "https://api.shengkezhi.com/open/v1/research/strategies/STRAT_MOMENTUM_001/status" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"status":"示例Status"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "STS_BJ60MIN_LEADERS",
    "status": "示例Status"
  }
}```
