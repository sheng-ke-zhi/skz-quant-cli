---
title: 组合构建历史
description: 前端同款 Agent 组合构建流程。
---
# 组合构建历史

**`GET /strategy/portfolios/builds`**

认证为当前用户的 Bearer token，前缀为 `/open/v1`；服务端映射到 C# `/api/strategy/portfolios/builds`，不经过 Rust `/research/portfolios` 创建入口。返回统一信封 code/msg/data，CLI 解封装 data。

CLI：`skz portfolio build history --offset 0 --limit 20`。

参数 offset（默认0）、limit（1–100，默认20）、search、status；返回运行数组。status 为 running/needs_review/requirements_unmet/failed/timed_out。自行根据 offset 翻页。
