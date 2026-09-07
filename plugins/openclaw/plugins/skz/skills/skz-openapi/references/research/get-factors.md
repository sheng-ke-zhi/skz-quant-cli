---
title: 因子列表
endpoint: GET /research/factors
source: https://docs.shengkezhi.com/api/research/get-factors
---

# 因子列表

`GET /research/factors` — 因子列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/factors`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| q | query | string | 否 | - | 关键词搜索，匹配因子名、描述或 AST 源码。 |
| route | query | string | 否 | - | 按所属路线编码过滤。 |
| engine | query | string | 否 | - | 按计算引擎简称过滤（TSA 或 CSA）。 |
| tag | query | string | 否 | - | 按质检标签过滤，如 detect_passed、positive_passed。 |
| sort | query | string | 否 | - | 排序字段，前缀 `-` 表示降序（如 `-create_time`）；指标名走数值排序，其余走字符串列。 |
| order | query | string | 否 | - | 排序方向，asc 或 desc，默认 desc。 |
| include_deleted | query | boolean | 否 | - | true 时列表不强制 is_deleted=0（逻辑审核抽屉需看到已删除因子）。 |
| page | query | integer | 否 | - | 页码，从 1 开始，默认 1。 |
| page_size | query | integer | 否 | - | 每页条数，默认 20，上限 100。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | FactorListItem[] | 是 | 当前分页的因子列表项。 |
| page | integer | 是 | 当前页码，从 1 开始。 |
| page_size | integer | 是 | 每页条数。 |
| sampled | integer | 是 | 本页实际返回的条数。 |
| total | integer | 是 | 命中总数（用于分页）。 |

### FactorListItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| agg | object | 是 | 汇总：best_sharpe / best_problem / mean_sharpe / median_sharpe / pos_sharpe_ratio / problem_count。 |
| compute_engine | string | 是 | 计算引擎简称：TSA 时序或 CSA 截面。 |
| create_time | string | 是 | 因子创建时间。 |
| creator | string | 是 | 因子创建者：生成该因子的 agent 或模型标识。 |
| delete_reason | string \| null | 否 | 删除原因；未删除或老库缺该列时为 null。 |
| description | string | 是 | 因子描述：逻辑说明与已知局限。 |
| engine_full | string | 是 | 计算引擎全称，如 TimeSeriesAstEngine。 |
| factor_code | string | 是 | 因子 AST 源码。 |
| factor_name | string | 是 | 因子名，格式 引擎_YYMMDD_HASH6，系统按内容哈希自动生成，不可手命名。 |
| is_deleted | boolean | 是 | 是否已删除。 |
| metrics | object | 是 | 指标 map：指标中文名 → 数值。 |
| route | string | 是 | 所属路线编码。 |
| route_name | string | 是 | 所属路线名称，缺失时回落为路线编码。 |
| tags | string[] | 是 | 质检标签列表，如 detect_passed、positive_passed。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/factors?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "agg": {
          "best_problem": "FTS_PROBLEM_D_60144637",
          "best_sharpe": 1.2202,
          "mean_sharpe": 0.3352,
          "median_calmar": 0.171,
          "median_sharpe": 0.3605,
          "pos_sharpe_ratio": 0.6875,
          "problem_count": 16
        },
        "compute_engine": "TSA",
        "create_time": "2026-08-20 17:10:07.083924",
        "creator": "SKZ_ExampleModel",
        "delete_reason": null,
        "description": "均线5期变化率60日排名；极低波动排名极端集中失效",
        "engine_full": "TimeSeriesAstEngine",
        "factor_code": "ma10 = Mean($close, 10)\nsl = Sub(ma10, Ref(ma10, 1))\nbkt = TSPctChange(sl, 5)\nRank(bkt, 60)",
        "factor_name": "TSA_260820_N26M85RL",
        "is_deleted": false,
        "metrics": {
          "卡玛比率": 0.2582,
          "夏普比率": 0.3352
        },
        "route": "fde0cb170b25",
        "route_name": "均线由抖转顺：捕捉趋势缓慢起步",
        "tags": ["detect_passed", "positive_passed"]
      }
    ],
    "page": 1,
    "page_size": 1,
    "sampled": 1,
    "total": 186833
  }
}
```
