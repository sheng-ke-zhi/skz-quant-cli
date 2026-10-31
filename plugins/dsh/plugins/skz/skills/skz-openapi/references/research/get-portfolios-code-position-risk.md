---
title: 组合持仓风险
description: 读取组合原始或日末持仓风险时间序列。
---
# 组合持仓风险

**`GET /research/portfolios/{code}/position-risk`**

CLI：`skz portfolio position-risk <code> [--sdt YYYY-MM-DD] [--edt YYYY-MM-DD] [--granularity raw|daily]`。

只读当前身份的组合风险数据，不触发组合计算或扣费。直接 API 调用遵循 [运行契约](../operating-contract.md#直接-api-调用边界)。风险产物缺失不代表已授权刷新组合。

`code` 是必填路径参数；可选查询参数 `sdt`、`edt` 包含起止当天，省略时不限日期；`granularity` 默认 `raw`，也可选 `daily` 取每天最后一个真实时间点。两种粒度都保留源数据时点，不把日末改写为零点。

成功返回标准 `{code,msg,data}` 信封，`data` 为 `{available,rows}`。每行包含 `dt`、`total_risk`、`long_risk`、`short_risk`、`net_exposure`、`max_single_risk`、`herfindahl`、`long_short_ratio`。CLI 解包输出 data。

七项指标为原始可空数值，不裁剪、不归一化。`total_risk` 是绝对权重敞口之和，`herfindahl` 是权重平方和原值，可超过 1；它们不直接代表资金风险率。`null` 表示指标不可用，不能补成零。源时间字符串原样保留。

产物未生成时 `available=false,rows=[]`；有效空文件或筛选无数据时 `available=true,rows=[]`。编号/日期/粒度无效返回 400，组合不存在返回 404，产物读取失败返回 500，读取限流返回 429/503。沿用只读重试，不能因缺失产物自动调用组合刷新。

后端使用短时缓存；组合更新后失效，并识别已发布文件的变化。HTTP 响应禁止浏览器和代理存储。
