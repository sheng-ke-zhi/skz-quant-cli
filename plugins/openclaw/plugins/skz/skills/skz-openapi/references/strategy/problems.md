---
title: 创建研究问题
description: 胜可知开放平台 POST /strategy/problems：创建研究问题并返回问题编码。
source: https://docs.shengkezhi.com/api/strategy/problems
---

# 创建研究问题

**`POST /strategy/problems`** — 创建研究问题，返回问题编码。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

研究问题常见字段如下：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | string | 否 | 问题名称 |
| `type` | string | 否 | 问题类型，如 `time_series` / `cross_section` |
| `market` | string | 否 | 数据集，如 `stock` / `etf` / `future` |
| `symbols` | array[string] | 否 | 标的代码列表 |
| `frequency` | string | 否 | 频率，如 `15m` / `60m` / `120m` / `240m` / `day` |
| `start_date` | string | 否 | 开始日期，格式 `yyyy-MM-dd` |
| `end_date` | string | 否 | 结束日期，格式 `yyyy-MM-dd` |

> **字段说明（说明）**
> 具体字段与校验规则以接口返回为准。若字段不满足要求，将返回 `400` 与对应错误信息。

## 响应

响应通常为 `{ code, msg, data }`。返回码与具体字段以实际返回为准。

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/problems" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "银行股短期动量研究",
    "type": "time_series",
    "market": "stock",
    "symbols": ["000001.SZ", "000002.SZ"],
    "frequency": "60m",
    "start_date": "2024-01-01",
    "end_date": "2025-12-31"
  }'
```

```json
{"code":0,"msg":"success","data":{"problemCode":"PRB_20260724_001"}}
```

