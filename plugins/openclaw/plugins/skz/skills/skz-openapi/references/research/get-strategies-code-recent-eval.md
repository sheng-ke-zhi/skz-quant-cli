---
title: 实盘策略近期评价
endpoint: GET /research/strategies/{code}/recent-eval
source: https://docs.shengkezhi.com/api/research/get-strategies-code-recent-eval
---

# 实盘策略近期评价

`GET /research/strategies/{code}/recent-eval` — 实盘策略近期评价。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/recent-eval`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| history | object | 是 | 历史回撤/收益（含中文键）。 |
| history_ok | boolean | 是 | 历史判定：全样本绝对收益为正且最大回撤低于阈值。 |
| is_good | boolean | 是 | 综合评价结论：仅当历史与近期两项均达标时为 true。 |
| params | RecentEvalParams | 是 | 近期评价所用的参数（窗口天数、目标波动率、回撤阈值）。 |
| reason | string | 是 | 达标或不达标的文字说明。 |
| recent | object | 是 | 近一年核心指标（含中文键 + sdt/edt）。 |
| recent_ok | boolean | 是 | 近期判定：近一年收益为正且近期回撤低于去尾历史回撤。 |

### RecentEvalParams 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| recent_days | integer | 是 | 近期评价窗口的交易日天数，默认 252（约近一年）。 |
| target_vol | number | 是 | 目标波动率参数，沿用 is_good_strategy 评价口径。 |
| max_dd_threshold | number | 是 | 历史最大回撤阈值，历史回撤超过该值即判定历史不达标。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/recent-eval" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "is_good": true,
    "history_ok": true,
    "recent_ok": true,
    "reason": "",
    "recent": {
      "下行波动率": 0.0619,
      "交易次数": 8908,
      "交易胜率": 0.4643,
      "单笔收益": 15.71,
      "单笔盈亏比": 1.4628,
      "卡玛比率": 1.6271,
      "周胜率": 0.537,
      "品种数量": 2,
      "夏普比率": 0.8295,
      "多头占比": 0.4385,
      "季胜率": 0.6,
      "年化交易次数": 8908.0,
      "年化收益": 0.0729,
      "年化波动率": 0.0879,
      "年胜率": 1.0,
      "开始日期": "2025-08-05",
      "持仓K线数": 9.94,
      "新高占比": 0.0873,
      "新高间隔": 104.0,
      "日胜率": 0.5119,
      "最大回撤": 0.0448,
      "月胜率": 0.6154,
      "空头占比": 0.5575,
      "结束日期": "2026-08-21",
      "绝对收益": 0.0729
    },
    "history": {
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
    },
    "params": {
      "recent_days": 252,
      "target_vol": 0.2,
      "max_dd_threshold": 0.2
    }
  }
}
```
