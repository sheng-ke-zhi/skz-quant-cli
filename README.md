# skz

胜可知量化平台的命令行工具。装好它，然后直接跟你的 AI 编程助手（Claude Code / Codex / openclaw / hermes / DSH / WorkBuddy）说人话——挖因子、跑策略探索、看实盘，它替你敲命令。

组合 TOML 原文：`skz portfolio config <code>`，输出 `{"toml":"..."}`；保留注释和格式，每次读取最新落盘配置。提取原文可用 `skz portfolio config <code> | jq -rj .toml`。

组合最新目标持仓：`skz portfolio latest-positions <code>`。每次独立请求最新已落盘权重，不走组合详情缓存，输出 `latest_weights` 和快照时间 `latest_weights_at`。

## 三步开工

```bash
# 1. 装（推荐用平台自带的包管理器）
# macOS / Linux（Homebrew）
brew install sheng-ke-zhi/tap/skz
# Windows（Scoop）
scoop bucket add skz https://github.com/sheng-ke-zhi/scoop-bucket
scoop install skz
# 任意平台（npm，需 Node.js ≥ 18）
npm install -g @shengkezhi-com/skz-quant-cli

# 2. 装 SKZ plugin（内含完整投研 skills）
skz plugin install claude             # 也可明确指定 codex / openclaw / hermes / dsh / workbuddy / all

# 3.（可选）配一个默认身份 —— 不配也行，agent 用到时会引导你补
skz auth login personal --allow-write
skz auth use personal
```

每个 harness 一份用户级 SKZ 安装，内部包含
`skz-guide`、`skz-create-route`、`skz-create-problem`、`skz-factor`、`skz-candidate`、`skz-strategy`、`skz-portfolio`、`skz-wallet`、`skz-openapi` 和 `skz-auto-research`。
Claude / Codex / OpenClaw / Hermes / WorkBuddy 装的是名为 `skz` 的原生 plugin；DSH 把技能清单中的十册拷到 `~/.dsh/skills/`。
安装成功后会清理带 SKZ 归属标记的旧 skills；外来或无法安全确认的目录不会被覆盖。

### WorkBuddy（Windows）

先安装并打开 WorkBuddy 桌面版，等待运行环境准备完成，然后运行：

```powershell
skz plugin install workbuddy
skz plugin status workbuddy
skz plugin upgrade workbuddy
skz plugin uninstall workbuddy
```

SKZ 调用桌面包内的 CodeBuddy CLI 管理用户级 `skz@skz`，不要求另装 CodeBuddy。
安装器和 CLI 共用定位逻辑：读取当前用户、当前会话的运行进程，以及当前用户/整机的 Windows App Paths、安装登记（含 32/64 位视图），不递归扫描磁盘、不猜默认安装目录。核对程序产品信息和内置 CLI 后，安装器把所选位置保存在 `~/.skz/plugins/workbuddy/desktop.json`；状态检查只读该记录。登记缺失可先打开 WorkBuddy 再点“重新检测”，或用文件选择器选一次程序。多个有效位置或运行位置与保存位置冲突时要求明确选择，不任取一个版本。
开发排障仍可显式指定 `$env:SKZ_WORKBUDDY_APP_DIR = 'D:\Apps\WorkBuddy'`；也会校验该位置，不回退到其他版本。
配置目录默认是 `~/.workbuddy`，可用 `WORKBUDDY_CONFIG_DIR` 指定绝对路径。
SKZ 优先使用 WorkBuddy 提供的运行时（`WORKBUDDY_EXTRA_PATHS` 或用户配置下的受管 Node），也兼容 PATH 中的 Node.js；不需要额外安装 npm、Node.js 或 CodeBuddy。
Node.js 最低版本由桌面包入口检查（当前为 18.20.8）。

面向不使用命令行的 Windows 用户，从 [GitHub Releases](https://github.com/sheng-ke-zhi/skz-quant-cli/releases/latest) 下载 `SKZ-WorkBuddy-Setup-<version>.exe` 双击运行。安装包随五目标矩阵自动构建发布，维护流程见 [发布说明](docs/releasing.md)。开发时也可本地构建：

```bash
# 维护者：构建机需要 Rust、Windows GNU 目标、MinGW 和 NSIS。
python3 scripts/release/build_windows_installer.py
```

`build_windows_installer.py` 使用 `release-dist/binaries/x86_64-pc-windows-gnu/skz.exe`，
输出面向用户的 `release-dist/windows-installer/胜可知WorkBuddy一键启动.exe` 及 SHA256；发布资产仍保留 `SKZ-WorkBuddy-Setup-<version>.exe`，供现有升级协议识别。
双击后直接显示准备进度，无欢迎页、“开始安装”或“完成”按钮；检查成功后自动打开 WorkBuddy 并关闭进度窗口。首次运行环境尚未准备好时会自动打开 WorkBuddy 等待其初始化，无需手工预装依赖。
首次运行仍会把 CLI 和完整插件保存到当前用户固定目录，配置用户 PATH 和升级入口，并在 Windows“已安装的应用”中提供移除入口；这是自动准备程序，不是完全不写入电脑的便携模式。首次账号登录与授权仍由本人完成。
校验成功后重启 WorkBuddy；运行前请保存对话中的工作。
同一个安装包处理首次安装、升级和同版修复；较旧的包不会降级本机新版。先在临时目录校验完整安装包，再保存旧程序和 SKZ 插件状态，更新程序和实际加载的技能并共同验收；失败恢复上一套，运行中断可再次运行安装包恢复。只通过 WorkBuddy 原生接口调整 SKZ，不覆盖宿主登记文件、账户或聊天数据。
界面显示各步骤中文详情；日志在 `%LOCALAPPDATA%\SKZ\Installer\last.log`，恢复备份和中断记录也在此目录。错误使用 UTF-16LE，日志使用 UTF-8，不依赖系统代码页。
开始菜单“升级胜可知”或明确请求 `skz update` 可打开中文升级窗口，从官方 GitHub Release 下载并校验安装包，再由用户开始升级；不常驻或静默重启。`skz update --check` 只检查，`--download` 只下载校验；`launch_requested:true` 仅表示已请求打开窗口，不能当作升级成功。
Windows“已安装的应用”中可卸载；升级和卸载保留 SKZ 账号凭据及只读策略。已禁用插件升级后保持禁用。
安装包目前未签名，正式对外分发前应签名；此构建命令不发布 GitHub Release 或更改版本号。
严格清洁验收使用 `tests/windows/assert_setup.ps1`：在没有全局 Node/npm/Scoop 的测试机上安装后执行，检查原生插件、技能清单、本地鉴权命令及唯一 PATH 项。
升级验收使用 `tests/windows/test_setup_upgrade.ps1`：在专用交互式会话提供旧版、新版及两种故障验收包，检查共同升级、失败回滚、中断恢复、拒绝降级、同版重装及禁用技能修复；脚本结束保留测试版本供检查，随后用普通卸载/安装恢复交付版本。
定位回归使用 `tests/windows/test_workbuddy_discovery.ps1 -AppDir <真实WorkBuddy目录> -ResolverPath src/plugin/workbuddy_desktop.ps1`，临时文件和测试登记键会自动清理。安装器内部入口 `__workbuddy-desktop` 不出现在用户帮助或技能命令中，只有 `--interactive` 才会显示重新检测/选择界面。

安装或升级后请重启 WorkBuddy 或开启新会话；升级保持已有禁用状态。
`status` 会校验实际缓存内容，禁用不等于未安装。同版缓存损坏或内容落后时，通过宿主原生管理 API 强制刷新：临时回环服务使用随机口令并检查认证，关闭 MCP 加载，不发起模型任务，结束即关闭；不直接写宿主缓存或登记。维护期间只暂时禁用 SKZ，完成或失败后恢复原有启用状态。
WorkBuddy 的 `status` 只读插件登记和文件，不启动宿主 CLI，也不访问宿主认证、会话或日志。安装后可直接在 WorkBuddy 说“我刚装好 SKZ，帮我看看能不能用了”；向导只检查版本和 SKZ 登录状态，不主动扫描宿主目录。WorkBuddy 登录与胜可知业务授权相互独立。
同名市场指向其他来源时拒绝覆盖；同时存在其他 scope 的 SKZ 时拒绝升级共享缓存。
卸载只处理用户范围，其他 scope 仍使用市场时保留市场和本地来源。
首版只支持 Windows，`all` 自动识别本机桌面安装；macOS/Linux 和 WSL 跨系统管理 Windows 暂不支持。

通过 Homebrew、Scoop 或独立 Windows 安装包安装后，都用统一升级入口：

```bash
skz update
```

它只按当前 `skz` 的实际安装路径选择渠道，并分别执行 `brew upgrade skz`、
`scoop update skz`；独立 Windows 安装渠道使用自身标记识别，打开桌面升级窗口。npm 安装的不走这条——`skz update` 认不出 npm 的安装路径，
会跳过升级并给出重装指引，直接 `npm install -g @shengkezhi-com/skz-quant-cli@latest` 即可。

## 然后别自己敲命令，跟 agent 说

| 技能 | 干什么 | 你可以说 |
|---|---|---|
| `skz-guide` | 从一句想法带你走到能上实盘：聊想法 → 定研究方向 → 挖因子 → 定研究问题 → 策略探索 | 「帮我研究一个动量策略」 |
| `skz-auto-research` | 已确认计划的安全批量编排：串行挖掘 / 批量探索 → HTML 审核报告；Python 3.10+，macOS/Linux/WSL | 「复现这个策略，跑完出汇总报告」 |
| `skz-create-route` | 定义、查重并创建因子路线：打磨观察对象、经济逻辑与失效条件，确认后生成路线 code | 「对手盘想清楚了，帮我建这条因子路线」 |
| `skz-create-problem` | 定义并创建研究问题：品种集合、市场类型、频率、训练/验证时间分段的设定规则与自检 | 「帮我创建一个股指期货日线择时的研究问题」 |
| `skz-factor` | 因子资产：因子库、某次因子挖掘的产出、单因子多问题评估、软删不成立的 | 「看看我的因子库」 |
| `skz-candidate` | 探索候选：评审实验结果、删除候选、把选中的候选保存入库 | 「看看这次探索有没有值得保存的」 |
| `skz-strategy` | 已入库策略：净值/持仓/回撤/交易明细、巡检、切换实盘·暂停·废弃、备注和赠予 | 「我实盘最近怎么样」 |
| `skz-portfolio` | 组合资产：把实盘策略按权重打包、查净值/回撤/持仓、再平衡 | 「把这几个策略打包成一个组合」 |

**花钱的，以及会写入或处置重要资产的操作，agent 会先问你**——因子挖掘、策略探索、保存入库、直接登记、切实盘、删因子、删候选、建组合，都得你点头才跑。保存入库、直接登记和创建研究问题不收费。

## 浏览器 OAuth 登录

```bash
skz auth login personal --read-only
skz auth use personal
skz auth status
skz whoami
```

`auth login` 默认只输出授权链接，手动打开后登录网页并点击“授权并继续”。CLI 换取并保存凭据后，浏览器跳到授权成功页。登录只保存身份，不自动切换机器级默认身份；随后用 `auth use` 选择。已有同名身份须加 `--replace`，且每次登录必须显式选择 `--read-only` 或 `--allow-write`。

授权链接写到 stderr；需要自动打开浏览器时加 `--browser`。`--no-browser` 保留兼容，与默认行为一致，不能和 `--browser` 同时使用。浏览器必须能访问运行 CLI 的电脑上的 `127.0.0.1` 回调；远程 SSH 环境需要自行配置端口转发。默认等待 180 秒，可用 `--timeout <秒>` 调整到 1～600 秒。

OAuth 配置从 `SKZ_BASE_URL` 所在 API 服务器的 `/.well-known/openid-configuration` 获取；生产发现结果中的官网旧授权地址会修正为 `https://quant.shengkezhi.com/oauth/authorize`。token 请求固定走当前 API 域名下的 `/oauth/token`，不使用发现文档可能返回的容器内网 token 地址；业务请求仍走 `/open/v1`。OAuth 身份绑定登录时的 API 根地址，不能跨生产、测试环境混用。公网 OAuth 地址要求 HTTPS，本机 mock 允许 HTTP。

access token 到期前 60 秒，在业务请求发出前自动续期并保存轮换后的 refresh token；多个 CLI 进程通过文件锁串行续期，不重放业务写请求。刷新失败或中断后，`auth status` 的 `loginRequired` 为 true，需要重新执行 `auth login <identity> --replace` 并指定写策略。`authType` 表示 `oauth` 或 `api_key`，`expiresIn` 是 access token 剩余秒数（API key 为 null）；`status` 本身只读本地文件，不联网验证或续期。

token 保存在原有受限权限凭据文件，stdout 只输出身份信息。`--account` 是本地账户标签，不代表后端核验的账户；实际归属用 `skz whoami` 确认。`auth remove <identity>` 仅删除本机身份，不撤销服务端授权。已有 API key 登录方式继续可用：

```bash
echo "sk_你的key" | skz auth add api-key --allow-write
skz auth use api-key
```

## 多账户与只读身份

每把 key 存成一个命名身份，随后选择机器级默认身份。切换身份不会自动发请求：

```bash
echo "sk_alice的key"   | skz auth add alice   --read-only
echo "sk_bob的key"     | skz auth add bob     --allow-write
echo "sk_charlie的key" | skz auth add charlie --allow-write

skz auth list
skz auth use alice
skz auth status
```

`--read-only` 是 CLI 本地策略：alice 的读命令照常执行，任何写和触发都在发请求前被拒绝。`--allow-write` 只表示 CLI 允许写，实际权限仍由 key 的后端 scope 决定。agent 不得因为 bob、charlie 都可写就自行选择；每条工作流开始前先用 `auth status` 确认默认身份。

如果同一账户需要多把不同权限的 key，用不同 identity 名并显式声明共同账户：

```bash
echo "sk_alice只读key" | skz auth add alice-read --account alice --read-only
echo "sk_alice写key"   | skz auth add alice-write --account alice --allow-write
```

## 全局只读模式

想让所有身份都只读——比如要放 agent 长时间自己跑——设一个环境变量：

```bash
export SKZ_READ_ONLY=1
skz auth status        # {"present":true,"readOnly":true} ← 一定要亲眼确认这一行
```

设了之后，即使当前身份是 `--allow-write`，所有写和触发（因子挖掘、探索、保存入库、建组合、改状态、删除、笔记标签）仍一律拒绝并退出码 8，**请求根本不会发出去**；读和进度轮询照常。

几件要知道的事：

- **`skz auth status` 那一步别省。** 变量名打错会静默变成「没设」，而你以为设上了。这是这类开关唯一的失效方式，也是唯一的检查手段。
- **关掉只能 `unset SKZ_READ_ONLY`。** 设成 `0` 或 `false` 会直接报错，不会当成关闭——否则一行 `SKZ_READ_ONLY=0 skz ...` 就绕过去了。
- **你自己也一样写不了。** 真要做一次写操作，在另一个没设这个变量的终端里跑。
- **这是防手滑，不是安全边界。** key 就在本机文件里，铁了心绕总有办法。它挡的是 agent 顺手闯祸，不是对抗性行为。要真正不可绕过，得让 key 的主人发一把不带写权限的 key。

## 前端同款 Agent 组合构建

`skz portfolio build catalog` 发现市场与候选；`skz portfolio build start < build-request.json` 提交自然语言目标、问题/策略范围及平台/BYOK 模型。`latest`、`get <runId>`、`history`、`notice` 查询运行，`read <runId>` 标记已读。完整请求和任务状态说明见 [Agent 构建流程](plugin-src/books/skz-portfolio/references/agent-build.md)。

提交前用 `wallet check portfolio-build --qty 1` 或 `wallet check portfolio-build-byok --qty 1` 检查本模型模式的 CLI 版本费用和余额，并确认本次操作。写请求不重试，断连后查 `portfolio build latest`。已有确定配置的 JSON/TOML 导入仍使用 `portfolio create`。
