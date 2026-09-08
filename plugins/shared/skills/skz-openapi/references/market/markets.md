---
title: 市场列表
description: 胜可知开放平台 GET /market/markets：返回数据集去重取值与各自标的数量，按数量倒序。
source: https://docs.shengkezhi.com/api/market/markets
---

# 市场列表

**`GET /market/markets`** — 返回 `market` 数据集去重取值与各自标的数量，按数量倒序。

> **数据范围（说明）**
> 接口返回国内金融市场数据集，覆盖平台已开放的股票、ETF、期货、指数等品类。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

无。

## 响应

`[{ market: string, count: number }]`

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/market/markets" -H "Authorization: Bearer sk_xxx"
```

```json
[
  {"market":"stock","count":5464},
  {"market":"etf","count":1532},
  {"market":"future","count":77},
  {"market":"index","count":38}
]
```

