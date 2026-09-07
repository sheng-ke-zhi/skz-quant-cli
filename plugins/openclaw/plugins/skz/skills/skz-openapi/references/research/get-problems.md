---
title: 研究问题列表
description: 胜可知开放平台 GET /research/problems：研究问题列表。
source: https://docs.shengkezhi.com/api/research/get-problems
---

# 研究问题列表

**`GET /research/problems`** — 研究问题列表。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `q` | query | string | 否 | `-` | 关键字，模糊匹配 code、名称或描述。 |
| `source` | query | string | 否 | `-` | 来源过滤，`all`（默认）、`builtin` 或 `user`。 |
| `problem_type` | query | string | 否 | `-` | 按问题类型过滤，`TimeSeriesProblem` 或 `CrossSectionalProblem`。 |
| `freq` | query | string | 否 | `-` | 按 K 线周期过滤。 |
| `dataset` | query | string | 否 | `-` | 按数据源过滤，`future`、`etf` 或 `stock`。 |
| `sort` | query | string | 否 | `-` | 排序字段，如 `code`、`name`、`type_label`、`freq`、`dataset` 或 `source`。 |
| `order` | query | string | 否 | `-` | 排序方向，`asc` 或 `desc`。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | `ProblemView`[] | 是 | 研究问题列表项数组。 |
| `total` | integer | 是 | 过滤后的研究问题总数。 |

### ProblemView 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | `string` | 是 | 研究问题编号，后端按内容哈希生成，格式 `前缀_PROBLEM_HASH`。 |
| `name` | `string` | 是 | 研究问题名称。 |
| `problem_type` | `string` | 是 | 研究问题类型枚举，`TimeSeriesProblem` 或 `CrossSectionalProblem`。 |
| `type_label` | `string` | 是 | 问题类型中文标签，如 `时序择时` 或 `截面多空`。 |
| `description` | `string` | 是 | 研究问题描述。 |
| `freq` | `string` | 是 | K 线周期，如 `日线`、`60分钟` 等。 |
| `dataset` | `string` | 是 | 数据源，`future`、`etf` 或 `stock`。 |
| `symbols` | `string[]` | 是 | 合约或标的代码列表。 |
| `source` | `string` | 是 | 数据来源，`builtin`（预置）或 `user`（用户自定义）。 |
| `editable` | `boolean` | 是 | 是否可删除；workspace 中的问题均为 `true`。 |
| `time_segments` | `Seg[]` \| null | 否 | 时间段划分（训练与验证区间）；仅详情接口返回，列表项省略该字段。 |

### Seg 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `edt` | `string` | 是 | 时间段结束日期，格式 `YYYYMMDD`，不得晚于 `20250701`。 |
| `name` | `string` | 是 | 时间段名称，取自必需段之一，如 `训练集A段`、`训练集`、`后置验证`。 |
| `sdt` | `string` | 是 | 时间段起始日期，格式 `YYYYMMDD`，不得晚于 `20250701`。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/problems?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "total": 16,
    "items": [
      {
        "code": "STS_QS_LEADERS",
        "name": "券商龙头时序研究",
        "problem_type": "TimeSeriesProblem",
        "type_label": "时序择时",
        "description": "A 股券商板块龙头：中信证券，国泰海通，东方财富，中信建投，华泰证券",
        "freq": "日线",
        "dataset": "stock",
        "symbols": [
          "600030.SH",
          "601211.SH"
        ],
        "source": "user",
        "editable": true
      },
      {
        "code": "STS_BJ60MIN_LEADERS",
        "name": "白酒龙头60分钟时序研究",
        "problem_type": "TimeSeriesProblem",
        "type_label": "时序择时",
        "description": "A 股白酒板块龙头：贵州茅台，五粮液；60分钟行情",
        "freq": "60分钟",
        "dataset": "stock",
        "symbols": [
          "600519.SH",
          "000858.SZ"
        ],
        "source": "user",
        "editable": true
      },
      {
        "code": "FTS_COMMODITY_DAY",
        "name": "商品期货主力连续合约日线",
        "problem_type": "TimeSeriesProblem",
        "type_label": "时序择时",
        "description": "商品期货主力连续合约日线",
        "freq": "日线",
        "dataset": "future",
        "symbols": [
          "A999.DCE",
          "B999.DCE"
        ],
        "source": "user",
        "editable": true
      }
    ]
  }
}```
