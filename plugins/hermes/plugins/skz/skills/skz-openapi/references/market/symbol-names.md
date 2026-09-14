---
title: 标的名称映射
description: 胜可知开放平台 GET /market/symbol-names：批量取得默认名称以及按市场隔离的标的名称映射。
source: https://docs.shengkezhi.com/api/market/symbol-names
---
# 标的名称映射

**`GET /market/symbol-names`** — 批量取得默认名称以及按市场隔离的标的名称映射。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应

返回 `names` 和 `markets` 两个映射：`names` 提供默认标的名称，`markets` 按市场隔离同名或同代码标的。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/market/symbol-names" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "names": {
    "000001.SZ": "平安银行"
  },
  "markets": {
    "stock": {
      "000001.SZ": "平安银行"
    }
  }
}
```
