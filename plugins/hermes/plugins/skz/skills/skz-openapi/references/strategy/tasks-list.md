---
title: 统一研究任务列表
description: 胜可知开放平台 GET /strategy/tasks：分页查询当前账户的统一研究任务。
source: https://docs.shengkezhi.com/api/strategy/tasks-list
---
# 统一研究任务列表

**`GET /strategy/tasks`** — 分页查询当前账户的统一研究任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `page`、`size`、`total`、`items` 和 `estimatedPendingChargeCent`。任务项字段与批量创建接口一致。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/strategy/tasks" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "page": 1,
  "size": 20,
  "total": 1,
  "items": [
    {
      "taskId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
      "kind": "factor_mining",
      "routeCode": "RT_20260913_001",
      "status": "queued"
    }
  ],
  "estimatedPendingChargeCent": 100
}
```
