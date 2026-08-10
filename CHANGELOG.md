## v0.1.15 (2026-08-10)

### Feat

- **strategy**: sync latest positions contract
- **problem**: 接入研究问题物理删除接口

## v0.1.14 (2026-08-07)

### Feat

- **cli**: 分页默认每页 5 条

## v0.1.13 (2026-08-06)

### Feat

- **gift**: 接入策略赠予码五个接口
- **delete**: 接入删整次探索与删研究路线两个接口

## v0.1.12 (2026-08-06)

### Feat

- **config**: 用 SKZ_READ_ONLY 禁掉全部写与触发

## v0.1.11 (2026-08-04)

### Fix

- **problem**: 同步研究问题时间上限契约

## v0.1.10 (2026-08-03)

### Feat

- **update**: 支持 Homebrew 和 Scoop 自更新
- **strategy**: 支持多文件批量登记并移除 --realtime
- **config**: 支持通过环境变量切换服务器

### Fix

- **problem**: 按数据集条件校验 symbols 市场后缀

## v0.1.9 (2026-07-31)

### Feat

- **strategy**: 新增策略直接登记与用户笔记管理能力

## v0.1.8 (2026-07-30)

### Fix

- 前置校验静默失败的参数与付费任务引用

## v0.1.7 (2026-07-30)

### Feat

- 时间戳输出统一换算成东八区（契约 2.2 → 2.3）

### Fix

- 付费任务预检 route/problem/实盘候选与组合 code 冲突，固定枚举和挖掘筛选不再静默失败（契约 2.3 → 2.4）

## v0.1.6 (2026-07-30)

### Refactor

- 移除 workspace 命令面（工作区归主站开通，不属开放 API）

## v0.1.5 (2026-07-30)

## v0.1.4 (2026-07-28)

### Feat

- **experiment**: 支持删除未入库探索候选
- **release**: 自动同步 Homebrew 和 Scoop 包管理器分发 metadata

## v0.1.3 (2026-07-27)

### Fix

- **release**: 由本地发布脚本通过 gh 上传 GitHub Release 产物

## v0.1.2 (2026-07-27)

### Feat

- **release**: 自动发布到 PyPI 和 GitHub Release
- **update**: 新增 skz update 自更新子命令

### Fix

- **pypi**: wheel 脚本 zip 权限补 S_IFREG，修复 pip 装后无执行位

### Refactor

- **client**: 迁移 ureq 2→3.2


- edition 2024、MSRV 1.97.1，松绑 ureq 并规范技能对人话术

## v0.1.1 (2026-07-25)

### BREAKING CHANGE

- `skz skill *` → `skz skills *`,老名字 exit 2
fix_params(clap unrecognized subcommand)。上线前改,无兼容层。
- `skz skill show playbook` → `skz skill show guide`;
装出来的目录 `skz-playbook/` → `skz-guide/`,frontmatter `name` 同改。
装过旧版的机器需先 uninstall 旧目录(或手删),否则新版 install 会与
旧目录并存、旧的永不更新。

### Feat

- **portfolio**: 新增组合 CLI 与第四册 skill
- **skill**: 四家 harness adapter 全支持 + --target all
- **error**: 写的传输层错误标「结果未知」并给查证指引
- 技能套件（三册）+ skz skill 安装器 + 研究/实盘命令面
- 实现面向 agent 的胜可知开放平台 skz CLI

### Fix

- **contract**: 写超时归 check_existing(exit 7)，契约升 2.1
- **error**: 平台面 404/422 归 fix_params，别掉进 internal

### Refactor

- **cli**: 子命令 skill 改成 skills
- **skill**: 第三册 playbook 改名 guide
