---
title: 批量查询缓存的最新仓位
endpoint: GET /research/strategies/positions/latest/cached
source: https://docs.shengkezhi.com/api/research/get-strategies-positions-latest-cached
---

# 批量查询缓存的最新仓位

`GET /research/strategies/positions/latest/cached` — 批量查询缓存的最新仓位。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/positions/latest/cached`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| weight_type | query | string | 是 | - | 仓位类型：ts 为时序策略，cs 为截面策略。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | LatestWeightRow[] | 是 | 所选最新权重视图的全部行，按策略和标的排序。 |

### LatestWeightRow 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dt | string | 是 | — |
| symbol | string | 是 | — |
| weight | number | 是 | — |
| strategy | string | 是 | — |
| update_time | string \| null | 否 | — |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/positions/latest/cached?weight_type=ts" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "dt": "2026-08-21 16:00:00",
        "symbol": "159329.SZ",
        "weight": 0.4167,
        "strategy": "ETS_1D_0H9J3W47",
        "update_time": "2026-08-21 20:09:54.229846"
      },
      {
        "dt": "2026-08-21 16:00:00",
        "symbol": "159399.SZ",
        "weight": -0.0833,
        "strategy": "ETS_1D_0H9J3W47",
        "update_time": "2026-08-21 20:09:54.229846"
      },
      {
        "dt": "2026-08-21 16:00:00",
        "symbol": "159740.SZ",
        "weight": 1.0,
        "strategy": "ETS_1D_0H9J3W47",
        "update_time": "2026-08-21 20:09:54.229846"
      }
    ]
  }
}
```
