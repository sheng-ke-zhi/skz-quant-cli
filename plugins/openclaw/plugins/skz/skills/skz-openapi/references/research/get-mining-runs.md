---
title: 因子挖掘记录
endpoint: GET /research/mining/runs
source: https://docs.shengkezhi.com/api/research/get-mining-runs
---

# 因子挖掘记录

`GET /research/mining/runs` — 因子挖掘记录。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/mining/runs`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| route_code | query | string | 否 | - | 只返回属于指定研究路线的挖掘记录。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | MiningRunItem[] | 是 | 挖掘记录列表项。 |
| total | integer | 是 | 挖掘记录总数。 |

### MiningRunItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| elapsed_s | integer | 是 | 全程墙钟耗时（秒），取自库内时间戳跨度。 |
| retain_rate | number | 是 | 保留率，保留数除以候选数，小数表示。 |
| retained | integer | 是 | 终选保留因子数 —— 即晋升进用户库（positive_passed）、/api/factors 可查的因子数，非暂存库未删除数（后者含体检 / 正测未过但未软删的候选，不进用户库）。 |
| route_code | string | 是 | 因子路线编号，12 位 hex，贯穿规划到挖掘的全流程溯源主键。 |
| route_name | string | 是 | 因子路线名称，如「量价共振趋势因子」。 |
| run_id | string | 是 | 单次挖掘记录 ID，格式 {时间戳}_{route_code前8}，如 20260531_053509_90ec76ba。 |
| started_at | string \| null | 否 | run 开始时间（库内最早 create_time）；空则 null。 |
| status | string | 是 | 派生 run 状态：succeeded（有晋升用户库的因子）或 no_factors（有构建但零晋升）或 build_failed（零构建）。 |
| total_candidates | integer | 是 | 本次挖掘生成的候选因子总数。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/mining/runs?page=1&page_size=1" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "elapsed_s": 756,
        "retain_rate": 0.018275862068965518,
        "retained": 53,
        "route_code": "fde0cb170b25",
        "route_name": "均线由抖转顺：捕捉趋势缓慢起步",
        "run_id": "b8df7d3a4d0144f1bc8bf4f9d9dee365",
        "started_at": "2026-08-20 17:03:38",
        "status": "succeeded",
        "total_candidates": 2900
      },
      {
        "elapsed_s": 949,
        "retain_rate": 0.005185185185185185,
        "retained": 14,
        "route_code": "08c61c074b15",
        "route_name": "ATR 百分比波动率 regime 时序因子",
        "run_id": "b3b5848f5d8c42e9aa3f413289c69186",
        "started_at": "2026-08-18 19:42:42",
        "status": "succeeded",
        "total_candidates": 2700
      },
      {
        "elapsed_s": 1065,
        "retain_rate": 0.03766666666666667,
        "retained": 113,
        "route_code": "fde0cb170b25",
        "route_name": "均线由抖转顺：捕捉趋势缓慢起步",
        "run_id": "434ce136d84840629fc8e0f6ed575504",
        "started_at": "2026-08-18 11:28:52",
        "status": "succeeded",
        "total_candidates": 3000
      }
    ],
    "total": 92
  }
}
```

## 与「因子挖掘任务列表」的区别

本接口读取已经落入工作区的挖掘产物。查看任务编排与运行状态时，请使用策略接口的[因子挖掘任务列表](https://docs.shengkezhi.com/api/strategy/miner-list-runs)（`../strategy/miner-list-runs`）。
