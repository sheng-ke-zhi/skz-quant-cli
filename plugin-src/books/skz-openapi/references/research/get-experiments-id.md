---
title: 策略研究执行详情
description: 胜可知开放平台 GET /research/experiments/{id}：策略研究执行详情。
source: https://docs.shengkezhi.com/api/research/get-experiments-id
---

# 策略研究执行详情

**`GET /research/experiments/{id}`** — 策略研究执行详情。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 业务状态

数据不存在或产物尚未就绪时，HTTP 状态仍为 `200`，响应信封使用业务码 `42201`，`data` 为 `null`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `overview` | `ExperimentOverview` | 是 | 单次执行的概要聚合。 |

### ExperimentOverview 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `problem_name` | object | 是 | 研究问题名称，取自策略 problem 定义。 |
| `problem_code` | object | 是 | 研究问题编号，大写加下划线命名，其首段用作策略编号前缀。 |
| `problem_type` | object | 是 | 研究问题类型（如时序 `TimeSeriesProblem` 或多空/多头收益类）。 |
| `description` | object | 是 | 研究问题的文字描述。 |
| `freq` | object | 是 | 行情频率（如「日线」）。 |
| `dataset` | object | 是 | 使用的数据集（如 `stock` 股票）。 |
| `symbols` | object | 是 | 研究标的清单。 |
| `time_segments` | object | 是 | 时段划分清单，每段含名称与起止（训练集各段、训练集、后置验证/样本外）。 |
| `run_at` | object | 是 | 本次复审运行时刻。 |
| `scanned` | object | 是 | 复审扫描的策略数。 |
| `passed` | object | 是 | 复审通过的策略数。 |
| `failed` | object | 是 | 复审淘汰的策略数。 |
| `skipped` | object | 是 | 复审跳过的策略数。 |
| `pass_rate` | `number` \| null | 否 | 复审通过率，后端按通过数除以扫描数计算。 |
| `elapsed_s` | object | 是 | 复审耗时（秒）。 |
| `review_fn` | object | 是 | 本次复审所用 review_fn 的引用路径。 |
| `status` | object | 是 | 探索执行状态。 |
| `errors` | object | 是 | 探索过程中的错误信息。 |
| `n_strategies` | object | 是 | 本次探索创建的策略总数。 |
| `n_backtests` | object | 是 | 本次执行的回测次数。 |
| `total_elapsed` | object | 是 | 本次探索总耗时（秒）。 |
| `model_configs_used` | object | 是 | 本次探索使用到的建模配置清单。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments/a79dfc93b7e64a6cbbe26f2a787a6bad" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "overview": {
      "problem_name": "低保证金品种日线",
      "problem_code": "FTS_PROBLEM_23098B",
      "problem_type": "TimeSeriesProblem",
      "description": "期货低保证金品种日线",
      "freq": "日线",
      "dataset": "future",
      "symbols": [
        "FG999.ZCE",
        "MA999.ZCE"
      ],
      "time_segments": [
        {
          "edt": "20190101",
          "name": "训练集A段",
          "sdt": "20170101"
        },
        {
          "edt": "20210101",
          "name": "训练集B段",
          "sdt": "20190101"
        }
      ],
      "run_at": "2026-08-18T12:18:30.612759+00:00",
      "scanned": 23,
      "passed": 6,
      "failed": 5,
      "skipped": 0,
      "pass_rate": 0.2608695652173913,
      "elapsed_s": 5.669,
      "review_fn": "skz_strategy_research.strategy_filter.review:default_review_fn",
      "status": "ok",
      "errors": [],
      "n_strategies": 23,
      "n_backtests": 23,
      "total_elapsed": 552.433813733,
      "model_configs_used": [
        "MA001",
        "TS001"
      ]
    }
  }
}```
