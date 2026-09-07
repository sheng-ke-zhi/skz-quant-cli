---
title: 实盘交易 K 线窗口
endpoint: GET /research/strategies/{code}/live/trades/{trade_key}/kline
source: https://docs.shengkezhi.com/api/research/get-strategies-code-live-trades-trade-key-kline
---

# 实盘交易 K 线窗口

`GET /research/strategies/{code}/live/trades/{trade_key}/kline` — 实盘交易 K 线窗口。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/live/trades/{trade_key}/kline`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |
| trade_key | path | string | 是 | - | 交易的 K 线定位键 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| bars | KlineBar[] | 是 | 出入场前后截取的 K 线窗口序列，按时间升序。 |
| entry | object | 否 | —（KlineMarker，开仓标记） |
| exit | object | 否 | —（KlineMarker，平仓标记） |
| symbol | string | 是 | 该笔交易的标的代码。 |

### KlineBar 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| time | string | 是 | 该 K 线的时间戳，来自 weights.feather 的 dt 列，ISO 本地时间。 |
| o | number | 是 | 开盘价（复权后，与回测同源）。 |
| h | number | 是 | 最高价（复权后，与回测同源）。 |
| l | number | 是 | 最低价（复权后，与回测同源）。 |
| c | number | 是 | 收盘价（复权后，与回测同源）。 |
| vol | number | 是 | 成交量。 |
| amount | number | 是 | 成交额。 |
| weight | number | 是 | 该 K 线上策略给该标的的持仓权重，来自 weights.feather 的 weight 列，正为多头、负为空头。 |

### KlineMarker 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| time | string | 是 | 标记发生的时间，开仓标记为开仓时刻、平仓标记为平仓时刻。 |
| price | number | 是 | 标记价位，取标记时刻对应 K 线的收盘价。 |
| side | string \| null | 否 | 交易方向（多头或空头）；仅开仓标记带此字段，平仓标记省略。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/live/trades/000858.SZ%7C2025-02-11%2011%3A30%3A00%7C2025-02-13%2010%3A30%3A00/kline" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "symbol": "000858.SZ",
    "bars": [
      {
        "time": "2025-01-22 15:00:00",
        "o": 2489.5025100000003,
        "h": 2492.3673000000003,
        "l": 2485.873776,
        "c": 2489.5025100000003,
        "vol": 3602201.0,
        "amount": 469296874.0,
        "weight": -0.385
      },
      {
        "time": "2025-01-23 10:30:00",
        "o": 2513.4618,
        "h": 2552.2353580000004,
        "l": 2509.5649600000006,
        "c": 2522.8142159999998,
        "vol": 9307207.0,
        "amount": 1208300448.0,
        "weight": -0.4019
      },
      {
        "time": "2025-01-23 11:30:00",
        "o": 2523.788426,
        "h": 2526.1265300000005,
        "l": 2503.135174,
        "c": 2510.1494860000003,
        "vol": 2782200.0,
        "amount": 358745372.0,
        "weight": -0.3676
      }
    ],
    "entry": {
      "time": "2025-02-11 11:30:00",
      "price": 2455.9834100000003,
      "side": "空头"
    },
    "exit": {
      "time": "2025-02-13 10:30:00",
      "price": 2558.2754600000003
    }
  }
}
```
