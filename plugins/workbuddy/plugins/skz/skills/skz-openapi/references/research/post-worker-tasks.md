---
title: 创建任务
description: 胜可知开放平台 POST /research/worker/tasks：创建任务。
source: https://docs.shengkezhi.com/api/research/post-worker-tasks
---

# 创建任务

**`POST /research/worker/tasks`** — 创建任务。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `kind` | string | 否 | 任务类型标识；缺省 `echo`。当前通用入口中 `fail` 用于验证失败状态，其余值执行短任务并成功；业务 pipeline 使用各自专用入口。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `status` | string | 是 | 提交后返回的初始状态，通常为 `queued`（已入队等待执行）。 |
| `task_id` | string | 是 | 新排队任务的唯一标识，前端据此轮询 `GET /api/worker/tasks/{id}` 查询进度。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/worker/tasks" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"kind":"echo"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "status": "示例Status",
    "task_id": "a79dfc93b7e64a6cbbe26f2a787a6bad"
  }
}```
