---
title: 实盘策略定义
endpoint: GET /research/strategies/{code}/definition
source: https://docs.shengkezhi.com/api/research/get-strategies-code-definition
---

# 实盘策略定义

`GET /research/strategies/{code}/definition` — 实盘策略定义。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/definition`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 响应 data

策略 TOML 定义的完整序列化，实测键集合稳定：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| factors | object[] | 是 | 策略引用的因子定义列表（含因子源码、引擎、创建信息）。 |
| model_config | object | 是 | 组合模型配置（名称、类型与超参）。 |
| post_process | string | 是 | 权重后处理方式，如 WEIGHT。 |
| problem | object | 是 | 研究问题定义（编号、数据集、标的池与多空设置）。 |
| route | string | 是 | 所属因子路线编码。 |
| runtime | object | 是 | 运行时参数（因子失败策略、增量回看窗口等）。 |
| strategy | string | 是 | 策略编号，回显路径参数。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/definition" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "factors": [
      {
        "compute_engine": "TSA",
        "create_time": "2026-06-27T17:56:38.664173",
        "creator": "SKZ_ExampleModel",
        "description": "20日不对称偏离乘以TSNormMms归一化信号，MMS门控凸显不对称；MMS极值失效、停牌跳空污染、单边趋势失效",
        "factor_code": "mx = Max($high, 20)\nmn = Min($low, 20)\nmid = Mean($close, 20)\nasym = Sub(Div(Sub(mx, mid), Add(mid, 1e-6)), Div(Sub(mid, mn), Add(mid, 1e-6)))\nTSNormMms($close, 5, 20)\n",
        "factor_name": "TSA_260627_FSCER9Z8",
        "is_deleted": false,
        "route": "7a31f4c9e102"
      },
      {
        "compute_engine": "TSA",
        "create_time": "2026-06-27T17:06:18.407063",
        "creator": "SKZ_ExampleModel",
        "description": "BGS凸显启发：30日不对称凸显与Stoch %K偏离50相乘，超买超卖加权；%K极值钝化失效；停牌复牌跳空失效；流动性危机极端值系统性失效",
        "factor_code": "high_max = Max($close, 30)\nlow_min = Min($close, 30)\nmean_px = Mean($close, 30)\nstoch_k = TSStochK($high, $low, $close, 14, 3, 0, 3, 0)\nMul(Div(Sub(high_max, mean_px), Add(Sub(mean_px, low_min), 1e-6)), Div(Sub(stoch_k, 50.0), 50.0))\n",
        "factor_name": "TSA_260627_ICG4K8P3",
        "is_deleted": false,
        "route": "7a31f4c9e102"
      },
      {
        "compute_engine": "TSA",
        "create_time": "2026-06-27T17:42:27.247961",
        "creator": "SKZ_ExampleModel",
        "description": "隔夜跳空不对称除以日波动，跳空与日内波动比较视角；std极小分母噪声放大、停牌跳空污染、单边涨失效",
        "factor_code": "ret = Sub($open, Ref($close, 1))\nup = Max(ret, 20)\ndn = Min(ret, 20)\nDiv(Sub(up, Abs(dn)), Add(Std($close, 20), 1e-6))\n",
        "factor_name": "TSA_260627_QSDR6ZBY",
        "is_deleted": false,
        "route": "7a31f4c9e102"
      }
    ],
    "model_config": {
      "kwargs": {},
      "model": "TS001",
      "name": "TS001"
    },
    "post_process": "WEIGHT",
    "problem": {
      "code": "STS_BJ60MIN_LEADERS",
      "dataset": "stock",
      "definitions": ["使用量价时序因子构建择时模型"],
      "description": "A 股白酒板块龙头：贵州茅台，五粮液；60分钟行情",
      "freq": "60分钟",
      "name": "白酒龙头60分钟时序研究",
      "problem_type": "TimeSeriesProblem",
      "special_time_segments": [],
      "symbols": ["600519.SH", "000858.SZ"],
      "time_segments": [
        { "edt": "20190101", "name": "训练集A段", "sdt": "20170101" },
        { "edt": "20210101", "name": "训练集B段", "sdt": "20190101" }
      ]
    },
    "route": "7a31f4c9e102",
    "runtime": {
      "factor_failure_policy": "skip",
      "incremental_lookback_bars": 3000,
      "update_mode": "auto"
    },
    "strategy": "STS_60M_1I7G6TS4"
  }
}
```
