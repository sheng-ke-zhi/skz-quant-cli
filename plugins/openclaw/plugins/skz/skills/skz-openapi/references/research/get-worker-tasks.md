---
title: 任务列表
endpoint: GET /research/worker/tasks
source: https://docs.shengkezhi.com/api/research/get-worker-tasks
---

# 任务列表

`GET /research/worker/tasks` — 任务列表。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/worker/tasks`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | TaskInfo[] | 是 | 当前用户名下的任务清单，每项为一条任务的状态快照（内存态，服务重启不保留）。 |

### TaskInfo 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| error | string \| null | 否 | 失败时的错误信息；成功或尚未失败时为空。 |
| id | string | 是 | 任务编号，后端生成（形如 task-3），前端用它轮询任务进度。 |
| kind | string | 是 | 任务类型标识，如 workspace 初始化对应 workspace_init。 |
| progress_percent | integer \| null | 否 | Copy jobs expose coarse byte progress; non-copy jobs leave it empty. |
| status | TaskStatus | 是 | 任务当前状态（queued 到 running 到 succeeded 或 failed）。 |
| user_id | string | 是 | 任务归属用户，取自网关注入的 X-Skz-User-Id；任务列表按此隔离。 |

### TaskStatus 取值

| 取值 | 说明 |
| --- | --- |
| queued | 已入队等待，尚未拿到并发令牌，未开始执行。 |
| running | 已拿到并发令牌，正在执行中。 |
| succeeded | 执行成功完成。 |
| failed | 执行失败（含任务内部 panic），失败原因见 error 字段。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/worker/tasks?page=1&page_size=2" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": []
  }
}
```
