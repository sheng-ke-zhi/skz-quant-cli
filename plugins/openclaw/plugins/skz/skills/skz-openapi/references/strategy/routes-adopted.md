---
title: 已采用路线列表
endpoint: GET /strategy/routes/adopted
source: https://docs.shengkezhi.com/api/strategy/routes-adopted
---

# 已采用路线列表

`GET /strategy/routes/adopted` — 获取当前用户已采用的因子路线列表。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

无。

## 响应

```json
[{ routeCode, name }]
```

## 实测

```bash
curl "https://api.shengkezhi.com/open/v1/strategy/routes/adopted" \
  -H "Authorization: Bearer sk_xxx"
```

```json
[
  {"routeCode":"RT_20260724_001","name":"放量突破后的短期动量"},
  {"routeCode":"RT_20260724_002","name":"银行股低估值修复"}
]
```

## 与「因子路线列表」的区别

本接口只返回当前用户已采用路线的 routeCode 与名称，字段精简，专供「选一条已有路线」的下拉选择器使用。

若要拿到路线的完整信息（核心洞察、经济学逻辑、失效场景、标签、创建时间），请用投研接口的因子路线列表。
