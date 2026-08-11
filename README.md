# skz

胜可知量化平台的命令行工具。装好它，然后直接跟你的 AI 编程助手（Claude Code / Codex / openclaw / hermes）说人话——挖因子、跑策略探索、看实盘，它替你敲命令。

## 三步开工

```bash
# 1. 装（推荐用平台自带的包管理器；没装 Python 环境也能直接用）
# macOS / Linux（Homebrew）
brew install sheng-ke-zhi/tap/skz
# Windows（Scoop）
scoop bucket add skz https://github.com/sheng-ke-zhi/scoop-bucket
scoop install skz

# 或者用 Python 生态（PyPI）
uv tool install skz-quant-cli
pipx install skz-quant-cli

# 2. 装技能
skz skills install                    # 默认装给 Claude Code；其他 harness 用 --target all

# 3.（可选）配一个默认身份 —— 不配也行，agent 用到时会引导你补
echo "sk_你的key" | skz auth add personal --allow-write
skz auth use personal
```

无论通过 Homebrew、Scoop、uv tool 还是 pipx 安装，都用统一自更新：

```bash
skz update
```

它只按当前 `skz` 的实际安装路径选择渠道，并分别执行 `brew upgrade skz`、
`scoop update skz`、`uv tool upgrade skz-quant-cli` 或 `pipx upgrade skz-quant-cli`。

## 然后别自己敲命令，跟 agent 说

| 技能 | 干什么 | 你可以说 |
|---|---|---|
| `skz-guide` | 从一句想法带你走到能上实盘：聊想法 → 定研究方向 → 挖因子 → 定研究问题 → 策略探索 | 「帮我研究一个动量策略」 |
| `skz-factor` | 因子资产：因子库、某次挖矿的产出、单因子跨时段表现、软删不成立的 | 「看看我的因子库」 |
| `skz-strategy` | 策略资产：评审/删除候选、保存入库、实盘净值/持仓/回撤/交易明细、切换实盘·暂停·废弃 | 「我实盘最近怎么样」 |
| `skz-portfolio` | 组合资产：把实盘策略按权重打包、查净值/回撤/持仓、再平衡 | 「把这几个策略打包成一个组合」 |

**花钱的和不可逆的，agent 会先问你**——挖矿、策略探索、保存入库、切实盘、删因子、删候选、建组合，都得你点头才跑。

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

设了之后，即使当前身份是 `--allow-write`，所有写和触发（挖矿、探索、保存入库、建组合、改状态、删除、笔记标签）仍一律拒绝并退出码 8，**请求根本不会发出去**；读和进度轮询照常。

几件要知道的事：

- **`skz auth status` 那一步别省。** 变量名打错会静默变成「没设」，而你以为设上了。这是这类开关唯一的失效方式，也是唯一的检查手段。
- **关掉只能 `unset SKZ_READ_ONLY`。** 设成 `0` 或 `false` 会直接报错，不会当成关闭——否则一行 `SKZ_READ_ONLY=0 skz ...` 就绕过去了。
- **你自己也一样写不了。** 真要做一次写操作，在另一个没设这个变量的终端里跑。
- **这是防手滑，不是安全边界。** key 就在本机文件里，铁了心绕总有办法。它挡的是 agent 顺手闯祸，不是对抗性行为。要真正不可绕过，得让 key 的主人发一把不带写权限的 key。
