---
title: 重试研究任务支付
description: 胜可知开放平台 POST /strategy/tasks/{taskId}/retry-payment：让支付失败的研究任务重新进入执行队列。
source: https://docs.shengkezhi.com/api/strategy/tasks-retry-payment
---
# 重试研究任务支付

**`POST /strategy/tasks/{taskId}/retry-payment`** — 让支付失败的研究任务重新进入执行队列。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

成功返回 HTTP `204 No Content`；任务不是 `payment_required` 时返回 `409`。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/tasks/01K5D7M8T9ABCDEFGHJKMNPQRS/retry-payment" \
  -H "Authorization: Bearer sk_xxx"
```

```json
null
```
