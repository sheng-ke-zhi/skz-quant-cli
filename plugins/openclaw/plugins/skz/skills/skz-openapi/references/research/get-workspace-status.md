---
title: 工作区状态
endpoint: GET /research/workspace/status
source: https://docs.shengkezhi.com/api/research/get-workspace-status
---

# 工作区状态

`GET /research/workspace/status` — 工作区状态。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/workspace/status`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| initialized | boolean | 是 | 当前用户 workspace 是否已初始化；false 时前端应引导调用 init 拷贝模板数据。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/workspace/status" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "initialized": true
  }
}
```
