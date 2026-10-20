---
title: 标记构建通知已读
description: 前端同款 Agent 组合构建流程。
---
# 标记构建通知已读

**`POST /strategy/portfolios/builds/{id}/read`**

认证为当前用户的 Bearer token，前缀为 `/open/v1`；服务端映射到 C# `/api/strategy/portfolios/builds`，不经过 Rust `/research/portfolios` 创建入口。返回统一信封 code/msg/data，CLI 解封装 data。

CLI：`skz portfolio build read <runId>`。

普通写操作，不重新计算或扣费，返回 boolean；受 CLI 只读策略限制，不自动重试。
