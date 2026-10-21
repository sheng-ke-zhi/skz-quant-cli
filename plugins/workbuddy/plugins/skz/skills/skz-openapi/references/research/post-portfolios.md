---
title: 创建组合
description: 胜可知开放平台 POST /research/portfolios：创建组合。
source: https://docs.shengkezhi.com/api/research/post-portfolios
---
# 创建组合

**`POST /research/portfolios`** — 创建组合。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `base_freq` | string | 否 | 基础数据频率，缺省 `1d`。 |
| `base_market` | string | 否 | 兼容旧调用方的市场提示；计算上下文由候选策略派生。 |
| `candidate_strategies` | string[] | 是 | 候选实盘策略编号，至少一项；云函数从中优化组合层权重。 |
| `description` | string | 否 | 组合用途或构建思路；缺省为空字符串。 |
| `portfolio_code` | string | 是 | 新组合编号，1～160 个字符，仅允许字母、数字、下划线和连字符。 |
| `price_field` | string | 否 | 回测价格字段，缺省 `close`。 |
| `rebalance_dates` | string[] | 否 | YYYY-MM-DD 日期；与 rebalance_date_rules 至少一种，由 FOX 合并排序去重。 |
| `rebalance_date_rules` | string[] | 否 | 支持月末等规则独立使用，不要求显式日期。 |
| `symbol_selector` / `strategy_selector` | object | 否 | 内置选择器配置，例如 allowed_symbols 和 symbols。 |
| `lookback_days` | integer | 否 | 外层配置保存，不替代优化训练窗口。 |
| `skip_existing` | boolean | 否 | 保留 FOX 跳过已有日期产物的设置。 |
| `rebalance_params` | object | 否 | 训练窗口、优化器参数、回退和风险归一；当前 skfolio 入口支持 MeanRisk。 |
| `status` | string | 否 | 新建只允许暂停，缺省暂停。 |
| `rebalance_method` | string | 否 | 再平衡方法，缺省 `equal_weight`。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `portfolio_code` | string | 是 | 已受理的组合编号；可用它轮询 `GET /api/portfolios`。 |
| `run_id` | string | 是 | 异步执行编号，相同编号已有 pending 时复用。 |
| `status` | string | 是 | 固定 `pending`：已受理，正在后台生成。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/portfolios" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"base_freq":"60min","base_market":"future","candidate_strategies":["FTS_60M_BL8JPD7Q","FTS_1D_80DB62JF","FTS_60M_J46XUU16","FTS_1D_XOQQWA4F"],"description":"开放平台组合示例","portfolio_code":"DOCS_E2E_DEMO","price_field":"close","rebalance_dates":["2025-01-01","2027-01-01"],"rebalance_method":"equal_weight"}'
```

## 完整配置导入

`skz portfolio create < portfolio.toml` 接受完整 TOML，也兼容旧 JSON stdin。转换只改变格式，保留高级字段、空对象、false 和调仓规则；免费预检、费用确认和写请求不重试的规则继续生效。

实际市场、频率和价格由候选策略及 FOX 派生，base_market/base_freq/price_field 只是兼容提示。外层 lookback_days=120 原值保存；rebalance_params.lookback_days=252 才参与 skfolio 训练窗口。

当前支持 MeanRisk；optimizer_params 支持标量 min_weights/max_weights、objective_function（MINIMIZE_RISK/MAXIMIZE_RETURN/MAXIMIZE_UTILITY/MAXIMIZE_RATIO）、risk_measure（VARIANCE/SEMI_VARIANCE/STANDARD_DEVIATION/SEMI_DEVIATION/CVAR）、solver（CLARABEL/SCS/ECOS/OSQP，须由计算环境安装）。risk_normalization 支持 volatility/drawdown、target_volatility、target_drawdown、trading_days_per_year、min_observations、max_scale。未知/不支持字段返回 422，JSON 类型错误沿用请求解析错误；search、自定义选择器代码、其它优化器和 position policy 当前明确拒绝。

15% 上限约束归一优化权重；风险归一的缩放杠杆可能使最终名义权重超过该值。以下 18 个策略编号是占位符，必须替换为本账号的实盘策略。

```toml
portfolio_code = "BONDS_MONTH_END"
status = "暂停"
candidate_strategies = ["LIVE_01", "LIVE_02", "LIVE_03", "LIVE_04", "LIVE_05", "LIVE_06", "LIVE_07", "LIVE_08", "LIVE_09", "LIVE_10", "LIVE_11", "LIVE_12", "LIVE_13", "LIVE_14", "LIVE_15", "LIVE_16", "LIVE_17", "LIVE_18"]
rebalance_dates = []
rebalance_date_rules = ["每月最后一天"]
rebalance_method = "skfolio"
lookback_days = 120
skip_existing = false
[strategy_selector]
[symbol_selector]
name = "allowed_symbols"
symbols = ["T999.CFX", "TL999.CFX"]
[rebalance_params]
fallback_method = "inverse_vol"
lookback_days = 252
min_observations = 120
optimizer = "MeanRisk"
[rebalance_params.risk_normalization]
max_scale = 2
method = "volatility"
min_observations = 60
target_volatility = 0.2
trading_days_per_year = 252
[rebalance_params.optimizer_params]
max_weights = 0.15
objective_function = "MINIMIZE_RISK"
risk_measure = "SEMI_VARIANCE"
solver = "CLARABEL"
```

```json
{
  "code": 0,
  "msg": "已受理，正在后台生成",
  "data": {
    "portfolio_code": "DOCS_E2E_DEMO",
    "status": "pending"
  }
}
```
