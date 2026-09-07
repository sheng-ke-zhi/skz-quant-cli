---
title: 因子详情
description: 胜可知开放平台 GET /research/factors/{factor_name}：因子详情。
source: https://docs.shengkezhi.com/api/research/get-factors-factor-name
---

# 因子详情

**`GET /research/factors/{factor_name}`** — 因子详情。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `factor_name` | path | string | 是 | `-` | 因子名称 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `compute_engine` | string | 是 | 计算引擎简称：`TSA` 时序或 `CSA` 截面。 |
| `create_time` | string | 是 | 因子创建时间。 |
| `creator` | string | 是 | 因子创建者：生成该因子的 agent 或模型标识。 |
| `delete_reason` | string \| null | 否 | 删除原因；未删除或老库缺该列时为 null。 |
| `description` | string | 是 | 因子描述：逻辑说明与已知局限。 |
| `engine_full` | string | 是 | 计算引擎全称，如 `TimeSeriesAstEngine`。 |
| `evaluations` | `FactorEvaluation`[] | 是 | 逐 problem 评估摘要。 |
| `factor_code` | string | 是 | 因子 AST 源码。 |
| `factor_name` | string | 是 | 因子名，格式 `引擎_YYMMDD_HASH6`，系统按内容哈希自动生成，不可手命名。 |
| `is_deleted` | boolean | 是 | 是否已删除。 |
| `route` | string | 是 | 所属路线编码。 |
| `route_name` | string | 是 | 所属路线名称，缺失时回落为路线编码。 |
| `tags` | `FactorTagDetail`[] | 是 | 质检标签明细列表（含 detail 说明）。 |

### FactorEvaluation 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `problem` | `string` | 是 | 研究问题编号（input_data 列，如 `FTS_PROBLEM_...`），标识本条评估针对哪个 problem。 |
| `method` | `string` | 是 | 评估方法编码，如 `MA001`，决定指标口径与时段划分。 |
| `status` | `string` | 是 | 评估状态（此处恒为 `success`，仅输出成功评估）。 |
| `sharpe` | `number` \| null | 否 | 代表性夏普（训练集·多空口径，与排行榜聚合同口径），供单因子详情夏普分布图；无则 null。 |
| `calmar` | `number` \| null | 否 | 代表性卡玛（训练集·多空口径），供单因子详情卡玛分布图；无则 null。 |

### FactorTagDetail 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `tag` | `string` | 是 | 质检标签名，如 `detect_passed`、`positive_passed`。 |
| `detail` | `string` | 是 | 标签明细，通常为质检结果 JSON 字符串（如通过的 problem 列表或各段正收益比例）。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/factors/TSA_260820_N26M85RL" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "factor_name": "TSA_260820_N26M85RL",
    "factor_code": "ma10 = Mean($close, 10)\nsl = Sub(ma10, Ref(ma10, 1))\nbkt = TSPctChange(sl, 5)\nRank(bkt, 60)",
    "compute_engine": "TSA",
    "engine_full": "TimeSeriesAstEngine",
    "description": "均线5期变化率60日排名；极低波动排名极端集中失效",
    "creator": "SKZ_ExampleModel",
    "create_time": "2026-08-20 17:10:07.083924",
    "route": "fde0cb170b25",
    "route_name": "均线由抖转顺：捕捉趋势缓慢起步",
    "is_deleted": false,
    "delete_reason": null,
    "tags": [
      {
        "tag": "detect_passed",
        "detail": "{\"_schema_version\": 1, \"problems\": [\"FTS_PROBLEM_F60_9E6B8CC7\"], \"checks_passed\": [\"finiteness\", \"future_info\", \"variance\", \"distribution\", \"rolling\"]}"
      },
      {
        "tag": "positive_passed",
        "detail": "{\"_schema_version\": 1, \"methods\": {\"ratio_train_a\": {\"passed\": true, \"reason\": \"训练集A段 正收益比例 0.695 > 0.618\"}, \"ratio_train_b\": {\"passed\": true, \"reason\": \"训练集..."
      }
    ],
    "evaluations": [
      {
        "problem": "FTS_PROBLEM_D_60144637",
        "method": "MA001",
        "status": "success",
        "sharpe": 0.878,
        "calmar": 0.5328
      },
      {
        "problem": "FTS_PROBLEM_D_60144637",
        "method": "TS001",
        "status": "success",
        "sharpe": 1.2202,
        "calmar": 1.1559
      },
      {
        "problem": "FTS_PROBLEM_D_60144637",
        "method": "TS002",
        "status": "success",
        "sharpe": 0.6851,
        "calmar": 0.6618
      }
    ]
  }
}```
