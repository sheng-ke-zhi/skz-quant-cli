---
title: 因子库汇总
endpoint: GET /research/factors/summary
source: https://docs.shengkezhi.com/api/research/get-factors-summary
---

# 因子库汇总

`GET /research/factors/summary` — 因子库汇总。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/factors/summary`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| deleted_factors | integer | 是 | 已删除因子数。 |
| engine_distribution | EngineDist[] | 是 | 按计算引擎分组的因子数分布。 |
| generated_at | string \| null | 否 | 生成时刻（当前恒为 null，占位后续物化）。 |
| route_distribution | RouteDist[] | 是 | 按路线分组的因子数分布（路线分布图数据源）。 |
| tag_distribution | TagDist[] | 是 | 按质检标签分组的因子计数（标签分布图数据源）。 |
| total_evaluations | integer | 是 | factor_summary.eval_count 的合计。 |
| total_factors | integer | 是 | 存活因子总数（已排除删除）。 |
| total_routes | integer | 是 | 路线总数。 |

### EngineDist 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| count | integer | 是 | 该引擎下的存活因子数（已排除删除）。 |
| engine | string | 是 | 计算引擎简称：TSA 时序或 CSA 截面。 |
| engine_full | string | 是 | 计算引擎全称，如 TimeSeriesAstEngine。 |

### RouteDist 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| avg_sharpe | number \| null | 否 | alive 因子 mean_sharpe 均值（2 位小数），无则 null。 |
| engine | string | 是 | 该路线因子的计算引擎简称（TSA 或 CSA）。 |
| factor_count | integer | 是 | 该路线存活因子数（已排除删除）。 |
| route_code | string | 是 | 路线编码，12 位十六进制，贯穿挖掘到策略的溯源主键。 |
| route_name | string | 是 | 路线名称，缺失时回落为 route_code。 |
| top_factors | RouteTopFactor[] | 是 | 该路线按夏普降序的 TOP6 存活因子；前端路线卡直接渲染，替代每路线单独查列表。 |
| total | integer | 是 | 该路线全部因子数（含已删除）。 |

### TagDist 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| count | integer | 是 | 带该标签的因子数。 |
| tag | string | 是 | 质检标签名，如 detect_passed（体检通过）或 positive_passed（正收益通过）。 |

### RouteTopFactor 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| factor_name | string | 是 | 因子名。 |
| sharpe | number \| null | 否 | 训练集·多空 夏普比率（跨 problem 均值），无评估则 null。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/factors/summary" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted_factors": 471,
    "engine_distribution": [
      { "count": 186833, "engine": "TSA", "engine_full": "TimeSeriesAstEngine" }
    ],
    "generated_at": null,
    "route_distribution": [
      {
        "avg_sharpe": 0.03,
        "engine": "TSA",
        "factor_count": 13311,
        "route_code": "53673ab8cfd4",
        "route_name": "RF4顶底背离非对称强度因子",
        "top_factors": [
          { "factor_name": "TSA_260714_V060WBP5", "sharpe": 0.6125 },
          { "factor_name": "TSA_260714_XJTN53JP", "sharpe": 0.4842 }
        ],
        "total": 13311
      },
      {
        "avg_sharpe": null,
        "engine": "TSA",
        "factor_count": 11238,
        "route_code": "c3f662edca49",
        "route_name": "路径一致性趋势增强时序因子",
        "top_factors": [
          { "factor_name": "TSA_260608_009540", "sharpe": null },
          { "factor_name": "TSA_260608_012D51", "sharpe": null }
        ],
        "total": 11238
      },
      {
        "avg_sharpe": null,
        "engine": "TSA",
        "factor_count": 9848,
        "route_code": "34f8b4a969f0",
        "route_name": "RF4背离强度动量翻转因子",
        "top_factors": [
          { "factor_name": "TSA_260715_00AGQALT", "sharpe": null },
          { "factor_name": "TSA_260715_00AIHL79", "sharpe": null }
        ],
        "total": 9848
      }
    ],
    "tag_distribution": [
      { "count": 187304, "tag": "positive_passed" },
      { "count": 184529, "tag": "detect_passed" },
      { "count": 2584, "tag": "positive_60min_passed" }
    ],
    "total_evaluations": 1542400,
    "total_factors": 186833,
    "total_routes": 51
  }
}
```
