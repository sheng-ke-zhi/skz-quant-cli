---
title: 更新实盘策略备注
description: 胜可知开放平台 PATCH /research/strategies/{code}/memo：更新实盘策略备注。
source: https://docs.shengkezhi.com/api/research/patch-strategies-code-memo
---

# 更新实盘策略备注

**`PATCH /research/strategies/{code}/memo`** — 更新实盘策略备注。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 策略编号 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `memo` | string | 是 | 用户笔记；首尾空白会裁剪，最多 10000 个 Unicode 字符，空字符串表示清除。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `code` | string | 是 | 策略编号。 |
| `memo` | string | 是 | 归一化后的用户笔记；空字符串表示已清除。 |

## 调用示例

```bash
curl -X PATCH "https://api.shengkezhi.com/open/v1/research/strategies/STRAT_MOMENTUM_001/memo" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"memo":"示例备注：本策略用于文档演示。"}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "STS_BJ60MIN_LEADERS",
    "memo": "示例笔记：本策略用于文档演示。"
  }
}```
