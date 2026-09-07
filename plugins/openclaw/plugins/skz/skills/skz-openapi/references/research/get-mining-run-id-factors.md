---
title: 挖掘结果因子列表
endpoint: GET /research/mining/{run_id}/factors
source: https://docs.shengkezhi.com/api/research/get-mining-run-id-factors
---

# 挖掘结果因子列表

`GET /research/mining/{run_id}/factors` — 挖掘结果因子列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/mining/{run_id}/factors`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| run_id | path | string | 是 | - | 挖掘执行 ID |
| q | query | string | 否 | - | 关键词，匹配因子名称、描述或代码。 |
| group | query | string | 否 | - | 研究问题分组前缀。 |
| pos_min | query | number | 否 | - | 正收益比例下限。 |
| sort | query | string | 否 | - | 排序字段，默认 best_sharpe。 |
| order | query | string | 否 | - | 排序方向，asc 或 desc，默认 desc。 |
| page | query | integer | 否 | - | 页码，从 1 开始。 |
| page_size | query | integer | 否 | - | 每页条数，默认 20。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | MiningFactorItem[] | 是 | 当前页的保留因子列表项。 |
| page | integer | 是 | 当前页码，从 1 起。 |
| page_size | integer | 是 | 每页条数。 |
| total | integer | 是 | 过滤后的因子总数（分页前）。 |

### MiningFactorItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| factor_name | string | 是 | 因子名，格式 {引擎}_{YYMMDD}_{HASH6}，如 TSA_260531_6BEFDD，系统自动规范化生成。 |
| factor_code | string | 是 | 因子 AST 表达式源码（含换行）。 |
| description | string | 是 | 因子描述（含研究逻辑与失效提示）。 |
| compute_engine | string | 是 | 计算引擎短码：TSA（时序）或 CSA（截面）。 |
| create_time | string | 是 | 因子创建时间。 |
| problem_count | integer | 是 | 该因子评估覆盖的研究问题数。 |
| eval_count | integer | 是 | 该因子的评估记录条数。 |
| metrics | object | 是 | 指标 map：指标中文名 → 数值。为跨研究问题的均值（方向多空、段训练集）。 |
| agg | object | 是 | 汇总：best_sharpe / best_problem / mean_sharpe / median_sharpe / pos_sharpe_ratio / problem_count。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/mining/b8df7d3a4d0144f1bc8bf4f9d9dee365/factors?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "factor_name": "TSA_260820_S4SE842A",
        "factor_code": "bop = TSBop($open, $high, $low, $close)\nema_bop = EMA(bop, 5)\nslope = Slope(Mean($close, 20), 10)\nMul(Sign(ema_bop), Sign(slope))",
        "description": "BOP方向乘均线方向，实体方向同步起步；窄幅震荡失效",
        "compute_engine": "TSA",
        "create_time": "2026-08-20 17:07:50.006893",
        "problem_count": 16,
        "eval_count": 64,
        "metrics": {
          "卡玛比率": 0.2789,
          "夏普比率": 0.34
        },
        "agg": {
          "best_problem": "ETS_PROBLEM_D_5DD48A9E",
          "best_sharpe": 1.5835,
          "mean_sharpe": 0.34,
          "median_calmar": 0.1656,
          "median_sharpe": 0.3388,
          "pos_sharpe_ratio": 0.765625,
          "problem_count": 16
        }
      }
    ],
    "total": 53,
    "page": 1,
    "page_size": 1
  }
}
```
