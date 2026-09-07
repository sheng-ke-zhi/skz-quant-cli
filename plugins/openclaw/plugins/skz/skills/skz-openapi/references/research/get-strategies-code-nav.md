---
title: 实盘策略净值
description: 胜可知开放平台 GET /research/strategies/{code}/nav：实盘策略净值。
source: https://docs.shengkezhi.com/api/research/get-strategies-code-nav
---

# 实盘策略净值

**`GET /research/strategies/{code}/nav`** — 实盘策略净值。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `dates` | string[] | 是 | 净值序列对应的交易日期，作为图表 X 轴。 |
| `drawdown` | number[] | 是 | 回撤序列，等于当前累计收益减去历史峰值（非正数）。 |
| `nav` | number[] | 是 | 累计净值主曲线，由每日收益累计得到（起点 1.0）。 |
| `oos_start` | string | 是 | 样本外起点日期，前端据此在净值图上叠加样本外/实盘阴影与起点虚线。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/nav" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "oos_start": "2025-01-17",
    "dates": [
      "2025-01-17 00:00:00",
      "2025-01-20 00:00:00",
      "2025-01-21 00:00:00"
    ],
    "nav": [
      1.0,
      1.0,
      0.9999394528991177
    ],
    "drawdown": [
      0.0,
      0.0,
      -6.0547100882352156e-05
    ]
  }
}```
