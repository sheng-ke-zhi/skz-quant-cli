---
title: 新增实盘策略标签
endpoint: POST /research/strategies/{code}/tags
source: https://docs.shengkezhi.com/api/research/post-strategies-code-tags
---

# 新增实盘策略标签

`POST /research/strategies/{code}/tags` — 新增实盘策略标签。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/strategies/{code}/tags`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| tag | string | 是 | 要为该策略新增的标签文本（与已有标签重复时返回 40903 标签重复）。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code | string | 是 | 标签所属的策略编号（形如 TS_1D_4E70093D，即 前缀_周期_内容哈希）。 |
| tag | string | 是 | 本次新增或删除的策略标签文本。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/strategies/STRAT_MOMENTUM_001/tags" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"tag": "重点关注"}'
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "STRAT_MOMENTUM_001",
    "tag": "重点关注"
  }
}
```
