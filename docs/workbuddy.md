# WorkBuddy 安装与排障

SKZ 的 WorkBuddy 集成目前仅支持 Windows，不支持 macOS、Linux 或从 WSL 管理 Windows 桌面版。安装前需已有 WorkBuddy 桌面版；一键安装包负责准备 skz 和完整插件。

## 一键安装

从 [GitHub Releases](https://github.com/sheng-ke-zhi/skz-quant-cli/releases/latest) 下载 `胜可知-WorkBuddy-Setup-<version>.exe`。发布列表显示中文标签，实际下载名为 `SKZ-WorkBuddy-Setup-<version>.exe`，下载后可自行重命名。

保存 WorkBuddy 中的工作后，双击安装包。程序会直接显示准备进度，无欢迎页、“开始安装”或“完成”按钮；检查成功后自动打开 WorkBuddy 并关闭进度窗口。首次运行环境尚未就绪时，会打开 WorkBuddy 等待初始化，无需手工预装依赖。

安装包会将 CLI 和完整插件保存到当前用户固定目录，配置用户 PATH 和升级入口，并在 Windows“已安装的应用”中提供移除入口。首次账号登录与授权仍由本人完成。

安装或升级后，在重新打开的 WorkBuddy 中开启新会话，可以说：“我刚装好 SKZ，帮我看看能不能用了。”向导只检查版本和 SKZ 登录状态，不主动扫描宿主目录。WorkBuddy 登录与胜可知业务授权相互独立。

## 手动安装与插件管理

已通过 Scoop 等渠道安装 skz 的用户，先安装并打开 WorkBuddy，等待运行环境准备完成，再执行：

```powershell
skz plugin install workbuddy
skz plugin status workbuddy
```

刷新和移除插件：

```powershell
skz plugin upgrade workbuddy
skz plugin uninstall workbuddy
```

安装或升级后重启 WorkBuddy，或开启新会话；运行前请保存对话中的工作。升级会保留已有禁用状态，`status` 显示禁用不等于未安装。

SKZ 通过桌面包内的 CodeBuddy CLI 管理用户级 `skz@skz`，无需另装 CodeBuddy。运行时优先使用 `WORKBUDDY_EXTRA_PATHS` 或用户配置目录下的受管 Node，也兼容 PATH 中的 Node.js；无需额外安装 npm 或 Node.js。桌面包入口负责检查 Node.js 最低版本，目前为 18.20.8。

`skz plugin install all` 只在检测到本机受支持的 WorkBuddy 桌面安装时处理它。

## 升级、修复与卸载

一键安装渠道可从开始菜单选择“升级胜可知”，或运行：

```powershell
skz update
```

这会打开中文升级窗口，从官方 GitHub Release 下载并校验安装包，再由用户开始升级。它不会常驻或静默重启 WorkBuddy。`launch_requested:true` 只表示已请求打开窗口，最终结果以安装器报告为准。

也可以只检查新版本，或只下载并校验安装包：

```powershell
skz update --check
skz update --download
```

同一个安装包处理首次安装、升级和同版修复；旧安装包不会降级本机新版。安装器先在临时目录校验完整安装包，再备份旧程序和 SKZ 插件状态，更新程序及实际加载的技能并共同验收。失败时恢复上一套；运行中断后可再次运行安装包恢复。

升级和卸载均保留 SKZ 账号凭据及只读策略。升级后，已禁用插件保持禁用。可在 Windows“已安装的应用”中卸载一键安装的程序；手动管理的插件用 `skz plugin uninstall workbuddy` 移除。

插件卸载只处理用户范围；其他 scope 仍使用市场时，会保留相应市场和本地来源。

## 常见问题

### 找不到 WorkBuddy，或检测到多个版本

安装器和 CLI 共用定位逻辑，读取当前用户、当前会话的运行进程，以及当前用户／整机的 Windows App Paths 和安装登记，包含 32 / 64 位视图。它会核对程序产品信息和内置 CLI，不递归扫描磁盘或猜测默认安装目录。

安装器将选定位置保存在 `~/.skz/plugins/workbuddy/desktop.json`。状态检查只读该记录。登记缺失时，先打开 WorkBuddy，再在安装器中选择“重新检测”，或通过文件选择器选定程序。存在多个有效位置，或运行位置与保存位置冲突时，需要明确选择版本。

开发排障时可显式指定位置；该位置仍会经过校验，不会回退到其他版本：

```powershell
$env:SKZ_WORKBUDDY_APP_DIR = 'D:\Apps\WorkBuddy'
```

配置目录默认是 `~/.workbuddy`，可用 `WORKBUDDY_CONFIG_DIR` 指定绝对路径。

### 状态显示内容损坏或落后

`skz plugin status workbuddy` 会校验插件登记和实际缓存内容，只读文件，不启动宿主 CLI，也不访问宿主认证、会话或日志。

通过 `skz plugin upgrade workbuddy` 刷新。同版缓存损坏或内容落后时，SKZ 会通过宿主原生管理 API 强制刷新，不直接改写宿主缓存或登记文件。维护期间只暂时禁用 SKZ，完成或失败后恢复原有启用状态。

同名市场指向其他来源时拒绝覆盖；同时存在其他 scope 的 SKZ 时拒绝升级共享缓存。遇到这类提示，先在 WorkBuddy 中处理冲突，再重试。

### 安装或升级失败

界面显示各步骤的中文详情，日志位于 `%LOCALAPPDATA%\SKZ\Installer\last.log`，恢复备份和中断记录也在此目录。安装器错误信息使用 UTF-16LE，日志使用 UTF-8，不依赖系统代码页。

保留日志和恢复记录，再运行安装包恢复。安装器只通过 WorkBuddy 原生接口调整 SKZ，不覆盖宿主登记文件、账户或聊天数据。

## 开发与验收

### 本地构建

构建机需要 Rust、Windows GNU 目标、MinGW 和 NSIS。在仓库根目录运行：

```bash
python3 scripts/release/build_windows_installer.py
```

脚本构建 Windows 目标和插件 bundle，再使用 `release-dist/binaries/x86_64-pc-windows-gnu/skz.exe` 制作安装器，输出 `release-dist/windows-installer/胜可知WorkBuddy一键启动.exe` 及 SHA256；发布资产保留 `SKZ-WorkBuddy-Setup-<version>.exe`，供升级协议识别。

此命令不发布 GitHub Release，也不更改版本号。安装包目前未签名，正式对外分发前应签名。正式安装包随五目标矩阵构建发布，流程见[维护者发布说明](releasing.md)。

### 原生插件维护

同版重装可能复用损坏的宿主缓存。SKZ 临时启动回环服务，通过带随机口令认证的宿主 API 请求强制刷新；维护时关闭 MCP 加载，不发起模型任务，结束即关闭服务。它不直接写宿主缓存或共享登记文件。

安装器内部入口 `__workbuddy-desktop` 不出现在用户帮助或技能命令中，只有 `--interactive` 才显示重新检测／选择界面。

### Windows 验收脚本

从仓库根目录运行，具体参数见各脚本：

| 验收范围 | 脚本 | 环境与检查内容 |
| --- | --- | --- |
| 清洁安装 | [assert_setup.ps1](../tests/windows/assert_setup.ps1) | 没有全局 Node / npm / Scoop 的测试机；检查原生插件、技能清单、本地鉴权命令及唯一 PATH 项 |
| 升级与恢复 | [test_setup_upgrade.ps1](../tests/windows/test_setup_upgrade.ps1) | 专用交互式会话，提供旧版、新版及两种故障验收包；检查共同升级、失败回滚、中断恢复、拒绝降级、同版重装及禁用技能修复 |
| 安装位置识别 | [test_workbuddy_discovery.ps1](../tests/windows/test_workbuddy_discovery.ps1) | 使用 `-AppDir <真实WorkBuddy目录> -ResolverPath src/plugin/workbuddy_desktop.ps1`；自动清理临时文件和测试登记键 |

升级验收脚本结束后保留测试版本供检查，随后需用普通卸载／安装流程恢复交付版本。
