---
title: 取消排队研究任务
description: 胜可知开放平台 DELETE /strategy/tasks/{taskId}：取消尚未执行的排队任务。
source: https://docs.shengkezhi.com/api/strategy/tasks-cancel
---
# 取消排队研究任务

**`DELETE /strategy/tasks/{taskId}`** — 取消尚未执行的排队任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

成功返回 HTTP `204 No Content`。只有 `queued` 或 `payment_required` 状态可取消。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/strategy/tasks/01K5D7M8T9ABCDEFGHJKMNPQRS" \
  -H "Authorization: Bearer sk_xxx"
```

```json
null
```
