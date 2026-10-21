# 与前端相同的 Agent 新建组合

用户给出自然语言目标和策略/研究问题范围，希望 Agent 探索并生成配置时，使用 `portfolio build`。已有确定的 portfolio.toml 需要直接导入时，使用 `portfolio create`；两者是不同流程，不把 TOML 或旧创建字段传给 Agent。

先按运行契约核对身份，再读取范围和已有任务：

```bash
skz portfolio build catalog
skz portfolio build latest
skz llm-config list
```

目录包含 markets、problems、strategies，结合当前市场选择问题或实盘策略；选择问题时，后端会把它关联的实盘策略并入候选。不要自行把问题集合替换成单一策略，也不需要提前猜最终组合 code。目录的 supportedMethods/engineMethods 供理解环境能力，实际目标仍写在 goal。

提交前按 billing.md 检查本次模型模式对应费用与余额：平台模型用 `skz wallet check portfolio-build --qty 1`，自有模型用 `skz wallet check portfolio-build-byok --qty 1`。这是 CLI 版本价目，pricingSource 为 cli，不是实时服务端报价；已知部署价格变化时先核对。BYOK 模型服务商费用另计。取得绑定本次 goal、范围、预算、模型和费用的明确许可后，才可提交。这里说明接口，不授权实际创建。

```bash
skz portfolio build start < build-request.json
```

build-request.json 示例（编号须替换成 catalog 返回的真实值）：

```json
{
  "market": "future",
  "problemCodes": ["PB_BONDS"],
  "strategyCodes": ["LIVE_BOND_1"],
  "goal": "只交易国债 T/TL，每月末调仓，风险归一后使用 skfolio 最小化半方差，优化权重上限15%",
  "minTrials": 30,
  "maxAttempts": 45,
  "evaluationConcurrency": 4,
  "timeoutSeconds": 2700,
  "selectionConstraints": {"max_drawdown": 0.2},
  "llm": {"mode": "platform"}
}
```

- `market`、`goal` 必填；`problemCodes`、`strategyCodes` 可省略，默认空数组。是否可执行及实际候选由后端按当前用户数据判定。
- `minTrials`、`maxExplorations`、`maxAttempts`、`evaluationConcurrency`、`timeoutSeconds` 为可选探索预算，不传时沿用后端默认值。
- `selectionConstraints` 支持 max_drawdown、max_volatility、min_sharpe；回撤使用小数，如 0.2 表示 20%。具体数值由后端在扣费前校验。
- `llm` 省略或 `{"mode":"platform"}` 使用平台模型；已保存 BYOK 配置使用 `{"mode":"byok","configId":"11111111-1111-1111-1111-111111111111"}`。也支持从受限来源注入 apiKey/baseUrl/model 和可选 saveAs，但不能与 configId 混用；不要把真实密钥写进对话、命令参数或提交的文件。
- `allowPartial` 可显式设置为 true/false；省略时沿用后端语义，CLI 不补入历史表单默认值。
- 新请求不接收 `requiredMethod`、`rebalanceRule`、`desiredName`、`lookbackDays`、`candidateStrategyCodes`。后端会清空旧表单约束或重新推导候选；方法、调仓规则、回看和品种要求统一写入 goal，避免“传了字段却没有生效”。

提交返回的 `runId` 是运行身份，`status:"running"` 仅表示已受理，不代表构建成功。后续只读查询：

```bash
skz portfolio build get <runId>
skz portfolio build latest
skz portfolio build history --offset 0 --limit 20
skz portfolio build history --status requirements_unmet --search T/TL
skz portfolio build notice
```

历史分页 limit 为 1–100，offset 从 0 开始；按实际返回数量继续翻页，不把第一页当全集。

`needs_review` 表示产物已生成但需要核对目标、配置和回测；它不是自动认可结果或启用实盘的许可。`requirements_unmet` 表示要求未满足，应展示 requirementMismatches，不能宣称成功。`failed`、`timed_out` 报告错误并核对已有任务，不能为恢复监控重发 start。生成 code 以 resultCode 为准；随后用 `portfolio get <code>`、`portfolio config <code>` 检查实际组合。

确认已处理通知后可用 `skz portfolio build read <runId>` 标记已读（写入，不触发计算）。只读模式允许 catalog/latest/get/history/notice，禁止 start/read。运行时刻输出转北京时间；request、result 和日期语义不改写。

start/read 都不自动重试。start 超时或断连返回 check_existing，按 `skz portfolio build latest` 和历史核对身份、目标及时间；没有可信 runId 时保留未知，不重发、不自动创建替代任务。HTTP 409 表示已有构建；先查询已有运行。每次重新 start 都是新的可能扣费操作，不是免费的恢复监控。
