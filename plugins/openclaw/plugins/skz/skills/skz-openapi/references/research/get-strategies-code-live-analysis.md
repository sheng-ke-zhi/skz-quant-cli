---
title: 实盘策略归因分析
endpoint: GET /research/strategies/{code}/live/analysis
source: https://docs.shengkezhi.com/api/research/get-strategies-code-live-analysis
---

# 实盘策略归因分析

`GET /research/strategies/{code}/live/analysis` — 实盘策略归因分析。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/strategies/{code}/live/analysis`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| as_of | string | 是 | — |
| live_cutoff | string | 是 | — |
| persisted | LivePersistedAnalysis | 是 | — |
| rebuilt | LiveRebuiltAnalysis | 是 | — |
| source | string | 是 | — |

### LivePersistedAnalysis 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| dates | string[] | 是 | — |
| total_returns | number[] | 是 | — |
| nav | number[] | 是 | — |
| drawdowns | object[] | 是 | — |
| symbol_return_contributions | SymbolReturnContribution[] | 是 | — |

### LiveRebuiltAnalysis 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| status | string | 是 | — |
| source | string | 是 | 数据来源：persisted = 直读 skz 落盘的 backtest_result.msgpack（零计算）；rebuilt = 后端用 weights + 行情重跑 WeightBacktest。两者区间口径不同，且 persisted 下交易明细没有开仓/平仓价格，前端须据此决定展示。 |
| error_code | integer \| null | 否 | — |
| sample_start | string | 是 | 本次分析实际覆盖的样本起止。persisted 取落盘自带的 start_date/end_date（由 skz 跑批窗口决定，与 live_cutoff 不是同一段）；rebuilt 取重建曲线首尾。 |
| sample_end | string | 是 | — |
| dates | string[] | 是 | — |
| curves | object | 是 | — |
| normalized_20 | object | 是 | — |
| compare_metrics | object | 是 | — |
| verdict | object | 是 | — |
| trades | LiveTradeItem[] | 是 | — |

### SymbolReturnContribution 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| symbol | string | 是 | — |
| return | number | 是 | — |

### LiveTradeItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| kline_key | string | 是 | — |
| symbol | string | 是 | — |
| 交易方向 | string | 是 | — |
| 盈亏 | number | 是 | — |
| 开仓时间 | string | 是 | — |
| 平仓时间 | string | 是 | — |
| 开仓价格 | number \| null | 否 | 开仓/平仓价格。仅 source="rebuilt" 时有值；直读落盘时为 null——落盘的 key_trades 不含价格列（那两列只在 wbt Rust 侧 key_trades_df 里）。用 null 而非 0.0，避免前端把 0 当成真实成交价渲染。 |
| 平仓价格 | number \| null | 否 | — |
| 持仓K线数 | integer | 是 | — |
| 持仓数量 | integer | 是 | — |
| year | string | 是 | — |
| kind | string | 是 | — |
| inherited | boolean | 是 | — |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/strategies/STS_60M_1I7G6TS4/live/analysis" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "source": "live",
    "live_cutoff": "2023-01-01",
    "as_of": "2026-08-21 00:00:00",
    "persisted": {
      "dates": ["2025-01-17 00:00:00", "2025-01-20 00:00:00"],
      "total_returns": [0.0, 0.0],
      "nav": [1.0, 1.0],
      "drawdowns": [
        {
          "净值回撤": -0.053701766650158564,
          "回撤修复": "2026-01-29",
          "回撤天数": 240,
          "回撤开始": "2025-02-25",
          "回撤结束": "2025-10-23",
          "恢复天数": 98.0,
          "新高间隔": 338.0
        },
        {
          "净值回撤": -0.04480308005386943,
          "回撤修复": null,
          "回撤天数": 45,
          "回撤开始": "2026-05-26",
          "回撤结束": "2026-07-10",
          "恢复天数": null,
          "新高间隔": null
        }
      ],
      "symbol_return_contributions": [
        { "symbol": "000858.SZ", "return": 0.008137571592867508 },
        { "symbol": "600519.SH", "return": 0.12288198170679789 }
      ]
    },
    "rebuilt": {
      "status": "ready",
      "source": "persisted",
      "error_code": null,
      "sample_start": "2025-01-17",
      "sample_end": "2026-08-21",
      "dates": ["2025-01-17 00:00:00", "2025-01-20 00:00:00"],
      "curves": {
        "基准": {
          "cum": [-0.0004946855654873739, 0.013467480661830011],
          "daily": [-0.0004946855654873739, 0.013962166227317385],
          "drawdown": [0.0, 0.0]
        },
        "多头": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] },
        "多空": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] },
        "空头": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] },
        "超额": {
          "cum": [0.0004946855654873739, -0.013467480661830011],
          "daily": [0.0004946855654873739, -0.013962166227317385],
          "drawdown": [0.0, -0.013962166227317385]
        }
      },
      "normalized_20": {
        "基准": {
          "cum": [-0.0004922397477855171, 0.013400894925555969],
          "daily": [-0.0004922397477855171, 0.013893134673341486],
          "drawdown": [0.0, 0.0]
        },
        "多头": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] },
        "多空": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] },
        "空头": { "cum": [0.0, 0.0], "daily": [0.0, 0.0], "drawdown": [0.0, 0.0] }
      },
      "compare_metrics": {
        "基准": {
          "下行波动率": 0.114,
          "卡玛": -0.3928,
          "回归年度回报率": -0.1868,
          "回撤风险": 2.1348,
          "夏普": -0.8383,
          "年化": -0.1683,
          "年化波动率": 0.2007,
          "开始日期": "2025-01-17",
          "新高占比": 0.0313,
          "新高间隔": 348.0,
          "日盈亏比": 1.063,
          "日胜率": 0.4465,
          "日赢面": -0.0789,
          "最大回撤": 0.4285,
          "盈亏平衡点": 1.0,
          "结束日期": "2026-08-21",
          "绝对收益": -0.2558,
          "长度调整平均最大回撤": 0.2651,
          "非零覆盖": 1.0
        },
        "多头": {
          "下行波动率": 0.0504,
          "交易次数": 6107,
          "交易胜率": 0.3995,
          "单笔收益": -0.37,
          "单笔盈亏比": 1.494,
          "卡玛比率": -0.0659,
          "周胜率": 0.3253,
          "品种数量": 2,
          "夏普比率": -0.0807,
          "多头占比": 0.452,
          "季胜率": 0.5714,
          "年化交易次数": 4018.18,
          "年化收益": -0.0052,
          "年化波动率": 0.0639,
          "年胜率": 0.5,
          "持仓K线数": 9.43,
          "新高占比": 0.0339,
          "新高间隔": 113.0,
          "日胜率": 0.5718,
          "最大回撤": 0.0789,
          "月胜率": 0.5,
          "空头占比": 0.0,
          "绝对收益": -0.0078
        },
        "多空": {
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
        "空头": {
          "下行波动率": 0.0444,
          "交易次数": 6665,
          "交易胜率": 0.487,
          "单笔收益": 15.43,
          "单笔盈亏比": 1.3528,
          "卡玛比率": 0.9173,
          "周胜率": 0.4578,
          "品种数量": 2,
          "夏普比率": 0.9125,
          "多头占比": 0.0,
          "季胜率": 0.7143,
          "年化交易次数": 4385.33,
          "年化收益": 0.0423,
          "年化波动率": 0.0464,
          "年胜率": 0.5,
          "持仓K线数": 9.91,
          "新高占比": 0.0653,
          "新高间隔": 246.0,
          "日胜率": 0.6736,
          "最大回撤": 0.0461,
          "月胜率": 0.55,
          "空头占比": 0.537,
          "绝对收益": 0.0643
        },
        "超额": {
          "下行波动率": 0.1689,
          "卡玛": 1.3007,
          "回归年度回报率": 0.2424,
          "回撤风险": 0.7845,
          "夏普": 1.0208,
          "年化": 0.2054,
          "年化波动率": 0.2013,
          "开始日期": "2025-01-17",
          "新高占比": 0.094,
          "新高间隔": 132.0,
          "日盈亏比": 0.9517,
          "日胜率": 0.5587,
          "日赢面": 0.0904,
          "最大回撤": 0.1579,
          "盈亏平衡点": 0.9739,
          "结束日期": "2026-08-21",
          "绝对收益": 0.3123,
          "长度调整平均最大回撤": 0.096,
          "非零覆盖": 1.0
        }
      },
      "verdict": {
        "history": {
          "alpha_degenerate": false,
          "complete_year_count": 1,
          "cond_history_dd_passed": true,
          "cond_history_sharpe_passed": true,
          "cond_yearly_passed": true,
          "history_alpha_max_drawdown": 0.12768676093992037,
          "history_alpha_sharpe": 0.9450535802775095,
          "is_good": true,
          "mode": "history",
          "reason": "",
          "yearly_metrics": [
            {
              "abs_return": -0.01830483381487667,
              "alpha_max_drawdown": 0.12768676093992037,
              "alpha_return": 0.0722260079749714,
              "days": 232,
              "is_complete_year": true,
              "year": 2025,
              "year_passed": true
            },
            {
              "abs_return": 0.07480383457195736,
              "alpha_max_drawdown": 0.0837726910725527,
              "alpha_return": 0.1580795082598037,
              "days": 151,
              "is_complete_year": false,
              "year": 2026,
              "year_passed": false
            }
          ]
        },
        "recent": null
      },
      "trades": [
        {
          "kline_key": "000858.SZ|2025-02-11 11:30:00|2025-02-13 10:30:00",
          "symbol": "000858.SZ",
          "交易方向": "空头",
          "盈亏": -0.04165,
          "开仓时间": "2025-02-11 11:30:00",
          "平仓时间": "2025-02-13 10:30:00",
          "开仓价格": null,
          "平仓价格": null,
          "持仓K线数": 8,
          "持仓数量": 3,
          "year": "2025",
          "kind": "loss",
          "inherited": false
        },
        {
          "kline_key": "000858.SZ|2025-03-12 15:00:00|2025-03-14 11:30:00",
          "symbol": "000858.SZ",
          "交易方向": "空头",
          "盈亏": -0.053922000000000005,
          "开仓时间": "2025-03-12 15:00:00",
          "平仓时间": "2025-03-14 11:30:00",
          "开仓价格": null,
          "平仓价格": null,
          "持仓K线数": 7,
          "持仓数量": 6,
          "year": "2025",
          "kind": "loss",
          "inherited": false
        }
      ]
    }
  }
}
```
