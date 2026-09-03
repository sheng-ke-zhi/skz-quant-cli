---
name: skz-wallet
description: 用 skz CLI 查询胜可知（Shengkezhi）账户的钱包余额、固定价目和指定付费操作是否负担得起。当用户问「资金余额」「钱包还有多少钱」「某项研究多少钱」「余额够不够」「还能挖掘或更新几次」时使用。不负责充值，也不触发因子挖掘、策略探索、实盘更新或保存入库。
---

# skz 技能 · wallet（资金与费用）

这册只负责回答账户资金和付费能力，不替用户做研究或资产操作。

## 使用前加载契约

执行任何 `skz` 命令前，完整读取 [references/operating-contract.md](references/operating-contract.md)。涉及费用、可负担性或付费动作时，完整读取 [references/billing.md](references/billing.md)。需要解释结果时，再读取 [references/communication.md](references/communication.md)。

七册分工：`skz-wallet` 负责资金和费用；`skz-guide` 负责研究导航、因子挖掘与策略探索；`skz-create-problem` 负责定义研究问题；`skz-factor` 负责因子资产；`skz-candidate` 负责实验、候选和保存入库；`skz-strategy` 负责已入库策略与实盘更新；`skz-portfolio` 负责组合。需要执行付费动作时切换到它的所属技能，本册不得代为触发。

## 命令

```bash
skz wallet balance
skz wallet costs
skz wallet check <mine|explore|refresh|save> [--qty <数量>]
```

- `balance` 返回现金、冻结、透支、额度钱包和 `totalAvailableCent`。回答“可用多少钱”使用合计可用额，不要只报现金余额。
- `costs` 是当前 CLI 固定价目，单位为分，`pricingSource` 为 `cli`；换算成人民币时除以 100。
- `check` 用合计可用额计算 `requiredCent`、`affordable` 和 `shortfallCent`。`affordable:false` 是一次成功的检查，不是接口故障。
- “还能做几次”先取对应 `unitPriceCent`，用 `totalAvailableCent / unitPriceCent` 向下取整；refresh/save 都按策略数计价。

充值不在 CLI 能力内。余额不足时只报告差额并引导用户到平台资金页，不猜充值链接、不代替用户支付。
