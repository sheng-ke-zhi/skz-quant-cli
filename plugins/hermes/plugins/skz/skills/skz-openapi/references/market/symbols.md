---
title: 标的分页查询
description: 胜可知开放平台 GET /market/symbols：按数据集 + 关键字过滤并分页列出标的。
source: https://docs.shengkezhi.com/api/market/symbols
---

# 标的分页查询

**`GET /market/symbols`** — 按数据集 + 关键字过滤 + 分页列出标的。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| Query 参数 | 必填 | 说明 |
| --- | --- | --- |
| `market` | 否 | 精确匹配数据集，如 `stock` / `etf` / `future` / `index` |
| `keyword` | 否 | 对 name / symbol 子串匹配，大小写无关，`% _ \` 会被转义为字面量 |
| `page` | 否 | 页码，默认 `1` |
| `size` | 否 | 每页条数，默认 `20`，上限 `200` |

## 响应

`{ page, size, total, items: [{ id, name, symbol, market, updateAt }] }`

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/market/symbols?market=stock&keyword=平安&page=1&size=3" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{"page":1,"size":3,"total":3,"items":[
  {"id":1,"name":"平安银行","symbol":"000001.SZ","market":"stock","updateAt":"2026-07-08T12:31:22.989173+00:00"},
  {"id":559,"name":"平安电工","symbol":"001359.SZ","market":"stock","updateAt":"2026-07-08T12:31:22.989173+00:00"},
  {"id":3980,"name":"中国平安","symbol":"601318.SH","market":"stock","updateAt":"2026-07-08T12:31:22.989173+00:00"}
]}
```

