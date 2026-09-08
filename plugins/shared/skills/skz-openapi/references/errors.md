---
title: 错误码
description: 胜可知开放平台统一错误结构与错误码列表：MISSING_API_KEY、INVALID_API_KEY、RATE_LIMITED、TOO_MANY_IPS 等。
source: https://docs.shengkezhi.com/api/errors
---

# 错误码

所有错误返回统一结构：

```json
{ "status": 401, "title": "错误说明", "errorCode": "MISSING_API_KEY" }
```

| HTTP | errorCode | 含义 |
| --- | --- | --- |
| 401 | `MISSING_API_KEY` | 未携带 Key |
| 401 | `INVALID_API_KEY` | Key 无效 / 已吊销 / 已过期 |
| 403 | `IP_NOT_ALLOWED` | 来源 IP 不在该 Key 的白名单内 |
| 403 | `TOO_MANY_IPS` | 当日使用的不同 IP 数已达上限 |
| 429 | `RATE_LIMITED` | 每分钟请求超限 |
| 429 | `QUOTA_EXCEEDED` | 当日配额超限 |
| 404 | `OPEN_ROUTE_UNKNOWN` | 未知的开放平台路由 |
| 400 | — | 参数错误 |

