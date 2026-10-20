---
title: 读取构建目录
description: 前端同款 Agent 组合构建流程。
---
# 读取构建目录

**`GET /strategy/portfolios/builds/catalog`**

认证为当前用户的 Bearer token，前缀为 `/open/v1`；服务端映射到 C# `/api/strategy/portfolios/builds`，不经过 Rust `/research/portfolios` 创建入口。返回统一信封 code/msg/data，CLI 解封装 data。

CLI：`skz portfolio build catalog`。

只读，返回 userId、markets、problems、strategies 以及支持的方法和规则目录。候选以当前用户目录为准。
