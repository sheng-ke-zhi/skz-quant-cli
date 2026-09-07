---
title: 实盘策略指标
endpoint: GET /research/strategies/{code}/metrics
source: https://docs.shengkezhi.com/api/research/get-strategies-code-metrics
---

# 实盘策略指标

`GET /research/strategies/{code}/metrics` — 实盘策略指标。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/metrics`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 响应 data

指标名到数值的动态映射，键集合随策略评估口径扩展（实测样例含 下行波动率、交易次数、交易胜率、单笔收益、单笔盈亏比、卡玛比率、周胜率、品种数量 等），值均为 number。指标口径详见绩效指标解读。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/metrics" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "下行波动率": 0.0546,
    "交易次数": 12772,
    "交易胜率": 0.4452,
    "单笔收益": 7.88,
    "单笔盈亏比": 1.4146,
    "卡玛比率": 0.6927,
    "周胜率": 0.494,
    "品种数量": 2,
    "夏普比率": 0.4877,
    "多头占比": 0.452,
    "季胜率": 0.5714,
    "年化交易次数": 8403.51,
    "年化收益": 0.0372,
    "年化波动率": 0.0762,
    "年胜率": 0.5,
    "开始日期": "2025-01-17",
    "持仓K线数": 9.68,
    "新高占比": 0.0601,
    "新高间隔": 228.0,
    "日胜率": 0.4909,
    "最大回撤": 0.0537,
    "月胜率": 0.45,
    "空头占比": 0.537,
    "结束日期": "2026-08-21",
    "绝对收益": 0.0565
  }
}
```
