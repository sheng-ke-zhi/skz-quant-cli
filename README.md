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

# 3.（可选）配 key —— 不配也行，agent 用到时会引导你补
echo "sk_你的key" | skz auth set
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
