# skz

skz 是胜可知量化平台的命令行工具，可配合 Claude Code、Codex、OpenClaw、Hermes、DSH 和 WorkBuddy 使用。安装 CLI 和对应插件后，可以用自然语言让 AI 助手查询市场数据、挖掘因子、开展策略研究，以及查看实盘策略和管理组合。

插件包含十个技能，分别负责研究引导、因子、候选策略、实盘策略、组合、费用查询等工作。助手按任务选择技能；涉及付费或重要资产变更时，按技能约定先取得你的确认。

## 安装

### Windows：WorkBuddy 一键安装

使用 Windows 和 WorkBuddy 的用户，可以通过一键安装包准备 skz 和完整插件，无需输入安装命令，也无需另装 Node.js、npm 或 CodeBuddy。

1. 先安装 WorkBuddy 桌面版。
2. 打开 [GitHub Releases 下载页](https://github.com/sheng-ke-zhi/skz-quant-cli/releases/latest)，选择 `胜可知-WorkBuddy-Setup-<version>.exe`。下载后的文件名为 `SKZ-WorkBuddy-Setup-<version>.exe`。
3. 保存 WorkBuddy 中的工作，再双击安装包。安装器会显示准备进度；首次运行环境尚未就绪时，会打开 WorkBuddy 等待初始化。完成后会重新打开 WorkBuddy。
4. 在 WorkBuddy 的新对话中说：“我刚装好 SKZ，帮我看看能不能用了。”

首次登录和授权需要本人完成。WorkBuddy 登录与胜可知业务授权相互独立；助手会引导你完成 SKZ 登录。

一键安装包也用于升级和同版修复。目前仅支持 Windows，不支持从 WSL 管理 Windows 桌面版。安装与排障细节见 [WorkBuddy 使用说明](docs/workbuddy.md)。

### 通过包管理器安装 CLI

使用其他 AI 助手，或希望手动管理安装时，选择一种适合本机的方式。

macOS / Linux（Homebrew）：

```bash
brew tap sheng-ke-zhi/tap https://github.com/sheng-ke-zhi/skz-quant-cli
brew install sheng-ke-zhi/tap/skz
```

Windows（Scoop）：

```powershell
scoop bucket add skz https://github.com/sheng-ke-zhi/skz-quant-cli
scoop install skz
```

npm（需 Node.js ≥ 18）：

```bash
npm install -g @shengkezhi-com/skz-quant-cli
```

Homebrew / Scoop 的安装清单随本仓库维护。使用过旧独立仓库的用户，需先按[迁移步骤](docs/releasing.md#已有安装迁移)切换安装源。

### 为 AI 助手安装插件

通过包管理器安装 CLI 后，再安装对应助手的 SKZ 插件。以 Claude Code 为例：

```bash
skz plugin install claude
skz plugin status claude
```

将 `claude` 换成 `codex`、`openclaw`、`hermes`、`dsh` 或 `workbuddy`，即可选择其他助手；`all` 只处理本机检测到的助手。WorkBuddy 一键安装包已包含这一步。

每个助手各有一份用户级 SKZ 安装，包含全部十个技能。Claude Code、Codex、OpenClaw、Hermes 和 WorkBuddy 使用名为 `skz` 的原生插件；DSH 将技能安装到 `$DSH_HOME/skills/`，默认是 `~/.dsh/skills/`。安装成功后会清理带可信 SKZ 归属标记的旧技能；外来或无法确认归属的目录不会被覆盖。

## 登录并确认账户

可以让助手引导登录，也可以自行运行：

```bash
skz auth login personal --allow-write
skz auth use personal
skz auth status
skz whoami
```

`auth login` 会输出授权链接。手动打开链接，登录网页并点击“授权并继续”；需要自动打开浏览器时，加上 `--browser`。登录会保存名为 `personal` 的本机身份，随后由 `auth use` 选为默认身份。

每次登录必须选择 `--allow-write` 或 `--read-only`。上面的示例允许 CLI 发出写请求，适合需要开展研究和管理资产的账户；只查询时使用 `--read-only`。`--allow-write` 不会增加服务端权限，也不替代付费和重要资产操作前的人工确认。

`auth status` 检查本机身份和只读状态，`whoami` 联网确认实际账户归属。多账户、API key、重新登录和远程 SSH 登录见[账户与授权说明](docs/authentication.md)。

## 可以让助手做什么

安装插件并登录后，直接描述你要做的事即可。

| 技能 | 用途 | 你可以说 |
| --- | --- | --- |
| `skz-guide` | 梳理想法、确定研究方向，衔接因子挖掘、研究问题和策略探索 | “帮我研究一个动量策略” |
| `skz-auto-research` | 按已确认的计划串行挖掘、批量探索，生成 HTML 审核报告 | “复现这个策略，跑完出汇总报告” |
| `skz-create-route` | 明确观察对象、经济逻辑与失效条件，查重并在确认后创建因子路线，取得路线 code | “对手盘想清楚了，帮我建这条因子路线” |
| `skz-create-problem` | 设定品种、市场类型、频率和训练／验证时间分段，检查后创建研究问题 | “帮我创建一个股指期货日线择时的研究问题” |
| `skz-factor` | 查看因子库和挖掘产出，做单因子多问题评估，对不成立的因子作软删除 | “看看我的因子库” |
| `skz-candidate` | 评审探索结果，删除候选或将选中的候选保存入库 | “看看这次探索有没有值得保存的” |
| `skz-strategy` | 查看净值、持仓、回撤和交易明细，巡检策略，切换实盘／暂停／废弃状态，管理备注和赠予 | “我实盘最近怎么样” |
| `skz-portfolio` | 将实盘策略按权重组成组合，或按目标交给 Agent 构建；查看净值、回撤和持仓，管理再平衡 | “把这几个策略打包成一个组合” |
| `skz-wallet` | 查询余额、当前 CLI 价目和指定操作的可负担性 | “余额还够挖几次因子？” |
| `skz-openapi` | 查阅开放 API、请求格式和平台页面链接 | “帮我查这个接口的请求格式” |

自动化投研编排需要 Python 3.10+，支持 macOS、Linux 和 WSL；其执行脚本不支持原生 Windows。它按已确认的计划产出因子、候选和报告，不自动保存入库、删除资产或切换实盘状态。详见[自动化投研技能](plugin-src/books/skz-auto-research/SKILL.md)。

### 费用与操作确认

插件要求助手在因子挖掘、策略探索、付费更新，以及保存入库、直接登记策略、切换实盘、删除资产、创建路线／研究问题／组合等操作前，取得绑定本次账户和参数的确认。保存入库、直接登记和创建研究问题不收费，但仍需确认资产变更。

可以先查询余额和当前 CLI 价目：

```bash
skz wallet balance
skz wallet costs
skz wallet check mine --qty 1
```

价目随 CLI 版本维护，不是实时服务端报价。使用自有模型（BYOK）时，模型服务商费用另计，连接探测也可能消耗模型额度。组合直接创建与刷新目前没有对应的 CLI 报价项，费用未核实时助手应暂停操作。具体规则见[费用说明](plugin-src/common/references/billing.md)和[人工确认边界](plugin-src/common/references/operating-contract.md#人工确认边界)。

## 只读运行

登录时选择 `--read-only`，可以限制某个身份。也可以为当前终端启动的所有 skz 命令启用全局只读模式，即使所选身份允许写入，也会阻止业务写入和付费触发。

Bash / zsh：

```bash
export SKZ_READ_ONLY=1
skz auth status
```

PowerShell：

```powershell
$env:SKZ_READ_ONLY = '1'
skz auth status
```

确认输出中的 `readOnly` 为 `true`。写命令被本地策略拒绝时返回退出码 8，不发出目标写请求；读命令和进度轮询仍可使用。

关闭全局只读模式时应移除该环境变量，不能设成 `0` 或 `false`。这项本地策略用于防止误操作；需要服务端强制限制时，应使用不带相应写权限的凭据。助手遇到只读限制应停止写入并交给用户处理，不能自行换身份或修改策略。详见[只读模式说明](docs/authentication.md#全局只读模式)。

## 常用查询

需要手动检查时，可以从这些命令开始。业务查询使用当前默认身份，通常输出 JSON。

| 查询内容 | 命令 |
| --- | --- |
| CLI 和契约版本 | `skz --version` |
| 命令帮助 | `skz --help` |
| 当前身份和只读状态 | `skz auth status` |
| 实际账户归属 | `skz whoami` |
| 钱包余额 | `skz wallet balance` |
| 组合 TOML 原文 | `skz portfolio config <code>` |
| 组合最新目标持仓 | `skz portfolio latest-positions <code>` |

组合配置返回 `{"toml":"..."}`，保留注释和格式，每次读取最新落盘配置。将 `YOUR_PORTFOLIO_CODE` 换成组合 code，再用 jq 提取原文：

```bash
skz portfolio config YOUR_PORTFOLIO_CODE | jq -rj .toml
```

`latest-positions` 每次独立请求最新已落盘权重，不读取组合详情缓存；结果包含 `latest_weights` 和快照时间 `latest_weights_at`。

### Agent 组合构建

除了按确定配置导入组合，还可以提交自然语言目标、研究问题／策略范围，并选择平台模型或自有模型（BYOK），让平台 Agent 探索并生成组合配置。

`skz portfolio build catalog` 查询可选范围；确认参数和费用后，用 `skz portfolio build start < build-request.json` 提交。平台模型和 BYOK 分别用 `skz wallet check portfolio-build --qty 1`、`skz wallet check portfolio-build-byok --qty 1` 检查费用和余额。

通过 `latest`、`get <runId>`、`history` 和 `notice` 查询运行，`read <runId>` 用于标记已读，属于写操作。提交不自动重试，断连后先查 `skz portfolio build latest` 核验已有任务。完整请求与状态说明见 [Agent 构建流程](plugin-src/books/skz-portfolio/references/agent-build.md)；已有确定配置的 JSON / TOML 导入仍使用 `portfolio create`。

## 升级与插件管理

通过 Homebrew、Scoop 或 Windows 一键安装包安装后，使用：

```bash
skz update
```

Homebrew / Scoop 渠道分别执行 `brew upgrade skz`、`scoop update skz`。Windows 一键安装渠道打开中文升级窗口，由你开始升级；也可以使用开始菜单中的“升级胜可知”。运行前请保存 WorkBuddy 中的工作。

npm 安装需通过 npm 升级；`skz update` 无法识别该渠道，会跳过并给出重装指引：

```bash
npm install -g @shengkezhi-com/skz-quant-cli@latest
```

手动检查、刷新或移除助手插件，以 Claude Code 为例：

```bash
skz plugin status claude
skz plugin upgrade claude
skz plugin uninstall claude
```

将 `claude` 换成对应助手名称。`plugin upgrade` 安装当前 CLI 随附的插件；需要新版技能时先升级 CLI。Windows 一键安装包会共同更新 CLI 和插件，升级后保留已有禁用状态。

## 更多文档

- [WorkBuddy 安装、升级与排障](docs/workbuddy.md)
- [账户、OAuth、API key 与只读策略](docs/authentication.md)
- [助手运行契约](plugin-src/common/references/operating-contract.md)
- [开放 API 参考](plugin-src/books/skz-openapi/SKILL.md)
- [开发与架构约定](CLAUDE.md)、[贡献与插件生成规范](AGENTS.md)
- [维护者发布流程](docs/releasing.md)
