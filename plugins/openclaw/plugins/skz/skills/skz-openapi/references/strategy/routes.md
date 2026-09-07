---
title: 创建因子路线
endpoint: POST /strategy/routes
source: https://docs.shengkezhi.com/api/strategy/routes
---

# 创建因子路线

`POST /strategy/routes` — 创建一条因子路线，返回路线编码。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| name | string | 是 | 路线名称，一句话概括研究主题 |
| key_inspect | string | 是 | 核心思路，说明在什么条件下预期出现什么变化 |
| economic_logic | string | 是 | 经济学或行为金融学依据 |
| why_effective | string | 是 | 是对未来具有预测力的原因 |
| market_mechanism | string | 是 | 主导市场机制，如 趋势跟踪、行为偏差、流动性溢价 等 |
| failure_scenarios | array[string] | 是 | 路线失效的场景，至少填写一条 |
| tags | array[string] | 否 | 分类标签 |

## 响应

```json
{ routeCode: string }
```

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/routes" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "放量突破后的短期动量",
    "key_inspect": "标的在放量突破关键价位后，未来 5 个交易日内是否延续上涨动量",
    "economic_logic": "突破行为往往伴随资金共识形成，短期 momentum 来自知情交易者与趋势跟随者的共同作用",
    "why_effective": "放量突破释放了新的信息，市场在短期内对该信息存在反应不足",
    "market_mechanism": "趋势跟踪",
    "failure_scenarios": [
      "市场整体处于持续下跌通道，个股突破为假突破",
      "突破伴随利好兑现，资金反向出货"
    ],
    "tags": ["动量", "量价"]
  }'
```

```json
{"routeCode":"RT_20260724_001"}
```
