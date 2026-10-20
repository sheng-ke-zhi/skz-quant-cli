这些是结构样例，不是可提交的业务配置或真实计算结果。`research.json` 来自后端 typed DTO 生成的 OpenAPI：每个响应包含非空字段样例与可空字段样例，覆盖当前 CLI 的 47 个研究后端强类型响应。

核验代码快照：

- user-research-backend：20bb9988768a6075adf7b7d8ca7d98029c22a7d5。
- skz-backend：59e14d152dfc1a9316cf26d6a6866b6e45b8f8dd（以当前本地工作区端点实现核对；下文报告记录完整来源）。
- 钱包按支付服务 client WalletSummaryVo 核对，不套用 admin CashSummaryVo 的可空性。

更新时从对应后端代码生成 `/api/openapi.json`，保存为本地文件后运行：

```bash
python3 scripts/contracts/refresh_research_fixtures.py --openapi /path/to/openapi.json
cargo test --test backend_contracts
```

样例生成器不访问网络。测试检查 CLI 能否解析、是否保留后端已知非空字段、数组、布尔值、整数/浮点与动态 JSON。对后端允许省略的可空字段，null 与省略均接受；不要求 CLI 编造缺失字段。

C# 和支付服务的特殊契约以测试内的源码核验样例补充：交易日历的 pretradeDate 可以为 null；排队回执包含 taskId；探索记录包含 problemCode；赠予状态包含 processing/failed。事件时区和业务时间轴的区别另有专项测试。现有 HTTP 测试同时覆盖路径、查询、请求体、信封、空响应及写请求不重试。
