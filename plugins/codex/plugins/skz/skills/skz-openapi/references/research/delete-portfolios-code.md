---
title: 删除策略组合
description: 胜可知开放平台 DELETE /research/portfolios/{code}：删除策略组合。
source: https://docs.shengkezhi.com/api/research/delete-portfolios-code
---
# 删除策略组合

**`DELETE /research/portfolios/{code}`** — 删除策略组合。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 组合编号 |

## 响应 data

该对象没有固定字段。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/portfolios/STRAT_MOMENTUM_001" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 40000,
  "data": null,
  "msg": "invalid request"
}```
