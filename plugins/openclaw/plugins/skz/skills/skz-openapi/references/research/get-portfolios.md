---
title: 组合列表
description: 胜可知开放平台 GET /research/portfolios：组合列表。
source: https://docs.shengkezhi.com/api/research/get-portfolios
---

# 组合列表

**`GET /research/portfolios`** — 组合列表。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `items` | `PortfolioListItem`[] | 是 | 按组合编号升序排列的组合列表。 |

### PortfolioListItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | `string` | 是 | 组合编号，也是组合目录名；仅含字母、数字、下划线或连字符。 |
| `description` | `string` | 是 | 用户填写的组合用途或构建思路；未填写时为空字符串。 |
| `status` | `string` | 是 | 组合业务状态；已生成组合通常为 `实盘`，异步占位项为 `生成中` 或 `生成失败`。 |
| `base_market` | `string` | 是 | 基础市场代码，例如 `A股`。 |
| `base_freq` | `string` | 是 | 基础数据频率，例如 `1d`。 |
| `symbol_count` | `integer` | 是 | 最新回测覆盖的标的数量；异步任务尚未完成时为 0。 |
| `strategy_count` | `integer` | 是 | 参与组合优化的候选策略数量。 |
| `sdt` | `string` | 是 | 回测起始日期，格式 `YYYY-MM-DD`；尚未生成回测时为空字符串。 |
| `edt` | `string` | 是 | 回测结束日期，格式 `YYYY-MM-DD`；尚未生成回测时为空字符串。 |
| `annual_return` | `number` \| null | 否 | 年化收益率，小数表示；缺少回测产物或生成失败时为 null。 |
| `sharpe` | `number` \| null | 否 | 年化夏普比率；缺少回测产物或生成失败时为 null。 |
| `max_drawdown` | `number` \| null | 否 | 最大回撤，小数表示且通常为负数；缺少回测产物或生成失败时为 null。 |
| `abs_return` | `number` \| null | 否 | 全区间绝对收益率，小数表示；缺少回测产物或生成失败时为 null。 |
| `has_performance` | `boolean` | 是 | 是否已落盘可展示的绩效 MsgPack。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/portfolios" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "code": "FP01",
        "description": "",
        "status": "实盘",
        "base_market": "stock",
        "base_freq": "60min",
        "symbol_count": 0,
        "strategy_count": 1,
        "sdt": "",
        "edt": "",
        "annual_return": null,
        "sharpe": null,
        "max_drawdown": null,
        "abs_return": null,
        "has_performance": false
      },
      {
        "code": "ST01",
        "description": "",
        "status": "实盘",
        "base_market": "stock",
        "base_freq": "60min",
        "symbol_count": 0,
        "strategy_count": 4,
        "sdt": "",
        "edt": "",
        "annual_return": null,
        "sharpe": null,
        "max_drawdown": null,
        "abs_return": null,
        "has_performance": false
      },
      {
        "code": "STOCK01",
        "description": "股票",
        "status": "实盘",
        "base_market": "stock",
        "base_freq": "60min",
        "symbol_count": 0,
        "strategy_count": 14,
        "sdt": "",
        "edt": "",
        "annual_return": null,
        "sharpe": null,
        "max_drawdown": null,
        "abs_return": null,
        "has_performance": false
      }
    ]
  }
}```
