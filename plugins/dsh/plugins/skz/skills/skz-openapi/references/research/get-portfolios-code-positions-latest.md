---
title: 组合最新目标持仓
description: 读取组合最新持仓与原始目标快照。
---
# 组合最新目标持仓

**`GET /research/portfolios/{code}/positions/latest`**

CLI：`skz portfolio latest-positions <code>`。返回 code 对应的 latest_weights、latest_weights_at、latest_weights_data_at、latest_weights_updated_at、position_snapshots，不读取回测绩效。请求带 Cache-Control: no-cache, no-store。

position_snapshots 包含 dt、data_at、updated_at、weights；保留真实数据时分秒及空仓，不把生成时间当成持仓数据时间。dt/data_at 和兼容文件名不换时区，updated_at 按事件时刻转北京时间。旧部署缺少新增字段时省略，不编造空快照。
