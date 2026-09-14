---
title: 预估研究任务费用
description: 胜可知开放平台 POST /strategy/tasks/preview：预估批量研究任务费用并检查可能重复的任务。
source: https://docs.shengkezhi.com/api/strategy/tasks-preview
---
# 预估研究任务费用

**`POST /strategy/tasks/preview`** — 预估批量研究任务费用并检查可能重复的任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求

`items` 支持 `factor_mining` 和 `strategy_exploration`。因子挖掘项可携带 `llm` 自定义模型配置。

## 响应

返回 `items`、`estimatedTotalChargeCent`、`possibleDuplicateCount`。每个预估项包含任务类型、路线、问题、预计费用、重复提示、模型模式和计费 Scope。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/tasks/preview" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"items":[{"kind":"factor_mining","routeCode":"RT_20260913_001"}]}'
```

```json
{
  "items": [
    {
      "index": 0,
      "kind": "factor_mining",
      "routeCode": "RT_20260913_001",
      "problemCode": null,
      "estimatedChargeCent": 100,
      "possibleDuplicate": false,
      "llmMode": "platform",
      "chargeScope": "factor_mining"
    }
  ],
  "estimatedTotalChargeCent": 100,
  "possibleDuplicateCount": 0
}
```
