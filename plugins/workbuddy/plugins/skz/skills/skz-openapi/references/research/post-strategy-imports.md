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
  -d '{"tomls":["strategy = \"STS_DOCS_IMPORT_001\"\npost_process = \"WEIGHT\"\nroute = \"docs_route\"\nfactors = []\n\n[problem]\nproblem_type = \"TimeSeriesProblem\"\nname = \"开放平台导入示例\"\ncode = \"STS_IMPORTED\"\ndescription = \"docs import\"\nfreq = \"日线\"\ndataset = \"stock\"\nsymbols = [\"600000.SH\"]\ntime_segments = [{ name = \"训练集\", sdt = \"20200101\", edt = \"20230101\" }, { name = \"后置验证\", sdt = \"20230101\", edt = \"20240101\" }]\n\n[runtime]\nfactor_failure_policy = \"skip\"\nupdate_mode = \"auto\"\nincremental_lookback_bars = 3000\n\n[model_config]\nname = \"TS002\"\nmodel = \"TS002\"\nkwargs = {}"]}'
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
        "strategy_code": "STS_DOCS_IMPORT_001",
        "inserted": true,
        "lifecycle": "暂停",
        "toml_sha256": "808bf0f1dff794ec86bb0bd12e1c213eea9d7ba4da41d744510a9306d45641e6"
      }
    ],
    "total": 1
  }
}```
