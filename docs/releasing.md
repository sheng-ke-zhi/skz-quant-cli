# 维护者发布

正式发布由推送 `vX.Y.Z` annotated tag 触发 `.github/workflows/release.yml`，没有手动触发入口。本地只准备版本，不再构建或上传发行文件。

## 五目标矩阵

| 目标 | GitHub runner | 链接工具 |
| --- | --- | --- |
| `aarch64-apple-darwin` | `macos-15` | 系统工具链 |
| `x86_64-apple-darwin` | `macos-15-intel` | 系统工具链 |
| `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` | `musl-gcc` |
| `x86_64-unknown-linux-musl` | `ubuntu-24.04` | `musl-gcc` |
| `x86_64-pc-windows-gnu` | `windows-2025` | MSYS2 MINGW64 GCC |

五份二进制都用 Rust 1.98.0、`--locked --profile dist`，并在各自 runner 执行 `--version`。保留现有 musl 和 Windows GNU 下载协议。调整 Rust 版本时同步 `rust-toolchain.toml`、CI 和发布 workflow。

流程为：校验 tag、凭据和同一提交的 main CI → 五目标并行编译 → Ubuntu 统一打包 → 独立任务发布。发布提交必须已经通过 `ci.yml` 的 main push 检查；如果原子推送同时触发 CI，校验任务最多等待 10 分钟。

每个目标只编译一次。统一生成一份插件 bundle，复用 Windows 二进制制作 WorkBuddy NSIS 安装器，再生成五份归档、SHA256SUMS 和六个 npm 包。发布任务只下载并校验锁定产物，不编译、不重新打包。main CI 已有的测试也不在发布任务重复运行。

构建和发布产物用 tar 传输，保留隐藏插件目录及执行权限；上传时关闭重复压缩。Artifacts 保留 7 天。不增加定时预热、缓存构建或其他平台的构建任务。

## WSL 本地构建

`python3 scripts/release/build_wsl.py` 默认构建上述五个目标，只生成本地产物，不发布。需要 `musl-tools`、`gcc-mingw-w64-x86-64`、`cargo-zigbuild` 和 Zig；可用 `uv tool install ziglang` 安装 `python-zig`。

macOS 的系统证书验证会链接 Security 和 CoreFoundation，因此还需完整 macOS SDK。在装有 Xcode 或 Command Line Tools 的 Mac 上运行 `xcrun --sdk macosx --show-sdk-path` 获取 SDK 路径，将整个 SDK 目录复制到 WSL（保留符号链接），再按 [cargo-zigbuild 的 SDKROOT 配置](https://github.com/rust-cross/cargo-zigbuild#macos-targets)设置实际路径：

```bash
export SDKROOT="$HOME/sdk/MacOSX.sdk"
python3 scripts/release/build_wsl.py
```

SDK 中必须包含 `System/Library/Frameworks/{Security,CoreFoundation}.framework` 的 `.tbd` 链接库和 `usr/lib/libSystem.tbd`。缺失或路径无效时，脚本会在开始任何目标的编译前停止并提示；不会静默跳过 macOS。原生 Mac 构建仍使用系统工具链，交叉构建产物需在真实 Mac 冒烟。

只构建 Linux/Windows 时不需要 SDK，可重复指定 `--target`：

```bash
python3 scripts/release/build_wsl.py \
  --target x86_64-unknown-linux-musl \
  --target aarch64-unknown-linux-musl \
  --target x86_64-pc-windows-gnu
```

## 一次性配置

### Homebrew / Scoop

Formula 位于主仓库 `Formula/skz.rb`，Scoop manifest 位于 `bucket/skz.json`。发布任务用 Actions 自带的 `GITHUB_TOKEN`，以一次提交更新 main 中的两份 metadata；无需额外 PAT。main 必须允许该 token 直接推送；并发修改导致非快进时停止，重跑发布任务即可。此 token 产生的 metadata push 不会再触发 CI。

### npm

在主仓库 **Settings → Secrets and variables → Actions → Repository secrets** 中添加 `NPM_TOKEN`，填写有以下六个包发布权限的 npm token：

- `@shengkezhi-com/skz-quant-cli`
- `@shengkezhi-com/skz-quant-cli-darwin-arm64`
- `@shengkezhi-com/skz-quant-cli-darwin-x64`
- `@shengkezhi-com/skz-quant-cli-linux-arm64`
- `@shengkezhi-com/skz-quant-cli-linux-x64`
- `@shengkezhi-com/skz-quant-cli-win32-x64`

工作流使用 Node 24、npm 11.17.0，通过 `setup-node` 配置 npm registry，并仅在发布步骤将 secret 注入 `NODE_AUTH_TOKEN`。`id-token: write` 用于生成 npm provenance；认证使用 `NPM_TOKEN`，无需配置 Trusted Publisher。token 必须支持无人值守发布，过期后更新同名 secret。

先完成这些设置，再推送首个发布 tag；npm 配置错误会留待发布任务报错，修复设置后重跑该失败任务即可。

## 已有安装迁移

Homebrew 保留原 tap 名称，修改下载仓库后继续升级：

```bash
brew tap --custom-remote sheng-ke-zhi/tap https://github.com/sheng-ke-zhi/skz-quant-cli
brew update
brew upgrade sheng-ke-zhi/tap/skz
```

Scoop 替换 bucket 后继续升级，已安装程序和配置保留：

```powershell
scoop bucket rm skz
scoop bucket add skz https://github.com/sheng-ke-zhi/skz-quant-cli
scoop update skz
```

原独立 tap/bucket 仓库不再自动更新。新安装命令见 README。

## 准备和推送版本

维护者从与 origin/main 同步的干净 main 工作树运行：

```bash
python3 scripts/release/release_wsl.py --check-only
python3 scripts/release/release_wsl.py
```

本地需 Git、uv/uvx 和 Cargo。脚本用 Commitizen 更新 PATCH、Cargo.lock 和 CHANGELOG，重新生成版本对应的插件，校验 annotated tag 指向最终提交，再原子推送 main 和 tag。`--check-only` 不 bump、不 push；准备完成但推送失败时用 `--resume` 重试推送，不会再次 bump。

也可以由维护者按同样契约准备版本并推送 annotated tag：Cargo.toml/Cargo.lock、CHANGELOG 和生成插件必须一致，tag 必须指向 main 中已通过 CI 的提交。不要用 force push 修复已公开 tag。自动化若用默认 `GITHUB_TOKEN` 推送 tag，通常不会触发另一个 workflow；此入口面向维护者本人的 Git 推送凭据。

## 失败恢复

在对应 Actions run 选择 **Re-run failed jobs**。成功的构建和打包任务保留，发布任务下载同一个 `release-payload`，按 `release-state.json` 核对 version、commit 和逐文件 SHA256 后继续：

- GitHub Release 已有文件校验一致时保留，只补传缺失文件；同名文件内容不同则停止，绝不 clobber。
- npm 跳过已存在的版本，先发五个原生包，再发主包；随后轮询可见性，并在临时目录全新安装、执行二进制及插件冒烟。
- Homebrew/Scoop 已同步时不产生新提交，失败时继续同步并核验公开下载。

发布由同一 concurrency group 串行执行。已有更新版本 tag 时，旧版本任务会停止，防止重跑时回退安装渠道。Artifact 超过 7 天、需改代码或改产物时，应修复后发新版本；不要选择全量重跑来替换已经上传的发行文件，也不要重新构建覆盖旧版。

各步骤日志及原始 payload 保存在 Actions；GitHub Release 本身保留正式下载文件。仓库工作流的更改不会主动发布，只有维护者推送版本 tag 才进入发行流程。
