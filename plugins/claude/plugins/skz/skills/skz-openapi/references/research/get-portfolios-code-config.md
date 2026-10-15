---
title: 读取组合 TOML 配置内容
description: 胜可知开放平台 GET /research/portfolios/{code}/config：读取组合 TOML 配置内容。
source: https://docs.shengkezhi.com/api/research/get-portfolios-code-config
---
# 读取组合 TOML 配置内容

**`GET /research/portfolios/{code}/config`** — 读取组合 TOML 配置内容。

返回当前账户组合已保存的 `portfolio.toml` 原文，保留注释、空白、换行和全部配置字段。每次直接读取最新落盘文件，响应带 `Cache-Control: private, no-store`；不依赖回测产物或任务状态。配置内容不做 TOML 语法校验，也不从构建产物补全字段。

响应仍为 JSON 信封，TOML 字符串位于 `data.toml`，并非直接返回 TOML 文件。CLI 使用 `skz portfolio config <code>`，输出解包后的 `{"toml":"..."}`；提取原文可用 `skz portfolio config <code> | jq -rj .toml`。测试环境 Base URL 为 `https://tapi.shengkezhi.cn/open/v1`。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 业务状态

组合编号无效或路径越界返回 HTTP `400`；配置文件不存在返回 HTTP `404`、业务码 `40400`；读取失败返回 HTTP `500`，错误信息已脱敏。该接口只读取配置，不触发组合计算或修改状态。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 组合编号 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `toml` | string | 是 | 已保存的 portfolio.toml 原文，保留注释、空白和所有配置字段。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/portfolios/PFO_FUTURE_2CEFD909/config" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "toml": "schema_version = \"1.0\"\nportfolio_code = \"PFO_FUTURE_2CEFD909\"\ndescription = \"第28次探索：FTS全量策略池 + skfolio MAXIMIZE_RETURN + 回撤口径风险归一化。提供逐策略 max_weights/min_weights 1D 数组、Use radius=0.15 target=0.15，规避标量 0D 数组与 nan 报错。\"\nstatus = \"暂停\"\ncandidate_strategies = [\"FTS_1D_1C6O0SOF\", \"FTS_1D_80DB62JF\", \"FTS_1D_VWDFPMXS\", \"FTS_1D_WVBBRGPN\", \"FTS_1D_XOQQWA4F\", \"FTS_60M_BL8JPD7Q\", \"FTS_60M_GCOAIIN9\", \"FTS_60M_J46XUU16\"]\nrebalance_dates = []\nrebalance_method = \"skfolio\"\nlookback_days = 120\nskip_existing = false\nrebalance_date_rules = [\"每月最后一天\"]\nt_plus_one = false\nlong_only = true\n\n[strategy_selector]\n\n[symbol_selector]\n\n[rebalance_params]\nfallback_method = \"inverse_vol\"\nlookback_days = 252\nmin_observations = 60\noptimizer = \"MeanRisk\"\n\n[rebalance_params.risk_normalization]\nmax_scale = 2\nmethod = \"drawdown\"\nmin_observations = 60\ntarget_drawdown = 0.15\ntrading_days_per_year = 252\n\n[rebalance_params.optimizer_params]\nmax_weights = [0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2]\nmin_weights = [0, 0, 0, 0, 0, 0, 0, 0]\nobjective_function = \"MAXIMIZE_RETURN\"\nrisk_measure = \"VARIANCE\"\nsolver = \"SCS\"\n"
  }
}```
