# CLI 与当前后端契约对齐

核对全部已支持的业务 HTTP 路由：114 个（研究后端 70、C# 平台后端 43、支付服务 1）。清单见 `tests/fixtures/backend-contracts/routes.json`；本地 auth/plugin/update 操作沿用现有测试。额外扫描 client 的真实路径字符串，发现并补登了组合最新目标持仓接口，避免只核对已有注释清单。

事实来源为本地代码和研究后端生成的 OpenAPI，未调用线上付费接口：

| 来源 | 核验快照 | 核对内容 |
| --- | --- | --- |
| user-research-backend | 20bb9988768a6075adf7b7d8ca7d98029c22a7d5 | 公开路由、请求参数、typed DTO、生成 OpenAPI、成功信封及空响应 |
| skz-backend | 59e14d152dfc1a9316cf26d6a6866b6e45b8f8dd | Market/Explore/Mining/Realtime/Tasks/LLM/Stats/PortfolioCreation 端点、网关路径映射 |
| rs-skz-service | 560315bd4430706187a5d9ed7328d71d08d84e19 | client WalletSummaryVo 及现金账户字段 |

代码快照不等于线上已部署版本；旧部署缺少新字段时保留兼容，不编造空持仓、零计数或不存在的身份信息。

## 修正结果

| 范围 | 修正 |
| --- | --- |
| 实验列表、概览与候选 | 外部 JSON 产物字段不再强制当字符串/浮点；保留完整 model、route、errors 等。列表支持 page/page_size/q/freq/pending_only；候选支持可选分页，不传分页仍返回全量。保留页码元数据。 |
| 研究问题 | 保留 gifted_from、strategy_count 及分页元数据；支持 with_summary、分页和排序。未请求摘要时不把策略数伪造为零。 |
| 因子、挖掘 | 保留 gifted_from；agg 和 route 按后端 Value 边界透传。挖掘耗时保留 JSON 数值原表示。 |
| 策略与绩效 | 保留赠予来源、领取时间、原始策略编号、分析样本起止和来源。分段/复审行、回测 symbol_returns 等动态 JSON 不再按旧样本裁剪。绘图时单独投影，原始响应完整输出。 |
| 组合 | 补齐列表指标、数据状态、刷新状态、曲线、归一化曲线、年度收益、策略配置及持仓快照。业务 dt/data_at 与文件标识原样保留，updated_at 才按事件时刻转北京时间。断连后查证提示指向只读 refresh-status。 |
| 赠予 | 保留展示名称、already_exists、claim_status，支持 processing/failed 状态，不能把处理中或失败误报为成功。 |
| C# 任务与市场 | 保留 queued taskId 和探索 problemCode；交易日历 pretradeDate 可空。其余原样透传、分页、信封及 204 行为按端点核对。 |
| 钱包 | client 现金账户 accountId 已保证存在，保持 i64；没有将 admin 视图的可空性误套到 client 接口。原工作区尚未提交的相关改动保持原状。 |

后端有些 `Value` 字段用 OpenAPI object 占位描述，实际实现允许字符串、数值、数组、对象或 null。CLI 以代码中的 Value 透传边界为准；事件 run_at 的字符串仍按原规则转北京时间，非字符串保持原样。

## 回归入口

- `cargo test --test backend_contracts`：47 个研究响应的非空/可空结构快照及 C#/时间/路由专项校验。样例刷新方式见 `tests/fixtures/backend-contracts/README.md`。
- `cargo test --test cli --test portfolio_build`：真实 CLI 进程 + 本机 HTTP mock，检查请求映射、分页、JSON、只读、不重试及错误动作。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --all --check`。
- 插件作者源变更后通过 `scripts/release/build_plugins.py --sync-only` 生成，禁止手改产物；再运行 `--check` 与插件测试。

本地验证结果：341 项 Rust 测试（含 53 项契约专项）、52 项插件测试、52 项离线安全测试通过；全目标 Clippy、格式、生成一致性和样例重生成比对通过。未执行线上业务创建、扣费、合并或发布。

## 路线级联删除兼容边界（2026-10-30）

后端 #85 经 #89 合入 main 的提交为 `204aa5b7822d9aeeef7c01db4c344ff6169188de`。核对其 `src/api/factors.rs` 的 DTO、预演、护栏和实际删除流程：路线及全部关联因子（含已软删）、标签、评估、汇总同事务物理删除，随后清理挖掘执行目录；清理部分失败不回滚主库。未请求任何生产删除。

| 后端响应 | 本次 CLI 行为 / Agent 决策 |
| --- | --- |
| 新版含 `deleted_factors` | 原样输出预计/本次实际永久删除数量；0 是明确的本次零删除，不代表历史未删过因子。 |
| 旧版不含 `deleted_factors` | 兼容解析，输出 null；不可用 `orphaned_factors` 换算，Agent 停止实删并核对部署。 |
| `failed_mining_runs` 非空的完整实删回执 | exit 0 保持兼容，字段和已删数量完整输出；不自动重发。读回因子（含已软删）、路线、执行，再预演剩余影响并重新确认，最多一次受控续作。 |
| 写传输结果未知 / 读回失败 | 不追加写调用；路线消失不足以证明清理完成。 |

已核对的 GitHub Releases 最新为 2026-07-02 的 `v0.2.8`（指向 `6ff0600645e5c841e20f3e70813c695fa4ade704`），早于本次改动，不能给出包含级联删除的最小发布版本，也不能证明生产是否部署。以源码提交记录边界，以同一身份/API 地址的预演字段确认可观测能力；旧 CLI v0.1.49 的技能/帮助/错误说明不适用于新行为，应随修复版本一起更新插件。响应字段只能识别当前回执，不能保证两次调用间部署或资产不变；影响变化需重新预演、确认。

CLI 仍是非交互 JSON 执行器，人工确认由 Agent 技能及共同操作契约执行，不新增 stdin 提示或伪造机器授权。`--force` 不等于用户授权，不能保留因子。回归覆盖新旧响应、明确零值、预演只读和 force 参数、部分失败单次调用、所有生成平台的确认与续作规则。
