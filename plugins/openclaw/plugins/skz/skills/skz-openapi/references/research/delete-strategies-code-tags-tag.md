---
title: 删除实盘策略标签
endpoint: DELETE /research/strategies/{code}/tags/{tag}
source: https://docs.shengkezhi.com/api/research/delete-strategies-code-tags-tag
---

# 删除实盘策略标签

`DELETE /research/strategies/{code}/tags/{tag}` — 删除实盘策略标签。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/strategies/{code}/tags/{tag}`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 策略编号 |
| tag | path | string | 是 | - | 要删除的标签；路径段需 URL 编码 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code | string | 是 | 标签所属的策略编号（形如 TS_1D_4E70093D，即 前缀_周期_内容哈希）。 |
| tag | string | 是 | 本次新增或删除的策略标签文本。 |

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/strategies/STRAT_MOMENTUM_001/tags/%E9%87%8D%E7%82%B9%E5%85%B3%E6%B3%A8" \
  -H "Authorization: Bearer sk_xxx"
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
