---
title: 提交 Agent 组合构建
description: 前端同款 Agent 组合构建流程。
---
# 提交 Agent 组合构建

**`POST /strategy/portfolios/builds`**

认证为当前用户的 Bearer token，前缀为 `/open/v1`；服务端映射到 C# `/api/strategy/portfolios/builds`，不经过 Rust `/research/portfolios` 创建入口。返回统一信封 code/msg/data，CLI 解封装 data。

CLI：`skz portfolio build start < build-request.json`。

付费写操作，提交前按运行契约核费并取得本次参数许可，不自动重试。请求字段 camelCase：market、goal、problemCodes、strategyCodes、minTrials、maxExplorations、maxAttempts、evaluationConcurrency、timeoutSeconds、allowPartial、selectionConstraints、llm。goal/market 必填；其它缺省沿用后端语义。llm 使用 platform，或 byok + configId，或 byok + apiKey/baseUrl/model（可选 saveAs）。模型密钥从受限来源注入，禁止日志回显。

方法、调仓规则和品种要求写入 goal；requiredMethod/rebalanceRule/desiredName/lookbackDays 仅为后端读取历史保留，新建会清空，CLI 明确拒绝。预算和候选由后端在扣费前核验。

202 返回 runId、status、request 等运行信息；running 仅表示受理。422 为输入错误，409 为已有构建，402 为余额不足，503 为服务不可用。断连或回执不确定时用 latest/history 核对，不自动重发。
