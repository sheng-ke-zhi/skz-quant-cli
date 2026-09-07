---
title: 创建组合
endpoint: POST /research/portfolios
source: https://docs.shengkezhi.com/api/research/post-portfolios
---

# 创建组合

`POST /research/portfolios` — 创建组合。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/portfolios`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| base_freq | string | 否 | 基础数据频率，缺省 1d。 |
| base_market | string | 是 | 基础市场代码；必须是服务端支持的市场之一。 |
| candidate_strategies | string[] | 是 | 候选实盘策略编号，至少一项；云函数从中优化组合层权重。 |
| description | string | 否 | 组合用途或构建思路；缺省为空字符串。 |
| portfolio_code | string | 是 | 新组合编号，1～160 个字符，仅允许字母、数字、下划线和连字符。 |
| price_field | string | 否 | 回测价格字段，缺省 close。 |
| rebalance_dates | string[] | 是 | 再平衡日期，至少一项，元素格式 YYYY-MM-DD。 |
| rebalance_method | string | 否 | 再平衡方法，缺省 equal_weight。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| portfolio_code | string | 是 | 已受理的组合编号；可用它轮询 GET /api/portfolios。 |
| status | string | 是 | 固定 pending：已受理，正在后台生成。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/portfolios" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "base_freq": "1d",
    "base_market": "cn_stock",
    "candidate_strategies": ["TS_1D_A96ACBB3"],
    "description": "示例Description",
    "portfolio_code": "STS_BJ60MIN_LEADERS",
    "price_field": "close",
    "rebalance_dates": ["2026-07-01"],
    "rebalance_method": "equal_weight"
  }'
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "portfolio_code": "STS_BJ60MIN_LEADERS",
    "status": "pending"
  }
}
```
