---
title: 批量轮询研究任务
description: 胜可知开放平台 POST /strategy/tasks/poll：批量查询最多 100 个研究任务的状态和进度。
source: https://docs.shengkezhi.com/api/strategy/tasks-poll
---
# 批量轮询研究任务

**`POST /strategy/tasks/poll`** — 批量查询最多 100 个研究任务的状态和进度。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回任务数组。除统一任务字段外，每项还包含 `percent`、`step` 和 `message` 进度信息。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/tasks/poll" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"taskIds":["01K5D7M8T9ABCDEFGHJKMNPQRS"]}'
```

```json
[
  {
    "taskId": "01K5D7M8T9ABCDEFGHJKMNPQRS",
    "status": "running",
    "percent": 35,
    "step": "evaluate",
    "message": "正在评估候选因子"
  }
]
```
