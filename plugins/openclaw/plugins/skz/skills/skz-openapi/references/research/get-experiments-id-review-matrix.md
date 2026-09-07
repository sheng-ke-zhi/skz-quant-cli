---
title: 策略研究评审矩阵
description: 胜可知开放平台 GET /research/experiments/{id}/review-matrix：策略研究评审矩阵。
source: https://docs.shengkezhi.com/api/research/get-experiments-id-review-matrix
---

# 策略研究评审矩阵

**`GET /research/experiments/{id}/review-matrix`** — 策略研究评审矩阵。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `id` | path | string | 是 | `-` | 策略探索实验编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | object[] | 是 | 各策略在各时段的指标行，键为策略与中文指标名（自由形）。 |
| `segments` | string[] | 是 | 矩阵中出现过的时段名称集合（训练集各段、训练集、后置验证等）。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/experiments/a79dfc93b7e64a6cbbe26f2a787a6bad/review-matrix" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "segments": [
      "后置验证",
      "训练集",
      "训练集A段"
    ],
    "items": [
      {
        "edt": "20190101",
        "sdt": "20170101",
        "segment_name": "训练集A段",
        "strategy": "FTS_1D_0UCFSYXF",
        "交易胜率": 0.5799,
        "单笔收益": 21.2,
        "卡玛比率": 2.9805,
        "夏普比率": 1.7183,
        "年化收益": 0.0466,
        "年化波动率": 0.0271,
        "年胜率": 1.0,
        "持仓K线数": 4.26,
        "最大回撤": 0.0156
      },
      {
        "edt": "20210101",
        "sdt": "20190101",
        "segment_name": "训练集B段",
        "strategy": "FTS_1D_0UCFSYXF",
        "交易胜率": 0.5748,
        "单笔收益": 20.02,
        "卡玛比率": 3.7266,
        "夏普比率": 2.0653,
        "年化收益": 0.0493,
        "年化波动率": 0.0238,
        "年胜率": 1.0,
        "持仓K线数": 4.22,
        "最大回撤": 0.0132
      },
      {
        "edt": "20230101",
        "sdt": "20210101",
        "segment_name": "训练集C段",
        "strategy": "FTS_1D_0UCFSYXF",
        "交易胜率": 0.5862,
        "单笔收益": 50.77,
        "卡玛比率": 3.654,
        "夏普比率": 2.5486,
        "年化收益": 0.1189,
        "年化波动率": 0.0467,
        "年胜率": 1.0,
        "持仓K线数": 4.39,
        "最大回撤": 0.0325
      }
    ]
  }
}```
