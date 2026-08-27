# AGENTS.md — 编码代理工作规范

面向在本仓库工作的所有编码代理。项目架构、I/O 契约与不变量见 [CLAUDE.md](CLAUDE.md)，本文只列协作铁律。

## 插件产物铁律

skills / 插件内容有且只有一个可编辑入口：

| 目录 | 角色 | 可否手改 |
| --- | --- | --- |
| `plugin-src/books/skz-*/` | 各技能正文（SKILL.md、agents/openai.yaml） | ✅ 编辑区 |
| `plugin-src/common/{scripts,references}/` | 全技能共享脚本与参考文档 | ✅ 编辑区 |
| `plugin-src/targets/<harness>/` | 平台差异覆盖（按需创建，见下） | ✅ 编辑区 |
| `plugins/` | 以上内容的机器生成物（4 harness × 6 skill） | ❌ 禁止手改 |

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

某平台需要不同的或独有的文件时，在 `plugin-src/targets/<harness>/` 下**镜像原稿相对路径**放置覆盖文件即可；渲染时盖在默认内容上，其他平台不受影响。例如给 codex 一份专属的 factor 技能 yaml：

```
plugin-src/targets/codex/books/skz-factor/agents/openai.yaml
```

还可以放到 `targets/<harness>/common/{references,scripts}/…` 覆盖共享文件，或放入默认版本中不存在的新文件（只会出现在该平台的产物里）。

注意：现有测试 `test_rendered_targets_are_identical_and_self_contained` 假设四个平台产物字节一致；首次引入真实覆盖内容时需同步放宽该断言。

## 版本发布

发版是维护者专属操作（`python3 scripts/release/release_wsl.py`：PATCH bump、tests、五平台构建、打包、publish）。普通贡献者与代理不得执行；仅当维护者明确要求时，自动化代理方可运行。禁止 force push。
