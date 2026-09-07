---
title: 组合详情
endpoint: GET /research/portfolios/{code}
source: https://docs.shengkezhi.com/api/research/get-portfolios-code
---

# 组合详情

`GET /research/portfolios/{code}` — 组合详情。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/portfolios/{code}`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 组合编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| compare | CompareSeries | 是 | 多空、分腿、基准和超额的累计收益对比曲线。 |
| compare_metrics | object | 是 | 多空关键指标对比（leg → 指标 map）。 |
| curves | object（动态键，值为 PerformanceCurve） | 是 | 多空、多头、空头、基准和超额的完整绩效曲线。 |
| drawdowns | DrawdownRow[] | 是 | 主要回撤区间。 |
| has_performance | boolean | 是 | 是否已落盘可展示的绩效 MsgPack；false 时可使用刷新接口生成。 |
| has_report | boolean | 是 | 是否存在可由 /api/portfolios/{code}/report 获取的 HTML 完整报告。 |
| latest_weights | TargetWeight[] | 是 | 最近一个再平衡日的目标标的权重。 |
| latest_weights_at | string | 是 | latest_weights 对应的数据日期，格式 YYYY-MM-DD。 |
| meta | PortfolioMeta | 是 | 组合配置与构建元信息。 |
| metrics | object | 是 | 全样本核心指标（中文键 → 数值），直读 MsgPack 的 stats。 |
| monthly | MonthlyRow[] | 是 | 按年份组织的月度收益矩阵。 |
| nav | NavSeries | 是 | 组合净值、回撤和累计收益曲线。 |
| normalized_20 | object（动态键，值为 PerformanceCurve） | 是 | 各绩效腿按 20% 年化波动率归一化后的曲线。 |
| positions | Positions | 是 | 历史每日持仓权重矩阵。 |
| rebalance_dates | string[] | 是 | 配置的再平衡日期，格式 YYYY-MM-DD。 |
| strategies | PortfolioStrategy[] | 是 | 参与组合的策略及组合层权重。 |
| symbol_returns | SymbolReturn[] | 是 | 各标的累计收益贡献，供归因排序展示。 |
| verdict | object | 是 | wbt is_good_strategy 判定（history + recent）。 |
| yearly | object（动态键，值为 number） | 是 | 年度收益，年份字符串映射至收益率。 |

### CompareSeries 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dates | string[] | 是 | 交易日期轴，所有归因曲线均与其等长。 |
| series | object | 是 | leg 名到累计收益序列的映射；常见键为 多空、多头、空头、基准、超额。 |

### PerformanceCurve 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| cum | number[] | 是 | 单利累计收益序列。 |
| daily | number[] | 是 | 日收益序列，与组合交易日期轴等长。 |
| drawdown | number[] | 是 | 相对历史累计收益高点的回撤序列。 |

### DrawdownRow 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| depth | number | 是 | 回撤深度，小数表示且不大于 0。 |
| drawdown_days | integer | 是 | 从开始至谷底的自然日或交易日计数，口径由 wbt 产物决定。 |
| end | string | 是 | 回撤谷底日期。 |
| new_high_gap | integer \| null | 否 | 相邻新高之间的天数；不可计算时为 null。 |
| recover | string \| null | 否 | 净值恢复至前高的日期；尚未恢复时为 null。 |
| recover_days | integer \| null | 否 | 从谷底至恢复的天数；尚未恢复时为 null。 |
| start | string | 是 | 回撤开始日期。 |

### TargetWeight 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| symbol | string | 是 | 标的代码，例如 600519.SH。 |
| weight | number | 是 | 目标权重，小数表示；正数为多头，负数为空头。 |

### PortfolioMeta 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code | string | 是 | 组合编号。 |
| description | string | 是 | 组合描述。 |
| status | string | 是 | 组合业务状态。 |
| base_market | string | 是 | 基础市场代码。 |
| base_freq | string | 是 | 基础数据频率。 |
| price_field | string | 是 | 回测价格字段，例如 close。 |
| rebalance_method | string | 是 | 再平衡方法，例如 equal_weight。 |
| lookback_days | integer | 是 | 优化器使用的历史回看交易日数。 |
| config_hash | string | 是 | 生成配置的内容哈希，用于追踪产物版本。 |
| generated_at | string | 是 | 产物生成时间，RFC3339 或 workspace 产物记录的等价格式。 |
| fee_bp | number | 是 | 单边费率，单位为基点（bp）。 |
| digits | integer | 是 | 权重保存的小数位数。 |
| symbol_count | integer | 是 | 回测覆盖的标的数量。 |
| sdt | string | 是 | 回测起始日期，格式 YYYY-MM-DD。 |
| edt | string | 是 | 回测结束日期，格式 YYYY-MM-DD。 |

### MonthlyRow 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| values | number[] | 是 | 12 个月的收益率；无数据的月份为 null。 |
| year | integer | 是 | 公历年份。 |

### NavSeries 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dates | string[] | 是 | 交易日期轴，格式 YYYY-MM-DD。 |
| nav | number[] | 是 | 累计净值，与 dates 等长，通常从 1.0 起。 |
| drawdown | number[] | 是 | 回撤序列，与 dates 等长且不大于 0。 |
| cum_return | number[] | 是 | 累计收益序列，与 dates 等长，等于净值减 1。 |

### Positions 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dates | string[] | 是 | 持仓日期轴，格式 YYYY-MM-DD。 |
| symbols | string[] | 是 | 权重矩阵覆盖的全部标的代码。 |
| weights | object | 是 | symbol → 每日权重（对齐 dates，缺失为 null）。 |

### PortfolioStrategy 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| strategy_id | string | 是 | 实盘策略编号。 |
| weight | number | 是 | 策略在组合中的配置权重，小数表示。 |

### SymbolReturn 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| ret | number | 是 | 该标的对组合累计收益的贡献，小数表示。 |
| symbol | string | 是 | 标的代码。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/portfolios/FP01" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "meta": {
      "code": "FP01",
      "description": "",
      "status": "实盘",
      "base_market": "stock",
      "base_freq": "60min",
      "price_field": "close",
      "rebalance_method": "equal_weight",
      "lookback_days": 120,
      "config_hash": "",
      "generated_at": "2026-08-21T16:11:50.164355",
      "fee_bp": 0.0,
      "digits": 2,
      "symbol_count": 0,
      "sdt": "",
      "edt": ""
    },
    "strategies": [
      { "strategy_id": "STS_60M_1I7G6TS4", "weight": 1.0 }
    ],
    "rebalance_dates": ["2025-01-13", "2026-01-01"],
    "latest_weights": [],
    "latest_weights_at": "",
    "metrics": null,
    "compare_metrics": {},
    "compare": { "dates": [], "series": {} },
    "curves": {},
    "normalized_20": {},
    "nav": { "dates": [], "nav": [], "drawdown": [], "cum_return": [] },
    "monthly": [],
    "yearly": {},
    "symbol_returns": [],
    "drawdowns": [],
    "verdict": { "history": null, "recent": null },
    "positions": { "dates": [], "symbols": [], "weights": {} },
    "has_report": true,
    "has_performance": false
  }
}
```
