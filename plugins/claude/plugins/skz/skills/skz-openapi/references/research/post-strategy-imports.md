---
title: 导入策略
description: 胜可知开放平台 POST /research/strategy-imports：导入策略。
source: https://docs.shengkezhi.com/api/research/post-strategy-imports
---

# 导入策略

**`POST /research/strategy-imports`** — 导入策略。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `tomls` | string[] | 是 | 完整、自包含的 UTF-8 策略 TOML 文本；每批最多 100 条。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `existing` | integer | 是 | 已存在、未重复登记的策略数。 |
| `inserted` | integer | 是 | 本次新登记的策略数。 |
| `items` | `StrategyImportItem`[] | 是 | 与请求 tomls 顺序一致的逐项结果。 |
| `total` | integer | 是 | 请求中的策略总数。 |

### StrategyImportItem 字段

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `strategy_code` | `string` | 是 | TOML 内声明的策略编号。 |
| `inserted` | `boolean` | 是 | 本次是否新写入 strategies.duckdb；同名已存在时为 false。 |
| `lifecycle` | `string` | 是 | 登记后的生命周期状态；新策略固定为“暂停”，已有策略返回其当前状态。 |
| `toml_sha256` | `string` | 是 | 上传内容的 SHA-256，供审计和客户端去重显示。 |

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/research/strategy-imports" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{"tomls":["示例Tomls"]}'
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "existing": 1,
    "inserted": 1,
    "items": [
      {
        "strategy_code": "STRAT_MOMENTUM_001",
        "inserted": false,
        "lifecycle": "示例Lifecycle",
        "toml_sha256": "示例Toml sha256"
      }
    ],
    "total": 1
  }
}```
