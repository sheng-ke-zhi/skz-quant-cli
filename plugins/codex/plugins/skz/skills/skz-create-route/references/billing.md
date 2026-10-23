# 付费操作与余额检查

以下操作的价格由当前 CLI 版本固定维护，统一以 `skz wallet costs` 输出为准；不要在技能正文另抄金额。该价目不是所有写操作的完整计费目录：

| operation | 对应命令 | 计价单位 |
|---|---|---|
| `mine` | `skz mine start` | 每次挖掘 |
| `explore` | `skz explore start` | 每次探索 |
| `refresh` | `skz strategy refresh` | 去重后的每个策略 |
| `portfolio-build` | `skz portfolio build start`（平台模型） | 每次 Agent 构建 |
| `portfolio-build-byok` | `skz portfolio build start`（自有模型） | 每次 Agent 构建的平台服务费；模型服务商费用另计 |

上述付费写在申请确认前运行：

```bash
skz wallet check <mine|explore|refresh|portfolio-build|portfolio-build-byok> --qty <数量>
```

- `affordable:true`：向用户同时说明本次 `requiredCent`、当前 `availableCent`、核心假设和失败信号，再取得绑定本次参数的确认。
- `affordable:false`：报告 `shortfallCent`，不要触发付费操作；充值只能由用户在平台资金页完成。
- 查询失败：说明余额无法验证并暂停。只有用户随后明确要求跳过余额检查，才可继续原有确认流程。
- 用户已在当前上下文明确批准具体操作时，仍检查余额；检查通过后不重复索要许可。

## Agent 组合构建

`portfolio build start` 与前端调用相同的 portfolio-agent 创建链路，按模型模式选择 portfolio-build 或 portfolio-build-byok 检查项。价格来自支付服务组合构建迁移的初始配置，由 CLI 版本维护，`pricingSource:"cli"` 不表示实时服务端报价。部署改价时须同步核对 CLI；BYOK 另有模型服务商费用。此报价不适用于直接配置的 `portfolio create` 或 `portfolio refresh`，不能混用。检查余额后按运行契约取得本次参数和费用授权，写请求不自动重试。

## 组合创建与刷新：费用未核实则暂停

`portfolio create` 和 `portfolio refresh` 会触发组合优化，按可能付费的写操作管理；服务端当前是否收费、计价单位和实际金额尚未核实。当前 CLI 没有对应的组合报价项。`wallet check refresh` 只对应策略更新，禁止用于组合创建或组合刷新报价；其他操作的单价也不能替代组合报价。

没有组合专属报价能力、报价失败或费用无法核实时，暂停组合创建与刷新，只保留只读查询，并向用户说明缺少的费用依据。用户同意操作、允许跳过余额检查、余额充足或计划校验通过，都不能替代费用核验。本仓库尚未定义人工核价流程，不得自行把口头金额当作报价继续执行。

只有取得可核实的组合专属费用依据后，才能展示本次操作、完整参数、费用来源与金额、当前余额、预计耗时和失败信号，并取得绑定本次参数及费用的明确许可；参数或费用变化必须重新确认。此前不得调用组合写命令，也不得为核价试跑一次。

余额是请求时快照，其他并发消费可能使它立即变化。CLI 检查不预扣、不占用资金，也不证明后端实际扣费；最终结果以后端响应为准。`pricingSource:"cli"` 表示价格随 CLI 版本维护，平台改价时必须升级 CLI 与插件。
