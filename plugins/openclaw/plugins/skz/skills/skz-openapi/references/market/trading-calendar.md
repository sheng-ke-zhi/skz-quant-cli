---
title: 交易日历
endpoint: GET /market/trading-calendar
source: https://docs.shengkezhi.com/api/market/trading-calendar
---

# 交易日历

`GET /market/trading-calendar` — 交易日历，对齐 Tushare trade_cal，含休市日。按 cal_date 升序。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

Query 参数：

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| exchange | 是 | 交易所，如 SSE（上交所）/ SZSE（深交所） |
| start | 否 | 起始日 yyyy-MM-dd（含端点） |
| end | 否 | 结束日 yyyy-MM-dd（含端点） |
| onlyOpen | 否 | 默认 false，含休市日；true 时返回交易日 |

## 响应

```json
[{ exchange, calDate, isOpen, pretradeDate }]
```

- `isOpen`：true=交易日，false=休市
- `pretradeDate`：上一交易日

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/market/trading-calendar?exchange=SSE&start=2026-01-01&end=2026-01-06" \
  -H "Authorization: Bearer sk_xxx"
```

```json
[
  {"exchange":"SSE","calDate":"2026-01-01","isOpen":false,"pretradeDate":"2025-12-31"},
  {"exchange":"SSE","calDate":"2026-01-02","isOpen":false,"pretradeDate":"2025-12-31"},
  {"exchange":"SSE","calDate":"2026-01-05","isOpen":true,"pretradeDate":"2025-12-31"},
  {"exchange":"SSE","calDate":"2026-01-06","isOpen":true,"pretradeDate":"2026-01-05"}
]
```
