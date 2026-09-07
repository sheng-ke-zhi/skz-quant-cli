---
title: 实盘策略持仓
endpoint: GET /research/strategies/{code}/positions
source: https://docs.shengkezhi.com/api/research/get-strategies-code-positions
---

# 实盘策略持仓

`GET /research/strategies/{code}/positions` — 实盘策略持仓。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/positions`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | PositionItem[] | 是 | 最近至多十个持仓快照的权重明细（按日期倒序、标的排序）。 |

### PositionItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dt | string | 是 | 该条持仓权重的数据日期。 |
| symbol | string | 是 | 标的代码（如 600519.SH）。 |
| weight | number | 是 | 该标的在对应持仓快照中的权重。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/positions" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      { "dt": "2026-08-21 15:00:00", "symbol": "000858.SZ", "weight": -0.7877 },
      { "dt": "2026-08-21 15:00:00", "symbol": "600519.SH", "weight": -0.8361 },
      { "dt": "2026-08-21 14:00:00", "symbol": "000858.SZ", "weight": -0.6546 }
    ]
  }
}
```
