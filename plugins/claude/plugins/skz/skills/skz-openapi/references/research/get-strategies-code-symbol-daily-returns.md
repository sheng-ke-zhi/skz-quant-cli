---
title: 实盘策略分品种日收益
description: 胜可知开放平台 GET /research/strategies/{code}/symbol-daily-returns：实盘策略分品种日收益。
source: https://docs.shengkezhi.com/api/research/get-strategies-code-symbol-daily-returns
---
# 实盘策略分品种日收益

**`GET /research/strategies/{code}/symbol-daily-returns`** — 实盘策略分品种日收益。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 策略编号 |
| `symbol` | query | string | 否 | `-` | 精确筛选单个品种代码；不传或传空字符串时返回全部品种。 |
| `start_date` | query | string | 否 | `-` | 起始交易日（含），格式 `YYYY-MM-DD`。 |
| `end_date` | query | string | 否 | `-` | 结束交易日（含），格式 `YYYY-MM-DD`。 |
| `page` | query | integer | 否 | `-` | 页码，从 1 起，默认 1。 |
| `page_size` | query | integer | 否 | `-` | 每页条数，默认 1000，上限 5000。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `aggregation_method` | string | 是 | 品种贡献聚合方式：`mean` 对应 `ts`，`sum` 对应 `cs`。 |
| `items` | `SymbolDailyReturnItem`[] | 是 | 当前分页的按品种日收益明细，按日期、品种升序排列。 |
| `page` | integer | 是 | 当前页码，从 1 起。 |
| `page_size` | integer | 是 | 每页条数。 |
| `strategy` | string | 是 | 策略编号。 |
| `total` | integer | 是 | 满足过滤条件的总行数（分页前）。 |
| `weight_type` | string | 是 | 策略权重口径：`ts` 或 `cs`。 |

### SymbolDailyReturnItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `date` | `string` | 是 | 交易日期，格式 `YYYY-MM-DD`。 |
| `symbol` | `string` | 是 | 品种或证券代码。 |
| `raw_return` | `number` | 是 | 该品种自身的原始日收益；同一自然日有多条记录时取加总值。 |
| `contribution` | `number` | 是 | 该品种对策略当日收益的实际贡献。`ts` 按当日有效品种数均分，`cs` 等于原始日收益。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STRAT_ALPHA/symbol-daily-returns?symbol=BBB&start_date=2024-01-04&end_date=2024-01-05&page=1&page_size=1000" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "strategy": "STRAT_ALPHA",
    "weight_type": "ts",
    "aggregation_method": "mean",
    "items": [
      {
        "date": "2024-01-04",
        "symbol": "BBB",
        "raw_return": 0.05,
        "contribution": 0.016666666666666666
      },
      {
        "date": "2024-01-05",
        "symbol": "BBB",
        "raw_return": -0.02,
        "contribution": -0.006666666666666667
      }
    ],
    "total": 2,
    "page": 1,
    "page_size": 1000
  }
}```
