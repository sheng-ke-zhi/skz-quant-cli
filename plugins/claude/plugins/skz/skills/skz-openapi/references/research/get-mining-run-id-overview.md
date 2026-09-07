---
title: 挖掘结果概览
description: 胜可知开放平台 GET /research/mining/{run_id}/overview：挖掘结果概览。
source: https://docs.shengkezhi.com/api/research/get-mining-run-id-overview
---

# 挖掘结果概览

**`GET /research/mining/{run_id}/overview`** — 挖掘结果概览。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `run_id` | path | string | 是 | `-` | 挖掘执行 ID |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `elimination_breakdown` | `EliminationItem`[] | 是 | 淘汰分布明细，按终态原因归类，合计等于候选总数。 |
| `funnel` | `FunnelStage`[] | 是 | 挖掘漏斗各阶段的剩余与淘汰计数。 |
| `kpi` | `MiningKpi` | 是 | 概览 KPI 指标块（候选、保留、淘汰、评估次数等）。 |
| `problem_groups` | `ProblemGroup`[] | 是 | 研究问题按前缀分组的计数。 |
| `route` | object | 是 | 本 run 的因子路线（结构同 factor-routes 项，failure_scenarios/tags 为落库 JSON 原样）。 |
| `run_dir` | string | 是 | 该次挖掘产物在 workspace 内的相对路径。 |
| `run_id` | string | 是 | 单次挖掘记录 ID。 |

### EliminationItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `count` | `integer` | 是 | 该归类下的因子数（各项合计等于候选总数）。 |
| `kind` | `string` | 是 | 条目类型：`retained`（保留）或 `eliminated`（淘汰）。 |
| `reason` | `string` | 是 | 归类原因中文名，如「高相关冗余」「体检淘汰」「正向测试淘汰」「终选保留」。 |
| `stage` | `string` | 是 | 对应漏斗阶段英文键（同 `FunnelStage.step`）。 |

### FunnelStage 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `eliminated` | `integer` | 是 | 该阶段淘汰的因子数。 |
| `remaining` | `integer` | 是 | 该阶段结束后剩余的因子数。 |
| `stage` | `string` | 是 | 漏斗阶段中文名，如「生成」「去重初筛」「因子体检」「正向测试」「终选保留」。 |
| `step` | `string` | 是 | 漏斗阶段英文键，如 `build`、`duplicate`、`detect`、`positive`、`filter`。 |

### MiningKpi 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `eliminated` | `integer` | 是 | 淘汰因子数，即候选数减保留数。 |
| `evaluate_methods` | `string[]` | 是 | 本次挖掘使用的全部评估方法编码。 |
| `problem_count` | `integer` | 是 | 本次挖掘覆盖的研究问题（problem）数。 |
| `retain_rate` | `number` | 是 | 保留率，小数表示。 |
| `retained` | `integer` | 是 | 终选保留因子数 —— 晋升进用户库（`positive_passed`）的因子数，与 `/api/factors` 可查集一致。 |
| `total_candidates` | `integer` | 是 | 候选因子总数（生成阶段全部因子）。 |
| `total_evaluations` | `integer` | 是 | 因子评估总次数，跨全部研究问题累计。 |

### ProblemGroup 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `count` | `integer` | 是 | 该组内去重后的研究问题数。 |
| `label` | `string` | 是 | 分组前缀对应的中文名。 |
| `prefix` | `string` | 是 | 研究问题分组前缀：`FTS`（期货时序）、`STS`（股票时序）或 `ETS`（ETF 时序）。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/mining/b8df7d3a4d0144f1bc8bf4f9d9dee365/overview" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "elimination_breakdown": [
      {
        "count": 53,
        "kind": "retained",
        "reason": "终选保留",
        "stage": "filter"
      },
      {
        "count": 2178,
        "kind": "eliminated",
        "reason": "高相关冗余",
        "stage": "duplicate"
      },
      {
        "count": 0,
        "kind": "eliminated",
        "reason": "体检淘汰",
        "stage": "detect"
      }
    ],
    "funnel": [
      {
        "eliminated": 0,
        "remaining": 2900,
        "stage": "生成",
        "step": "build"
      },
      {
        "eliminated": 2178,
        "remaining": 722,
        "stage": "去重初筛",
        "step": "duplicate"
      },
      {
        "eliminated": 0,
        "remaining": 722,
        "stage": "因子体检",
        "step": "detect"
      }
    ],
    "kpi": {
      "eliminated": 2847,
      "evaluate_methods": [
        "MA001",
        "TS001"
      ],
      "problem_count": 16,
      "retain_rate": 0.018275862068965518,
      "retained": 53,
      "total_candidates": 2900,
      "total_evaluations": 3392
    },
    "problem_groups": [
      {
        "count": 3,
        "label": "ETF时序",
        "prefix": "ETS"
      },
      {
        "count": 10,
        "label": "期货时序",
        "prefix": "FTS"
      },
      {
        "count": 3,
        "label": "股票时序",
        "prefix": "STS"
      }
    ],
    "route": {
      "code": "fde0cb170b25",
      "compute_engine": "TimeSeriesAstEngine",
      "create_time": "2026-08-05 11:50:00.493354",
      "creator": "SKZ_ExampleModel",
      "economic_logic": "趋势刚形成时，共识开始增强，但价格通常还没透支；后续跟随资金会推动行情延续。",
      "failure_scenarios": [
        "消息刺激造成短暂假突破",
        "宽幅震荡中均线频繁转向"
      ],
      "key_inspect": "观察5日均线由抖动转为持续上倾，同时价格刚脱离震荡区、但尚未明显远离均线的阶段。",
      "market_mechanism": "趋势跟踪",
      "name": "均线由抖转顺：捕捉趋势缓慢起步",
      "tags": [
        "5日均线",
        "斜率"
      ],
      "why_effective": "它不追求买在最低点，而是在趋势得到初步确认、风险仍可贴近控制时上车，兼顾胜率和盈亏比。"
    },
    "run_dir": "pipeline_results/route_factor_miner/b8df7d3a4d0144f1bc8bf4f9d9dee365",
    "run_id": "b8df7d3a4d0144f1bc8bf4f9d9dee365"
  }
}```
