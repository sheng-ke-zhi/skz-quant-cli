---
title: 策略研究执行列表
description: 胜可知开放平台 GET /research/experiments：策略研究执行列表。
source: https://docs.shengkezhi.com/api/research/get-experiments
---

# 策略研究执行列表

**`GET /research/experiments`** — 策略研究执行列表。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | `ExperimentListItem`[] | 是 | 执行列表项，每项对应一次策略探索执行。 |
| `total` | integer | 是 | 执行总数。 |

### ExperimentListItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `id` | `string` | 是 | 本次执行的唯一编号，即 experiment 目录名（形如 problem_code 加日期时间戳）。 |
| `problem_name` | object | 是 | 研究问题名称，取自策略 problem 定义。 |
| `problem_code` | object | 是 | 研究问题编号，大写加下划线命名，其首段用作策略编号前缀。 |
| `problem_type` | object | 是 | 研究问题类型（如时序 `TimeSeriesProblem` 或多空/多头收益类）。 |
| `description` | object | 是 | 研究问题的文字描述（完整定义），供列表 hover 弹窗展示。 |
| `freq` | object | 是 | 行情频率（如「日线」）。 |
| `dataset` | object | 是 | 使用的数据集（如 `stock` 股票）。 |
| `route` | object | 是 | 本次探索的因子路线编号（route_code，取候选策略 toml 所在目录）；一次探索对应一条路线。 |
| `run_at` | object | 是 | 本次复审运行时刻。 |
| `strategy_count` | `integer` | 是 | 本次实际产出回测的策略数（有回测产物的策略）。 |
| `n_strategies` | object | 是 | 本次探索创建的策略总数（含被剪枝、未回测的）。 |
| `n_backtests` | object | 是 | 本次执行的回测次数。 |
| `status` | object | 是 | 探索执行状态。 |
| `errors` | object | 是 | 探索过程中的错误信息。 |
| `scanned` | `integer` \| null | 否 | 复审扫描的策略数。 |
| `passed` | `integer` \| null | 否 | 复审通过的策略数。 |
| `failed` | object | 是 | 复审淘汰的策略数。 |
| `skipped` | object | 是 | 复审跳过的策略数。 |
| `pass_rate` | `number` \| null | 否 | 复审通过率，后端按通过数除以扫描数计算。 |
| `symbols_count` | `integer` | 是 | 研究标的数量。 |
| `elapsed_s` | object | 是 | 复审耗时（秒）。 |
| `total_elapsed` | object | 是 | 本次策略探索全流程总耗时（秒）。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
        "problem_name": "低保证金品种日线",
        "problem_code": "FTS_PROBLEM_23098B",
        "problem_type": "TimeSeriesProblem",
        "description": "期货低保证金品种日线",
        "freq": "日线",
        "dataset": "future",
        "route": "08c61c074b15",
        "run_at": "2026-08-18T12:18:30.612759+00:00",
        "strategy_count": 6,
        "n_strategies": 23,
        "n_backtests": 23,
        "status": "ok",
        "errors": [],
        "scanned": 23,
        "passed": 6,
        "failed": 5,
        "skipped": 0,
        "pass_rate": 0.2608695652173913,
        "symbols_count": 20,
        "elapsed_s": 5.669,
        "total_elapsed": 552.433813733
      },
      {
        "id": "15eee5fb3db847faab330f9d79a4c962",
        "problem_name": "低保证金品种日线",
        "problem_code": "FTS_PROBLEM_23098B",
        "problem_type": "TimeSeriesProblem",
        "description": "期货低保证金品种日线",
        "freq": "日线",
        "dataset": "future",
        "route": "7ae5eb1f0c6b",
        "run_at": "2026-08-18T12:15:40.572291+00:00",
        "strategy_count": 6,
        "n_strategies": 27,
        "n_backtests": 27,
        "status": "ok",
        "errors": [],
        "scanned": 27,
        "passed": 7,
        "failed": 8,
        "skipped": 0,
        "pass_rate": 0.25925925925925924,
        "symbols_count": 20,
        "elapsed_s": 6.725,
        "total_elapsed": 1912.545435152
      },
      {
        "id": "15bc493822b0478da54c25a3d6e2e8b7",
        "problem_name": "低保证金品种日线",
        "problem_code": "FTS_PROBLEM_23098B",
        "problem_type": "TimeSeriesProblem",
        "description": "期货低保证金品种日线",
        "freq": "日线",
        "dataset": "future",
        "route": "68f42c2521be",
        "run_at": "2026-08-18T12:03:52.601318+00:00",
        "strategy_count": 6,
        "n_strategies": 34,
        "n_backtests": 34,
        "status": "ok",
        "errors": [],
        "scanned": 34,
        "passed": 8,
        "failed": 11,
        "skipped": 0,
        "pass_rate": 0.23529411764705882,
        "symbols_count": 20,
        "elapsed_s": 7.866,
        "total_elapsed": 1204.998990828
      }
    ],
    "total": 39
  }
}```
