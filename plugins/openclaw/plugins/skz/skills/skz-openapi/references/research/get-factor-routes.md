---
title: 因子路线列表
endpoint: GET /research/factor-routes
source: https://docs.shengkezhi.com/api/research/get-factor-routes
---

# 因子路线列表

`GET /research/factor-routes` — 因子路线列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/factor-routes`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | FactorRouteView[] | 是 | 路线视图列表。 |
| total | integer | 是 | 路线总数。 |

### FactorRouteView 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code | string | 是 | 路线编码，12 位十六进制，全流程溯源主键。 |
| compute_engine | string | 是 | 计算引擎全称，如 TimeSeriesAstEngine。 |
| create_time | string | 是 | 路线创建时间。 |
| creator | string | 是 | 创建人（缺失序列化为空串）。 |
| description | string | 是 | 路线简述。 |
| economic_logic | string | 是 | 经济学逻辑：判断路线逻辑是否成立的核心依据。 |
| failure_scenarios | string[] | 是 | 失效场景列表：该路线可能失灵的市场情形。 |
| key_inspect | string | 是 | 核心洞察：该路线要捕捉的关键量价现象。 |
| market_mechanism | string | 是 | 市场机制：超额收益来源类型，如「错误定价」。 |
| name | string | 是 | 路线名称，用于筛选下拉展示。 |
| tags | string[] | 是 | 路线标签，如时序、量价、威科夫等。 |
| why_effective | string | 是 | 有效性依据：说明该类因子为何能带来超额收益。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/factor-routes" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "code": "90ec76bab887",
        "compute_engine": "TimeSeriesAstEngine",
        "create_time": "2026-05-26 17:41:18.151748",
        "creator": "SKZ_ExampleModel",
        "description": "5日均量线上穿20日均量线且价格站上60日均线，量价双趋势共振确认主升浪起点。",
        "economic_logic": "均量线金叉反映一段时期内市场参与度的持续提升而非单日异动，叠加价格趋势确立，表明资金正系统性地流入，趋势持续性更强。",
        "failure_scenarios": [
          "横盘震荡市中均量线频繁交叉产生大量假信号",
          "放量末期才出现金叉，此时趋势已接近尾声"
        ],
        "key_inspect": "5日均量线上穿20日均量线且价格站上60日均线，量价双趋势共振确认主升浪起点。",
        "market_mechanism": "趋势跟踪",
        "name": "量价共振趋势因子",
        "tags": ["量价因子", "趋势确认"],
        "why_effective": "量能趋势的持续性比单日放量更具预测力，机构资金的建仓和拉升通常跨越多个交易日，均量线金叉能过滤噪音并捕捉资金持续流入的拐点。"
      },
      {
        "code": "2b15d20b6324",
        "compute_engine": "TimeSeriesAstEngine",
        "create_time": "2026-05-26 20:53:45.117864",
        "creator": "SKZ_ExampleModel",
        "description": "量化威科夫六大经典信号（Spring弹簧/SOS跳跃小溪/LPS最后支撑/EVR量价背离/UTAD派发诱多/SOW弱势信号）为二值AST因子",
        "economic_logic": "Composite Man 主力资金在吸筹-拉升-派发-下跌四阶段留下的量价足迹，通过特定的价量组合识别主力真实意图",
        "failure_scenarios": ["震荡市中信号频繁假突破", "极端行情下成交量失真"],
        "key_inspect": "量化威科夫六大经典信号（Spring弹簧/SOS跳跃小溪/LPS最后支撑/EVR量价背离/UTAD派发诱多/SOW弱势信号）为二值AST因子",
        "market_mechanism": "错误定价",
        "name": "威科夫时序扳机信号因子",
        "tags": ["时序", "量价"],
        "why_effective": "主力资金的大额交易难以完全伪装，放量突破、缩量回踩等特定量价组合反映了知情交易者的真实行为，市场对这些信号的识别存在滞后性"
      },
      {
        "code": "d03d29a24f2d",
        "compute_engine": "TimeSeriesAstEngine",
        "create_time": "2026-05-18 20:59:47.230563",
        "creator": "SKZ_ExampleModel",
        "description": "多周期价格动量的强度与持续性预测后续收益",
        "economic_logic": "价格动量反映市场参与者的趋势追踪行为，强动量品种往往受到资金持续流入，短期内维持趋势延续。",
        "failure_scenarios": [
          "市场剧烈反转时动量信号失效（如重大黑天鹅事件）",
          "流动性极低时动量追踪成本过高导致信号衰减"
        ],
        "key_inspect": "多周期价格动量的强度与持续性预测后续收益",
        "market_mechanism": "行为偏差",
        "name": "动量因子路线",
        "tags": ["动量", "时序"],
        "why_effective": "行为金融学证据显示投资者存在反应不足与羊群效应，推动动量因子在中短期内保持显著的预测能力。"
      }
    ],
    "total": 51
  }
}
```

## 与「已采用路线列表」的区别

本接口返回因子库中的完整路线视图。只需要路线编码和名称时，可使用策略接口的[已采用路线列表](https://docs.shengkezhi.com/api/strategy/routes-adopted)（`../strategy/routes-adopted`）。
