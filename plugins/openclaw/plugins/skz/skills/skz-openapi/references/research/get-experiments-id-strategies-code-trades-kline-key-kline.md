---
title: 交易 K 线窗口
endpoint: GET /research/experiments/{id}/strategies/{code}/trades/{kline_key}/kline
source: https://docs.shengkezhi.com/api/research/get-experiments-id-strategies-code-trades-kline-key-kline
---

# 交易 K 线窗口

`GET /research/experiments/{id}/strategies/{code}/trades/{kline_key}/kline` — 交易 K 线窗口。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/experiments/{id}/strategies/{code}/trades/{kline_key}/kline`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| id | path | string | 是 | - | 策略探索实验编号 |
| code | path | string | 是 | - | 候选策略编号 |
| kline_key | path | string | 是 | - | 关键交易列表返回的 K 线定位键 |

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
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments/a79dfc93b7e64a6cbbe26f2a787a6bad/strategies/FTS_1D_0UCFSYXF/trades/SF999.ZCE%7C2017-08-07T16%3A00%3A00%7C2017-08-23T16%3A00%3A00/kline" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "symbol": "SF999.ZCE",
    "bars": [
      {
        "time": "2017-06-26T16:00:00",
        "o": 4918.615238196536,
        "h": 4933.281254560894,
        "l": 4911.282230014357,
        "c": 4924.11499433317,
        "vol": 42.0,
        "amount": 1128500.0,
        "weight": -0.3333
      },
      {
        "time": "2017-06-27T16:00:00",
        "o": 4927.78149842426,
        "h": 4949.7805229707965,
        "l": 4909.448977968812,
        "c": 4935.114506606438,
        "vol": 142.0,
        "amount": 3818200.0,
        "weight": -0.2143
      },
      {
        "time": "2017-06-28T16:00:00",
        "o": 4964.446539335155,
        "h": 4966.279791380699,
        "l": 4900.282717741088,
        "c": 4903.9492218321775,
        "vol": 78.0,
        "amount": 2095000.0,
        "weight": 0.25
      }
    ],
    "entry": {
      "time": "2017-08-07T16:00:00",
      "price": 5510.755648907487,
      "side": "多头"
    },
    "exit": {
      "time": "2017-08-23T16:00:00",
      "price": 6830.697121699699
    }
  }
}
```
