---
title: 查询构建运行
description: 前端同款 Agent 组合构建流程。
---
# 查询构建运行

**`GET /strategy/portfolios/builds/{id}`**

认证为当前用户的 Bearer token，前缀为 `/open/v1`；服务端映射到 C# `/api/strategy/portfolios/builds`，不经过 Rust `/research/portfolios` 创建入口。返回统一信封 code/msg/data，CLI 解封装 data。

CLI：`skz portfolio build get <runId>`。

只读；没有对应记录时 data 为 null。保留 runId、request、resultCode、actualMethod、result、requirementMismatches；事件时刻转北京时间。needs_review 需要核对结果，requirements_unmet 表示目标未满足，不得自动启用实盘。
