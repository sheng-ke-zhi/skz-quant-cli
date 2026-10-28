# 账户与授权

skz 将凭据保存为命名身份，通过 `auth use` 选择机器级默认身份。联网业务命令使用该身份；每条工作流开始前，应检查本机状态并联网确认实际账户：

```bash
skz auth status
skz whoami
```

## 浏览器 OAuth 登录

```bash
skz auth login personal --read-only
skz auth use personal
skz auth status
skz whoami
```

`auth login` 默认将授权链接输出到 stderr。手动打开链接，登录网页并点击“授权并继续”。CLI 换取并保存凭据后，浏览器跳转到授权成功页。登录只保存身份，不自动选择默认身份，随后需运行 `auth use`。

每次登录必须显式选择 `--read-only` 或 `--allow-write`。已有同名身份时，须加 `--replace`。例如，由用户重新登录并允许写入：

```bash
skz auth login personal --replace --allow-write
skz auth use personal
```

`--allow-write` 仅允许 CLI 发出写请求，实际权限仍由服务端决定；插件规定的付费与重要资产操作确认仍然适用。

### 浏览器、回调与等待时间

- 加 `--browser` 可自动打开浏览器。`--no-browser` 与默认行为一致，为兼容保留，不能和 `--browser` 同时使用。
- 浏览器须能访问运行 CLI 的电脑上的 `127.0.0.1` 回调。远程 SSH 环境需自行配置端口转发。
- 默认等待 180 秒，可用 `--timeout <秒>` 调整，范围为 1～600 秒。

### 登录状态与重新授权

`auth status` 只读本机文件，不联网验证凭据，也不触发续期。常用字段：

| 字段 | 含义 |
| --- | --- |
| `authType` | `oauth` 或 `api_key` |
| `expiresIn` | OAuth access token 的剩余秒数；API key 为 null |
| `loginRequired` | OAuth 刷新失败或中断后为 true，需要重新登录 |
| `readOnly` | 当前身份策略与全局只读开关共同决定的最终状态 |

OAuth 业务请求会在 access token 到期前 60 秒自动续期，并保存轮换后的 refresh token。多个 CLI 进程通过文件锁串行续期，不重放业务写请求。刷新失败或中断后，由用户重新执行 `auth login <identity> --replace` 并指定写策略。

凭据保存在受限权限文件中，stdout 只输出身份信息。`--account` 是本地账户标签，不代表后端核验的账户，实际归属用 `skz whoami` 确认。`auth remove <identity>` 仅删除本机身份，不撤销服务端授权。

## API key 与多账户

已有 API key 时，可以从 stdin 添加身份。下方 key 均为占位符；真实凭据不要粘贴到对话、日志或提交到仓库。

```bash
echo "sk_你的key" | skz auth add api-key --allow-write
skz auth use api-key
```

每把 key 保存为一个命名身份，再选择默认身份。切换本机默认身份不会自动发出业务请求：

```bash
echo "sk_alice的key"   | skz auth add alice   --read-only
echo "sk_bob的key"     | skz auth add bob     --allow-write
echo "sk_charlie的key" | skz auth add charlie --allow-write

skz auth list
skz auth use alice
skz auth status
```

`--read-only` 是 CLI 本地策略：alice 的读命令照常执行，业务写入和付费触发在发出目标请求前被拒绝。`--allow-write` 只表示 CLI 允许写，实际权限仍由 key 的后端 scope 决定。助手不能根据 bob、charlie 都可写就自行选择身份。

同一账户有多把不同权限的 key 时，使用不同身份名，并显式声明共同账户：

```bash
echo "sk_alice只读key" | skz auth add alice-read --account alice --read-only
echo "sk_alice写key"   | skz auth add alice-write --account alice --allow-write
```

## 全局只读模式

`SKZ_READ_ONLY=1` 对继承该环境变量的 skz 进程生效，即使当前身份为 `--allow-write`，也会阻止业务写入和付费触发。

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

确认 `auth status` 输出中的 `readOnly` 为 `true`。变量名写错会导致开关未生效，设置后应检查实际状态。

被限制的操作包括因子挖掘、策略探索、保存入库、组合创建、状态修改、删除、笔记和标签写入；命中限制后退出码为 8，不发出目标写请求。读命令和进度轮询照常；只读不承诺上游服务零内部副作用，例如已有任务的超时结算与退款。

需要关闭全局只读模式时，由用户移除当前终端中的环境变量：

```bash
unset SKZ_READ_ONLY
```

PowerShell：

```powershell
Remove-Item Env:SKZ_READ_ONLY
```

不要设成 `0` 或 `false`，CLI 会报参数错误。移除环境变量不会改变身份自身的只读策略；应再次用 `auth status` 检查最终状态。也可以在未设置该变量的另一个终端中，使用已获授权的可写身份操作。

本地策略用于防止误操作，凭据仍保存在本机；需要服务端强制限制时，应使用不带相应写权限的凭据。助手遇到 `readOnly:true` 或退出码 8 时，应停止写入并交给用户，不能自行切换身份、修改环境变量或绕过 CLI。

## OAuth 环境与协议细节

以下内容用于开发和排障，普通登录无需手工配置。

OAuth 配置从 `SKZ_BASE_URL` 所在 API 服务器的 `/.well-known/openid-configuration` 获取。生产发现结果中的官网旧授权地址会修正为 `https://quant.shengkezhi.com/oauth/authorize`。

token 请求固定使用当前 API 域名下的 `/oauth/token`，不采用发现文档可能返回的容器内网 token 地址；业务请求仍走 `/open/v1`。OAuth 身份绑定登录时的 API 根地址，不能跨生产、测试环境混用。公网 OAuth 地址要求 HTTPS，本机 mock 允许 HTTP。

有关助手的身份、授权和写后核验规则，见[运行契约](../plugin-src/common/references/operating-contract.md)。

## HTTPS 证书与响应大小

业务请求、OAuth 和 Windows 更新器使用系统证书验证策略，并保持证书链和主机名验证开启。在企业或云代理环境中，应将代理 CA 正确安装到系统信任库；精简 Linux 环境需安装并更新 CA 证书包。

业务成功响应的正文上限为 64 MiB，JSON 解析和输出还会额外占用内存。超过上限时，读命令返回 `internal`（退出码 6），不会自动重试；写命令返回 `check_existing`（退出码 7），应先按提示查询执行结果。OAuth、错误正文和更新元数据仍使用各自较小的读取上限。
