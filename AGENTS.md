# AGENTS.md — 编码代理工作规范

面向在本仓库工作的所有编码代理。项目架构、I/O 契约与不变量见 [CLAUDE.md](CLAUDE.md)，本文只列协作铁律。

## 插件产物铁律

skills / 插件内容有且只有一个可编辑入口：

| 目录 | 角色 | 可否手改 |
| --- | --- | --- |
| `plugin-src/books/skz-*/` | 各技能正文（SKILL.md、agents/openai.yaml） | ✅ 编辑区 |
| `plugin-src/common/{scripts,references}/` | 全技能共享脚本与参考文档 | ✅ 编辑区 |
| `scripts/release/build_plugins.py` | 平台配置与共享 bundle 生成逻辑 | ✅ 编辑区 |
| `plugins/shared/skills/` | 唯一一套生成的 skills（8 skill，供 5 harness 共用） | ❌ 禁止手改 |
| `plugins/<harness>/` | 平台原生配置，不含 skills 副本；DSH 不需要配置文件 | ❌ 禁止手改 |

1. 改任何插件内容只改 `plugin-src/`，然后重新生成：
   ```bash
   python3 scripts/release/build_plugins.py --sync-only
   ```
2. 提交前自检（CI 跑同样的命令）：
   ```bash
   python3 scripts/release/build_plugins.py --check
   python3 tests/plugins/test_plugin_bundle.py -v
   ```
3. 手改 `plugins/` 会在下次 sync 时被静默覆盖，并使 `--check` 报 stale 拒绝构建。
4. `plugins/**` 已在 `.gitattributes` 标记为 generated：GitHub 折叠其 diff、不计入语言统计，评审请聚焦 `plugin-src/`。

## 平台差异怎么写

平台差异只放在生成脚本的原生配置中。skills 正文、脚本与参考资料统一修改 `plugin-src/`，不支持 `targets/<harness>/` 的 skill 覆盖；生成器会拒绝此类文件，避免重新引入分叉副本。

发布 bundle 将 skills 仅放在 `shared/skills/`，manifest 对共享内容记录一次 SHA256/mode。安装器为每个 harness 将共享内容映射到原生插件的 `plugins/skz/skills/`。安装缓存保持自包含，原生管理器无需支持跨插件目录引用或符号链接；不把这些缓存视为新的作者源。

## 版本发布

发版是维护者专属操作（`python3 scripts/release/release_wsl.py`：PATCH bump、tests、五平台构建、打包、publish）。普通贡献者与代理不得执行；仅当维护者明确要求时，自动化代理方可运行。禁止 force push。
