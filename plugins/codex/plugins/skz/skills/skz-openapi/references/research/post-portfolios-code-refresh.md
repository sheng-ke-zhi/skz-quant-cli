---
title: 刷新组合数据
description: 胜可知开放平台 POST /research/portfolios/{code}/refresh：刷新组合数据。
source: https://docs.shengkezhi.com/api/research/post-portfolios-code-refresh
---

# 刷新组合数据

**`POST /research/portfolios/{code}/refresh`** — 刷新组合数据。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 组合编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `portfolio_code` | string | 是 | 已受理的组合编号；可用它轮询 `GET /api/portfolios`。 |
| `status` | string | 是 | 固定 `pending`：已受理，正在后台生成。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/portfolios/STRAT_MOMENTUM_001/refresh" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "portfolio_code": "STS_BJ60MIN_LEADERS",
    "status": "示例Status"
  }
}```
