---
title: 组合持仓风险
description: 只读组合已生成的七项持仓风险指标。
---
# 组合持仓风险

**`GET /research/portfolios/{code}/position-risk`**

CLI：`skz portfolio position-risk <code> [--sdt YYYY-MM-DD] [--edt YYYY-MM-DD] [--granularity raw|daily]`。

只读已生成数据，不触发组合计算或扣费。sdt/edt 为包含边界的日期，granularity 省略时沿用后端默认 raw。

返回 `{available, rows}`；每行包含 dt、total_risk（总风险）、long_risk（多方风险）、short_risk（空方风险）、net_exposure（净敞口）、max_single_risk（最大单项风险）、herfindahl（集中度原值）、long_short_ratio（多空风险比）。指标保留 null，dt 保留源时间。总风险和 Herfindahl 原值不当百分比展示。

available=false 表示尚未生成；available=true 且 rows=[] 表示所选区间无数据，均不等于零风险。
