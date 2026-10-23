---
name: skz-auto-research
description: 胜可知（Shengkezhi）平台的端到端自动化投研编排——把交易经验复现或新研究问题跑成因子、候选策略与单文件 HTML 审核报告。当用户说「复现交易策略」「自动跑一轮挖因子加探索」「用既有因子路线解决新问题」「跑完出汇总报告」时使用。默认只读预检，创建路线与付费触发须用户明确授权；不自动保存入库、删除资产或切换实盘状态。模糊想法与单步研究使用 skz-guide。
---

# skz 自动化投研编排

把已确认的研究计划跑成因子、候选策略和一份可审核的 HTML 报告，不代替用户做投资决定。

## 使用前加载

执行任何 `skz` 命令前读取 [references/operating-contract.md](references/operating-contract.md) 和
[references/cli-contract.md](references/cli-contract.md)；付费前读取 [references/billing.md](references/billing.md)。
申请确认、解释结果和交付时读取 [references/communication.md](references/communication.md)。
按场景读取 [references/scenario-a-replay.md](references/scenario-a-replay.md) 或
[references/scenario-b-newproblem.md](references/scenario-b-newproblem.md)，生成报告前读取
[references/report-spec.md](references/report-spec.md)。共同契约优先于历史案例；案例的耗时、价格与统计不是当前平台保证。

公共安装、工具和技能职责见 [运行契约](references/operating-contract.md#公共安装工具与职责)。任务跨边界时切换技能，不猜另一册的契约。

## 场景与范围

| | A · 交易经验复现 | B · 新研究问题 |
|---|---|---|
| 起点 | 用户原话与待验证机制 | 已确认且已存在的研究问题 |
| 路线 | 优先复用同机制路线；新建需七字段确认与机制去重 | 用户勾选的既有路线 |
| 挖掘 | 串行，跑到批准轮次或低 retained 正常早停 | 不挖掘、不创建路线 |
| 探索 | 一条路线 × 已确认问题 | 已确认路线 × 已确认问题的笛卡尔积 |
| 尾段 | 等待终态 → 抓取 → 报告 → 指路人工审核 | 同左 |

A 建议至少 5 个同类问题、10 轮挖掘、retained 阈值 50；这些是研究建议，不是强制平台约束。
用户可明确批准较小实验，driver 不补建问题、不扩大费用。问题创建交给 `skz-create-problem`，先展示完整定义再确认。新路线的七字段成色与机制查重先交给 `skz-create-route`；driver 只提交已确认的正文。
B 推荐按因子库存、探索覆盖、同类问题历史通过/候选情况和 dataset/freq 适配给分档清单；
`avg_sharpe:null` 不等于质量差，零库存路线不得触发探索。研究要求使用用户原话，不编造摘要替换原文。

## 安全闸门

- 开场及每次新写前检查 `auth status`、`whoami`；只读身份停止写，不切换账号、key 或环境策略。
- 每次付费前动态 `wallet check`，展示核心假设、失败判据、费用与预计等待，取得用户对完整计划的明确许可。
  `--yes-paid` 不等于许可：必须同时有匹配账号、目录、配置、选项快照与预算的人审记录；脚本不能自批。
- 每次写前 fsync 落盘 intent、逻辑键和费用保留；响应 ID 立即持久化。
  写超时/缺 ID 是未知，不是失败。只按可信回执 ID 与任务台核验，不能按时间猜 ID，不能重发或释放费用。
- `queued/dispatching/running` 仍在途。成功要求 `succeeded/done:true/ok:true`；失败要求 `failed/done:true/ok:false`。
  timeout、矛盾/未知状态、未知提交暂停；本地进程停止不意味着云端任务取消。
- 默认不补跑付费任务。用户另行批准后，白名单明确执行失败才可每个逻辑任务补跑一次；
  保留失败/未知费用及原计划余款，不能通过少做原计划腾预算。权限、配置、无因子、timeout 不补跑。
- 工作目录排他锁只保护本目录，不是账号全局锁。一个账号不得并发启动多个写流水线。
  未归属本账本的云端在途任务先人工核验；旧账本不自动迁移，不删除锁、不杀其他 worker。
- 不自动 `promote`、删除资产或切策略状态；报告仅指路到策略审核页，后续动作交给对应技能并重新确认。

## 执行入口

脚本仅依赖 Python 3.10+ 标准库和已安装的 `skz`；排他锁使用 POSIX `fcntl`。
支持 macOS、Linux、WSL；原生 Windows 不支持此 driver，请在 WSL 内安装 CLI 并执行，不能移除锁来绕过限制。
`run_pipeline.sh` 另需 Bash；也可直接 `python3 pipeline.py`，参数相同。

脚本路径必须从本次实际加载的 skill 根目录解析，不硬编码某个平台的安装路径。
下面的 `SK` 指该根目录下的 `scripts`；路径含空格时始终加引号。

```bash
SK="<本技能根目录>/scripts"
# 默认仅云端只读预检；会写本地状态、回执与快照。
bash "$SK/run_pipeline.sh" --config <计划.json> --workdir <独占目录>
# 用户确认后，再保存人工审批记录并执行；不得替用户授权。
bash "$SK/run_pipeline.sh" --config <计划.json> --workdir <独占目录> \
  --yes-paid --approval <审批.json> --budget-cent <预算分>
# 新路线额外 --create-route；另行授权一次补跑才加 --retry-authorization。
bash "$SK/run_pipeline.sh" --help
```

计划、审批和补跑记录的字段及例子见 CLI 契约第 9 节。当前唯一写入口是 `pipeline.py`（shell 仅 exec）；
不分发旧 `orchestrate.py`，避免重复编排、混用账本或绕过新闸门。共享 `scripts/resume.py` 只能帮助只读定位在途任务，
不会自动将历史任务导入本账本。

## 长任务与交付

等待统一遵循[共同契约：长任务等待与宿主生命周期](references/operating-contract.md#长任务等待与宿主生命周期)：默认前台阻塞并收集结果，受管后台必须同时有宿主持久托管与可靠完成唤醒保证。监控窗口到期时保存真实 ID 和账本、报告暂停；后续以同一配置/独占目录续跑，不重发任务，本地停止不等于云端取消。

driver 等待完整链路，写 `events.ndjson`、`pipeline_state.json`、`fetched/summary.json` 与 `report.html`。
事件是本地日志，不是推送承诺；是否显示进度由宿主能力决定。只有 `final_completed` 与报告哈希校验通过才可交付。
退出 0 = 完整成功；退出 1 = 失败或部分数据缺失报告；退出 2 = 暂停需核验，不等于云端停止。
结构校验不是浏览器视觉验证；交付前检查目录锚点、UTF-8、章节、哈希并在可用浏览器检查布局，不能假称已渲染。
通过宿主支持的附件/文件展示机制交付 HTML；本地绝对路径不是用户可访问的文件链接。

报告必须区分全周期和后置验证、历史通过数和现存候选数、单轮 retained 和累计库存。
候选阶段统一称「后置验证表现」，尚不能确认真正样本外；分别列明验证区间、训练/筛选使用情况和入库后的 `nav.oos_start`，缺失写未知。日期新旧不能证明独立性，跨越起点不能直接沿用整段指标，详见报告规范与候选册。
缺字段显示“—”并标部分完成，不补零、不编造收益；写清后置日期、成本未知、多重检验和选择偏差。
净敞口低不证明没有市场风险，现金只能是明确标注的参考基准，不冒称风险调整后的超额收益。
结尾指路平台策略审核，明确保存入库与切实盘都是后续独立决定。
