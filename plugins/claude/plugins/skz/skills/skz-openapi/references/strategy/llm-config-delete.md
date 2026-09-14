---
title: 删除自定义大模型配置
description: 胜可知开放平台 DELETE /strategy/llm-configs/{id}：删除当前账户下的指定大模型配置。
source: https://docs.shengkezhi.com/api/strategy/llm-config-delete
---
# 删除自定义大模型配置

**`DELETE /strategy/llm-configs/{id}`** — 删除当前账户下的指定大模型配置。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

成功返回 HTTP `204 No Content`，响应体为空。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/strategy/llm-configs/1b91e8e7-84ed-4f03-99ea-97f850555a40" \
  -H "Authorization: Bearer sk_xxx"
```

```json
null
```
