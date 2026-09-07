---
title: 策略研究产出列表
description: 胜可知开放平台 GET /research/experiments/{id}/strategies：策略研究产出列表。
source: https://docs.shengkezhi.com/api/research/get-experiments-id-strategies
---

# 策略研究产出列表

**`GET /research/experiments/{id}/strategies`** — 策略研究产出列表。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | `ExperimentStrategyItem`[] | 是 | 本次执行产出的策略清单项。 |
| `total` | integer | 是 | 策略总数。 |

### ExperimentStrategyItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | `string` | 是 | 策略编号（`前缀_频率_内容哈希`，由问题编号与因子集合确定）。 |
| `model` | object | 是 | 建模算法名称，取自策略 toml 的 `model_config`。 |
| `route` | object | 是 | 该策略所属的因子路线编号（route_code，取自策略 toml 所在目录）。 |
| `passed` | `boolean` | 是 | 是否通过复审（依据 review_candidates 通过清单）。 |
| `factor_count` | `integer` | 是 | 策略内嵌的因子数量。 |
| `metrics` | object | 是 | 回测指标 map，键为中文指标名（如夏普比率、最大回撤）。 |
| `verdict` | object | 是 | 回测结论。 |
| `start_date` | object | 是 | 回测起始日期。 |
| `end_date` | object | 是 | 回测结束日期。 |
| `symbol_count` | object | 是 | 回测覆盖的标的数量。 |
| `weight_type` | object | 是 | 持仓权重类型。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments/a79dfc93b7e64a6cbbe26f2a787a6bad/strategies?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "code": "FTS_1D_0UCFSYXF",
        "model": "MA001",
        "route": "08c61c074b15",
        "passed": true,
        "factor_count": 21,
        "metrics": {
          "下行波动率": 0.0214,
          "交易次数": 333536,
          "交易胜率": 0.565,
          "单笔收益": 22.27,
          "单笔盈亏比": 0.991,
          "卡玛比率": 1.5642,
          "周胜率": 0.5795,
          "品种数量": 20,
          "夏普比率": 1.5999,
          "多头占比": 0.4887,
          "季胜率": 0.7188,
          "年化交易次数": 43302.97,
          "年化收益": 0.0512,
          "年化波动率": 0.032,
          "年胜率": 0.75,
          "开始日期": "2017-01-04",
          "持仓K线数": 4.32,
          "新高占比": 0.1283,
          "新高间隔": 497.0,
          "日胜率": 0.5513,
          "最大回撤": 0.0327,
          "月胜率": 0.6667,
          "空头占比": 0.4957,
          "结束日期": "2024-12-31",
          "绝对收益": 0.3942
        },
        "verdict": {
          "alpha_degenerate": false,
          "complete_year_count": 8,
          "cond_history_dd_passed": true,
          "cond_history_sharpe_passed": true,
          "cond_yearly_passed": true,
          "history_alpha_max_drawdown": 0.24808602243789313,
          "history_alpha_sharpe": 1.1962296317575214,
          "is_good": true,
          "mode": "history",
          "reason": "",
          "yearly_metrics": [
            {
              "abs_return": 0.04927480026285662,
              "alpha_max_drawdown": 0.10495726818799696,
              "alpha_return": 0.19436459813906945,
              "days": 243,
              "is_complete_year": true,
              "year": 2017,
              "year_passed": true
            },
            {
              "abs_return": 0.04064282549552944,
              "alpha_max_drawdown": 0.04911152318362276,
              "alpha_return": 0.24717638960922092,
              "days": 243,
              "is_complete_year": true,
              "year": 2018,
              "year_passed": true
            }
          ]
        },
        "start_date": "2017-01-04",
        "end_date": "2024-12-31",
        "symbol_count": 20,
        "weight_type": "ts"
      },
      {
        "code": "FTS_1D_EIW1I91O",
        "model": "TS002",
        "route": "08c61c074b15",
        "passed": true,
        "factor_count": 5,
        "metrics": {
          "下行波动率": 0.0436,
          "交易次数": 143920,
          "交易胜率": 0.517,
          "单笔收益": 72.86,
          "单笔盈亏比": 1.4278,
          "卡玛比率": 1.1152,
          "周胜率": 0.5379,
          "品种数量": 20,
          "夏普比率": 1.1874,
          "多头占比": 0.5195,
          "季胜率": 0.625,
          "年化交易次数": 18685.13,
          "年化收益": 0.0693,
          "年化波动率": 0.0583,
          "年胜率": 0.75,
          "开始日期": "2017-01-04",
          "持仓K线数": 14.14,
          "新高占比": 0.1108,
          "新高间隔": 250.0,
          "日胜率": 0.5337,
          "最大回撤": 0.0621,
          "月胜率": 0.5625,
          "空头占比": 0.4595,
          "结束日期": "2024-12-31",
          "绝对收益": 0.5335
        },
        "verdict": {
          "alpha_degenerate": false,
          "complete_year_count": 8,
          "cond_history_dd_passed": true,
          "cond_history_sharpe_passed": true,
          "cond_yearly_passed": true,
          "history_alpha_max_drawdown": 0.2065198535193009,
          "history_alpha_sharpe": 0.9383378953996445,
          "is_good": true,
          "mode": "history",
          "reason": "",
          "yearly_metrics": [
            {
              "abs_return": 0.09952923247170166,
              "alpha_max_drawdown": 0.13156157051230288,
              "alpha_return": 0.17571795596304526,
              "days": 243,
              "is_complete_year": true,
              "year": 2017,
              "year_passed": true
            },
            {
              "abs_return": -0.012265993590489774,
              "alpha_max_drawdown": 0.1075926891198638,
              "alpha_return": -0.010195254182048613,
              "days": 243,
              "is_complete_year": true,
              "year": 2018,
              "year_passed": true
            }
          ]
        },
        "start_date": "2017-01-04",
        "end_date": "2024-12-31",
        "symbol_count": 20,
        "weight_type": "ts"
      },
      {
        "code": "FTS_1D_F70MAC1X",
        "model": "MA001",
        "route": "08c61c074b15",
        "passed": true,
        "factor_count": 3,
        "metrics": {
          "下行波动率": 0.0245,
          "交易次数": 735980,
          "交易胜率": 0.5434,
          "单笔收益": 10.27,
          "单笔盈亏比": 0.9623,
          "卡玛比率": 0.9041,
          "周胜率": 0.5868,
          "品种数量": 20,
          "夏普比率": 1.3438,
          "多头占比": 0.4586,
          "季胜率": 0.7188,
          "年化交易次数": 95552.27,
          "年化收益": 0.0511,
          "年化波动率": 0.038,
          "年胜率": 0.75,
          "开始日期": "2017-01-04",
          "持仓K线数": 3.41,
          "新高占比": 0.117,
          "新高间隔": 538.0,
          "日胜率": 0.5384,
          "最大回撤": 0.0565,
          "月胜率": 0.6771,
          "空头占比": 0.4773,
          "结束日期": "2024-12-31",
          "绝对收益": 0.3935
        },
        "verdict": {
          "alpha_degenerate": false,
          "complete_year_count": 8,
          "cond_history_dd_passed": true,
          "cond_history_sharpe_passed": true,
          "cond_yearly_passed": true,
          "history_alpha_max_drawdown": 0.2311517608266589,
          "history_alpha_sharpe": 1.101793576592828,
          "is_good": true,
          "mode": "history",
          "reason": "",
          "yearly_metrics": [
            {
              "abs_return": 0.04958224258904782,
              "alpha_max_drawdown": 0.10266360204818198,
              "alpha_return": 0.023068912941101194,
              "days": 243,
              "is_complete_year": true,
              "year": 2017,
              "year_passed": true
            },
            {
              "abs_return": 0.04557812368940385,
              "alpha_max_drawdown": 0.03989846947993503,
              "alpha_return": 0.15670609572373878,
              "days": 243,
              "is_complete_year": true,
              "year": 2018,
              "year_passed": true
            }
          ]
        },
        "start_date": "2017-01-04",
        "end_date": "2024-12-31",
        "symbol_count": 20,
        "weight_type": "ts"
      }
    ],
    "total": 6
  }
}```
