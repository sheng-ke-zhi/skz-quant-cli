---
title: 批量创建研究任务
description: 胜可知开放平台 POST /strategy/tasks：批量加入因子挖掘或策略探索任务队列。
source: https://docs.shengkezhi.com/api/strategy/tasks-create
---
# 批量创建研究任务

**`POST /strategy/tasks`** — 批量加入因子挖掘或策略探索任务队列。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求

每项可提供 `kind`、`routeCode`、`problemCode`、`clientRequestId` 和 `llm`。`clientRequestId` 用于幂等提交。

## 响应

返回 `items` 和 `estimatedTotalChargeCent`。任务项包含任务及运行 ID、任务类型、问题和路线、队列状态与位置、计费状态、错误信息、模型快照、时间字段以及是否可取消。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/tasks" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"items":[{"kind":"factor_mining","routeCode":"RT_20260913_001","clientRequestId":"request-001"}]}'
```

```json
{
  "items": [
    {
      "taskId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
      "fcRunId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
      "kind": "factor_mining",
      "routeCode": "RT_20260913_001",
      "status": "queued",
      "estimatedChargeCent": 100,
      "llmMode": "platform",
      "chargeScope": "factor_mining"
    }
  ],
  "estimatedTotalChargeCent": 100
}
```
