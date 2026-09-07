---
title: 实盘策略交易列表
endpoint: GET /research/strategies/{code}/live/trades
source: https://docs.shengkezhi.com/api/research/get-strategies-code-live-trades
---

# 实盘策略交易列表

`GET /research/strategies/{code}/live/trades` — 实盘策略交易列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/live/trades`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |
| year | query | string | 否 | - | 平仓年份；不传表示全部年份。 |
| kind | query | string | 否 | - | 盈亏方向：all、win 或 loss，默认 all。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | LiveTradeItem[] | 是 | — |

### LiveTradeItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| kline_key | string | 是 | — |
| symbol | string | 是 | — |
| 交易方向 | string | 是 | — |
| 盈亏 | number | 是 | — |
| 开仓时间 | string | 是 | — |
| 平仓时间 | string | 是 | — |
| 开仓价格 | number \| null | 否 | 开仓/平仓价格。仅 source="rebuilt" 时有值；直读落盘时为 null——落盘的 key_trades 不含价格列（那两列只在 wbt Rust 侧 key_trades_df 里）。用 null 而非 0.0，避免前端把 0 当成真实成交价渲染。 |
| 平仓价格 | number \| null | 否 | — |
| 持仓K线数 | integer | 是 | — |
| 持仓数量 | integer | 是 | — |
| year | string | 是 | — |
| kind | string | 是 | — |
| inherited | boolean | 是 | — |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/live/trades?page=1&page_size=2" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "kline_key": "000858.SZ|2025-02-11 11:30:00|2025-02-13 10:30:00",
        "symbol": "000858.SZ",
        "交易方向": "空头",
        "盈亏": -0.04165,
        "开仓时间": "2025-02-11 11:30:00",
        "平仓时间": "2025-02-13 10:30:00",
        "开仓价格": null,
        "平仓价格": null,
        "持仓K线数": 8,
        "持仓数量": 3,
        "year": "2025",
        "kind": "loss",
        "inherited": false
      },
      {
        "kline_key": "000858.SZ|2025-03-12 15:00:00|2025-03-14 11:30:00",
        "symbol": "000858.SZ",
        "交易方向": "空头",
        "盈亏": -0.053922000000000005,
        "开仓时间": "2025-03-12 15:00:00",
        "平仓时间": "2025-03-14 11:30:00",
        "开仓价格": null,
        "平仓价格": null,
        "持仓K线数": 7,
        "持仓数量": 6,
        "year": "2025",
        "kind": "loss",
        "inherited": false
      },
      {
        "kline_key": "000858.SZ|2025-06-12 10:30:00|2025-06-16 10:30:00",
        "symbol": "000858.SZ",
        "交易方向": "多头",
        "盈亏": -0.038877999999999996,
        "开仓时间": "2025-06-12 10:30:00",
        "平仓时间": "2025-06-16 10:30:00",
        "开仓价格": null,
        "平仓价格": null,
        "持仓K线数": 9,
        "持仓数量": 6,
        "year": "2025",
        "kind": "loss",
        "inherited": false
      }
    ]
  }
}
```
