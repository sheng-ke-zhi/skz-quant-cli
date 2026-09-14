---
title: 创建投研因子路线
description: 胜可知开放平台 POST /research/factor-routes：创建投研因子路线。
source: https://docs.shengkezhi.com/api/research/post-factor-routes
---

# 创建投研因子路线

**`POST /research/factor-routes`** — 创建投研因子路线。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `compute_engine` | string \| null | 否 | 计算引擎，因子 AST 的解释执行引擎；留空或空串时回退默认引擎 `TimeSeriesAstEngine`。 |
| `economic_logic` | string | 是 | 经济逻辑，路线赖以生效的资金行为或市场结构解释；参与 route_code 哈希。 |
| `failure_scenarios` | string[] | 否 | 失效场景列表，列举该路线可能失灵的市场情形；落库时存为 JSON 字符串数组。 |
| `key_inspect` | string | 是 | 核心洞察，该路线量化捕捉的关键信号或市场规律；参与 route_code 哈希。 |
| `market_mechanism` | string | 是 | 市场机制，收益来源归类（如「错误定价」或「风险溢价」）；参与 route_code 哈希。 |
| `name` | string | 是 | 路线名称，前端筛选下拉直接展示；参与 route_code 内容哈希的 5 个必填字段之一。 |
| `tags` | string[] | 否 | 路线标签列表（如「时序」「量价」「威科夫」）；落库时存为 JSON 字符串数组。 |
| `why_effective` | string | 是 | 有效性依据，说明该逻辑为何能形成可持续的超额收益来源；参与 route_code 哈希。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | string | 是 | 与 `route_code` 同值的别名，兼容前端有的组件读 `code`、有的读 `route_code` 两种取字段习惯。 |
| `route_code` | string | 是 | 新建研究路线的编码，后端按路线内容哈希幂等生成，重复提交同一路线返回同一编码。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/factor-routes" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"compute_engine":"TimeSeriesAstEngine","economic_logic":"示例逻辑：量能趋势确认后跟随，资金持续流入推动行情延续。","failure_scenarios":["示例失效场景：横盘震荡中信号频繁假突破"],"key_inspect":"示例洞察：5日均量线上穿20日均量线且价格站上60日均线。","market_mechanism":"趋势跟踪","name":"示例路线：量价共振趋势启动","tags":["示例标签：量价"],"why_effective":"示例依据：量能趋势的持续性比单日放量更具预测力。"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "6f2e8d1a4c9b",
    "route_code": "6f2e8d1a4c9b"
  }
}```
