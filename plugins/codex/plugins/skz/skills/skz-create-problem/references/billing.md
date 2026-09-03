# 付费操作与余额检查

价格由当前 CLI 版本固定维护，统一以 `skz wallet costs` 输出为准；不要在技能正文另抄金额。当前四类操作是：

| operation | 对应命令 | 计价单位 |
|---|---|---|
| `mine` | `skz mine start` | 每次挖掘 |
| `explore` | `skz explore start` | 每次探索 |
| `refresh` | `skz strategy refresh` | 去重后的每个策略 |
| `save` | `skz promote start` / `skz strategy register` | 每个策略 |

付费写在申请确认前运行：

```bash
skz wallet check <mine|explore|refresh|save> --qty <数量>
```

- `affordable:true`：向用户同时说明本次 `requiredCent`、当前 `availableCent`、核心假设和失败信号，再取得绑定本次参数的确认。
- `affordable:false`：报告 `shortfallCent`，不要触发付费操作；充值只能由用户在平台资金页完成。
- 查询失败：说明余额无法验证并暂停。只有用户随后明确要求跳过余额检查，才可继续原有确认流程。
- 用户已在当前上下文明确批准具体操作时，仍检查余额；检查通过后不重复索要许可。

余额是请求时快照，其他并发消费可能使它立即变化。CLI 检查不预扣、不占用资金，也不证明后端实际扣费；最终结果以后端响应为准。`pricingSource:"cli"` 表示价格随 CLI 版本维护，平台改价时必须升级 CLI 与插件。
