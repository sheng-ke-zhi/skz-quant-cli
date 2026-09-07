---
title: 领取策略赠予
endpoint: POST /research/gifts/{gift_code}/claim
source: https://docs.shengkezhi.com/api/research/post-gifts-gift-code-claim
---

# 领取策略赠予

`POST /research/gifts/{gift_code}/claim` — 领取策略赠予。

实际调用地址：`https://api.shengkezhi.com/open/v1/research/gifts/{gift_code}/claim`。需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| gift_code | path | string | 是 | - | 32 位十六进制赠予码 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- | --- |
| from_user_id | string | 是 | 赠予方 user_id。 |
| items | GiftClaimItem[] | 是 | 逐条落地结果，顺序与码内策略一致。 |

### GiftClaimItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| origin_strategy_code | string | 是 | 赠予方侧的原编号。 |
| strategy_code | string | 是 | 落到本人实盘库里的编号；与原编号撞名且内容不同时带 `_G{n}` 后缀。 |
| inserted | boolean | 是 | 本次是否新写入；本库已有内容一致的同名策略时为 false。 |
| renamed | boolean | 是 | 是否因撞名而改了编号。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/gifts/GIFT-8K3M2P/claim" \
  -H "Authorization: Bearer sk_xxx"
```

响应示例：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "from_user_id": "a79dfc93b7e64a6cbbe26f2a787a6bad",
    "items": [
      {
        "origin_strategy_code": "STRAT_MOMENTUM_001",
        "strategy_code": "STRAT_MOMENTUM_001",
        "inserted": false,
        "renamed": false
      }
    ]
  }
}
```
