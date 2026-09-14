---
title: 创建实盘价值审核任务
description: 胜可知开放平台 POST /research/experiments/{id}/strategies/{code}/promote：创建实盘价值审核任务。
source: https://docs.shengkezhi.com/api/research/post-experiments-id-strategies-code-promote
---

# 创建实盘价值审核任务

**`POST /research/experiments/{id}/strategies/{code}/promote`** — 创建实盘价值审核任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |
| `code` | path | string | 是 | `-` | 候选策略编号 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `memo` | string \| null | 否 | 可选用户笔记，首尾空白会裁剪，最多 10000 个 Unicode 字符；仅首次入库时写入。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `created_at` | string | 是 | 任务创建时间，RFC3339 UTC。 |
| `error` | object \| null | 否 | 任务失败时的错误详情 JSON（含错误码与消息），成功时为 `null`。 |
| `experiment_id` | string \| null | 否 | 该策略所属实验的编号，对应 strategy_exploration 下的实验目录。 |
| `lifecycle` | string \| null | 否 | 策略在实盘跟踪中的生命周期阶段标签，由 promote 请求携带，可为空。 |
| `phase` | string | 是 | 任务当前所处的细粒度阶段，如 `realtime_running` 到 `completed` 或 `failed`。 |
| `promotion_id` | string | 是 | promote 异步任务编号，格式为 `promote_策略_时间戳_序号`，后端自动生成。 |
| `realtime` | object \| null | 否 | FC（Function Compute）实盘回调透传的实时结果 JSON，任务尚未回调时为 `null`。 |
| `registered` | boolean | 是 | 提交 promote 时标记该策略是否已在实盘库登记。 |
| `status` | `PromotionStatus` | 是 | promote 任务的整体状态，取值见 `PromotionStatus`（`running` 或 `succeeded` 或 `failed`）。 |
| `strategy_code` | string | 是 | 被推入实盘跟踪的策略编号，如 `TS_1D_4E70093D`。 |
| `updated_at` | string | 是 | 任务最近一次更新时间，RFC3339 UTC。 |

### PromotionStatus 取值

| 取值 | 说明 |
|---|---|
| `running` | 任务进行中，实盘回调结果尚未返回。 |
| `succeeded` | 任务成功，策略已通过实盘校验并写入实盘跟踪库。 |
| `failed` | 任务失败，实盘回调报错或校验未通过。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/experiments/experiment_20260701_001/strategies/STRAT_MOMENTUM_001/promote" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"memo":"示例Memo"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "created_at": "2026-07-01T08:00:00Z",
    "error": {},
    "experiment_id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
    "lifecycle": "示例Lifecycle",
    "phase": "示例Phase",
    "promotion_id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
    "realtime": {},
    "registered": true,
    "status": "running",
    "strategy_code": "STRAT_MOMENTUM_001",
    "updated_at": "2026-07-01T08:00:00Z"
  }
}```
