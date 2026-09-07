---
title: 策略关键交易列表
endpoint: GET /research/experiments/{id}/strategies/{code}/trades
source: https://docs.shengkezhi.com/api/research/get-experiments-id-strategies-code-trades
---

# 策略关键交易列表

`GET /research/experiments/{id}/strategies/{code}/trades` — 策略关键交易列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/experiments/{id}/strategies/{code}/trades`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| id | path | string | 是 | - | 策略探索实验编号 |
| code | path | string | 是 | - | 候选策略编号 |
| year | query | string | 否 | - | 交易平仓年份筛选（对应 key_trades 的年份分组）；留空表示不限年份，返回全部年份的关键交易。 |
| kind | query | string | 否 | - | 盈亏方向筛选：win 只看盈利交易，loss 只看亏损交易，all 表示全部；缺省为 all。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | object[] | 是 | 关键交易卡列表：由回测 key_trades 摊平；每个元素为自由形对象，含 kline_key、标的、交易方向、单笔盈亏、开仓与平仓时间，以及盈亏标签 kind（取 win 或 loss）等键。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments/a79dfc93b7e64a6cbbe26f2a787a6bad/strategies/FTS_1D_0UCFSYXF/trades?page=1&page_size=2" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "kind": "win",
        "kline_key": "SF999.ZCE|2017-08-07T16:00:00|2017-08-23T16:00:00",
        "symbol": "SF999.ZCE",
        "year": "2017",
        "交易方向": "多头",
        "平仓时间": "2017-08-23T16:00:00",
        "开仓时间": "2017-08-07T16:00:00",
        "持仓K线数": 13,
        "持仓数量": 7,
        "盈亏": 0.239521
      },
      {
        "kind": "win",
        "kline_key": "SF999.ZCE|2017-08-25T16:00:00|2017-09-18T16:00:00",
        "symbol": "SF999.ZCE",
        "year": "2017",
        "交易方向": "空头",
        "平仓时间": "2017-09-18T16:00:00",
        "开仓时间": "2017-08-25T16:00:00",
        "持仓K线数": 17,
        "持仓数量": 7,
        "盈亏": 0.16987
      },
      {
        "kind": "win",
        "kline_key": "SF999.ZCE|2017-08-23T16:00:00|2017-09-19T16:00:00",
        "symbol": "SF999.ZCE",
        "year": "2017",
        "交易方向": "空头",
        "平仓时间": "2017-09-19T16:00:00",
        "开仓时间": "2017-08-23T16:00:00",
        "持仓K线数": 20,
        "持仓数量": 2,
        "盈亏": 0.166935
      }
    ]
  }
}
```
