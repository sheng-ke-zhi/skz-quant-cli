---
title: 实盘策略周期收益
description: 胜可知开放平台 GET /research/strategies/{code}/periodic：实盘策略周期收益。
source: https://docs.shengkezhi.com/api/research/get-strategies-code-periodic
---

# 实盘策略周期收益

**`GET /research/strategies/{code}/periodic`** — 实盘策略周期收益。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `monthly` | `PeriodicMonthly` | 是 | 月度收益：年份与月份轴加上逐年逐月的收益矩阵，喂月度热力图。 |
| `yearly` | object（动态键，值为 number） | 是 | 年度收益：年份字符串到该年累计收益的映射，喂年度收益柱图。 |

### PeriodicMonthly 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `years` | `integer[]` | 是 | 覆盖的年份列表（升序），对应月度矩阵的行。 |
| `months` | `integer[]` | 是 | 月份列表，固定为 1 到 12，对应月度矩阵的列。 |
| `matrix` | `number[][]` | 是 | 逐年 × 12 月的月度收益矩阵，缺月为 null。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/periodic" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "monthly": {
      "years": [
        2025,
        2026
      ],
      "months": [
        1,
        2
      ],
      "matrix": [
        [
          -0.000976838153664531,
          -0.006986891372798237
        ],
        [
          0.006793218252199779,
          0.034930561121631384
        ]
      ]
    },
    "yearly": {
      "2025": -0.01830483381487667,
      "2026": 0.07480383457195736
    }
  }
}```
