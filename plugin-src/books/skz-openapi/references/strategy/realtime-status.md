---
title: 更新并同步实盘策略状态
description: 胜可知开放平台 PATCH /strategy/realtime/strategies/{code}/status：更新策略状态，并同步后台实盘镜像。
source: https://docs.shengkezhi.com/api/strategy/realtime-status
---
# 更新并同步实盘策略状态

**`PATCH /strategy/realtime/strategies/{code}/status`** — 更新策略状态，并同步后台实盘镜像。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

透传投研后端的 `{code,msg,data}` 信封；成功时 `data` 包含策略编码和更新后的状态。

## 调用示例

```bash
curl -X PATCH "https://api.shengkezhi.com/open/v1/strategy/realtime/strategies/STS_001/status" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"status":"暂停"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "STS_001",
    "status": "暂停"
  }
}
```
