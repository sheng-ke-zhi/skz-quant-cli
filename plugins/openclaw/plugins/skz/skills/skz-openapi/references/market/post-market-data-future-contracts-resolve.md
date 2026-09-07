---
title: 批量解析期货当前合约
endpoint: POST /research/market-data/future-contracts/resolve
source: https://docs.shengkezhi.com/api/market/post-market-data-future-contracts-resolve
---

# 批量解析期货当前合约

`POST /research/market-data/future-contracts/resolve` — 批量解析期货当前合约。

该接口的主要用途是实盘合约换算：999 期货主力、888 期货指数这类平台规范标的并非交易所真实合约，无法直接下单；本接口将其解析为最新时点下可直接通过 CTP 实盘交易的交易所标准合约——999 主力映射为单一当前主力合约（权重 1），888 指数映射为一篮子成分合约及各自的 index_weight 权重。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| symbols | string[] | 是 | 待解析的规范 888/999 期货代码；每批 1 至 100 个，结果保持输入顺序 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| items | FutureContractResolution[] | 是 | 与请求 symbols 同序的逐项结果；重复输入会重复返回 |

### FutureContractResolution 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| symbol | string | 是 | 原始输入 symbol |
| contracts | FutureContractItem[] | 是 | 当前真实合约；合法但无映射时为空，999 至多一项，888 可为多项 |

### FutureContractItem 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| contract | string | 是 | CTP instrument id；大小写按交易所约定转换 |
| exchange | string | 是 | CTP 交易所代码，如 SHFE、CZCE、CFFEX |
| index_weight | number | 是 | 999 固定为 1；888 为最新指数成分权重，独立 half-up 保留四位小数 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/market-data/future-contracts/resolve" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"symbols":["RB999.SHF","RB888.SHF","MA999.ZCE","I999.DCE"]}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {
        "symbol": "RB999.SHF",
        "contracts": [
          { "contract": "rb2610", "exchange": "SHFE", "index_weight": 1.0 }
        ]
      },
      {
        "symbol": "RB888.SHF",
        "contracts": [
          { "contract": "rb2609", "exchange": "SHFE", "index_weight": 0.0062 },
          { "contract": "rb2610", "exchange": "SHFE", "index_weight": 0.5911 },
          { "contract": "rb2611", "exchange": "SHFE", "index_weight": 0.1139 },
          { "contract": "rb2612", "exchange": "SHFE", "index_weight": 0.0009 },
          { "contract": "rb2701", "exchange": "SHFE", "index_weight": 0.2418 },
          { "contract": "rb2702", "exchange": "SHFE", "index_weight": 0.0002 },
          { "contract": "rb2703", "exchange": "SHFE", "index_weight": 0.0359 },
          { "contract": "rb2704", "exchange": "SHFE", "index_weight": 0.0011 },
          { "contract": "rb2705", "exchange": "SHFE", "index_weight": 0.0088 },
          { "contract": "rb2706", "exchange": "SHFE", "index_weight": 0.0001 },
          { "contract": "rb2707", "exchange": "SHFE", "index_weight": 0.0001 },
          { "contract": "rb2708", "exchange": "SHFE", "index_weight": 0.0 }
        ]
      },
      {
        "symbol": "MA999.ZCE",
        "contracts": [
          { "contract": "MA2610", "exchange": "CZCE", "index_weight": 1.0 }
        ]
      },
      {
        "symbol": "I999.DCE",
        "contracts": [
          { "contract": "i2701", "exchange": "DCE", "index_weight": 1.0 }
        ]
      }
    ]
  }
}
```
