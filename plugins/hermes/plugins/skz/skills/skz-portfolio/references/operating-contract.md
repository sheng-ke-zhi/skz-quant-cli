# skz agent 运行契约

执行任何 `skz` CLI 命令前读本文件。它定义身份、授权、结构化 I/O、重试和写后确认规则。

## 身份与只读策略

每条工作流开工前运行：

```bash
skz auth status
skz whoami
```

- `active` 决定资产、余额和数据归属。不得根据读写权限自行切换身份。
- `readOnly:true` 表示当前身份或整台机器禁止写。停止所有写操作并交给用户，不得换 key、改环境变量或绕过 CLI。
- 没有 token、没有默认身份或缺 scope 时，按错误里的 `action` 和 `remediation` 处理。不得自行切换到权限更高的身份。
- token 只能由用户提供到受限凭据文件；不要在输出中展示 token。

## 人工确认边界

调用前必须获得用户明确确认的操作：

| 操作 | 原因 |
|---|---|
| `mine start` / `explore start` | 付费触发 |
| `promote start` | 付费、保存入库并消费候选 |
| `portfolio create` | 付费触发组合优化 |
| `strategy status --status 实盘` | 真金开始运行 |
| `strategy status --status 废弃` | 不可逆并进入写保护 |
| `strategy register` | 未经回测的策略直接入库 |
| `factor delete` | 对已有资产作逻辑处置 |
| `mining delete-run` | 永久删除单次挖掘产物 |
| `experiment delete` / `delete-run` | 永久删除候选或整次探索 |
| `factor-routes delete` | 永久删除路线并级联删除执行 |
| 删除时增加 `--force` | 越过后端软护栏，必须二次确认 |
| `problem delete` | 物理删除且不可恢复 |
| `gift create` / `gift claim` | 不可撤回披露或写入资产 |

以下操作可以自主执行：所有读命令；`factor-routes delete --dry-run`；`gift preview/list/revoke`；`route create`；`problem create`；切换到暂停；标签整理；正常写 memo。清空 memo 前先读回旧内容。

确认必须绑定本次实际参数。付费操作应先展示：核心假设、失败信号、预计耗时/扣费，再请求确认。验证计划可运行 `scripts/validate_plan.py`；脚本只校验，不执行写操作。

## 结构化 I/O

- 成功：stdout 是一份 JSON，exit 0。空数组或 `total:0` 是成功，不是错误。
- 失败：stderr 是 `{"error":{"kind","action",...}}`。按 `action` 和退出码分支，不解析自然语言 `message`。
- 异步查询 exit 0 只表示查询成功。任务结果必须看 body：`done:true` 是已结束，`ok:false` 才是任务失败。
- 分页默认通常只有 5 条。先看 `total`，需要时显式翻页，不得把首页当全集。
- 先查看真实 schema，再写 `jq` 投影。字段猜错时 `jq` 可能静默返回 `null`。
- 中文键使用 bracket 记法，例如 `jq '.["夏普比率"]'`。
- 枚举和筛选值写错可能只返回空结果；看到空结果先核对合法值和大小写。

退出码动作：

| exit | action | 下一步 |
|---|---|---|
| 2 | `fix_params` | 修正参数后重试 |
| 3 | `fix_auth` | 交给用户修复身份、token 或 scope |
| 4 | `retry` | 只重试幂等读；尊重 `retryAfter` |
| 5 | `retry_later` | 等待后重试读，不重放写 |
| 6 | `give_up` | 停止并报告，例如额度不足 |
| 7 | `check_existing` | 查询已有资源或任务；不要直接重触发 |
| 8 | `not_permitted` | 本机禁止写，停止并交给用户 |

## 重试与写后确认

- 读命令和 poll 可以有限重试。
- 所有写命令一律不自动重试，即便单个端点看似幂等。
- 写命令遇到超时或连接失败时，结果是“未知”，不是“失败”。先运行 `scripts/verify_write.py` 或错误中的 `remediation.verifyWith` 读回确认。
- 只有证实没有写入后才允许重试一次。付费写重试前必须重新确认。连续两次不确定就停止。

常见读回路径：

| 写操作 | 读回确认 |
|---|---|
| `route create` | `factor-routes list` |
| `problem create` / `delete` | `problem list` / `problem get` |
| `mine start` / `explore start` | 对应 `runs --status active` |
| `promote start` / `strategy register` | `strategy list`，有 promotion id 时再 `promote get` |
| `experiment delete` / `delete-run` | `experiment strategies` / `experiment list` |
| `factor-routes delete` | `factor-routes list` 加 `mining runs --route` |
| `mining delete-run` | `mining runs`，确认目标 run_id 已消失 |
| `gift create` / `revoke` | `gift list` |
| `gift claim` | `gift preview` 的 `already_claimed` |
| strategy status/tag/memo | `strategy get` |
| `portfolio create` / `refresh` | `portfolio list` / `portfolio get` 的 `has_performance` |

## 时间和原始标识

- 事件时刻已转换为东八区并带 `+08:00`；不要再加八小时。
- 交易日和区间边界字段原样使用，不做时区换算。
- `trades` 等松散块里的时间不保证转换。`kline_key` 是路径标识，必须逐字符原样传回。
- `skz --version` 输出 CLI 与 skill contract；`skz --help` 和子命令 `--help` 是命令真源。
