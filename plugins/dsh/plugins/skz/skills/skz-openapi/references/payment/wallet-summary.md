---
title: 钱包概览
description: 胜可知开放平台 POST /payment/wallet/summary：查询当前 API Key 所属账户的钱包概览。
source: https://docs.shengkezhi.com/api/payment/wallet-summary
---
# 钱包概览

**`POST /payment/wallet/summary`** — 查询当前 API Key 所属账户的钱包概览。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 副作用与授权

只读查询：读取当前身份的钱包，不发起扣费，不因 POST 要求写授权。仍遵守 [运行契约](../operating-contract.md#直接-api-调用边界) 的身份与凭据边界，余额不代表写操作许可。

## 响应

该接口要求有效 API Key，但不额外要求业务 Scope。金额单位均为分；响应由计费服务原样返回，不使用 `{code,msg,data}` 信封。

## 调用示例

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/payment/wallet/summary" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "userId": 10086,
  "cash": {
    "accountId": 20001,
    "currency": "CNY",
    "balanceCent": 5000,
    "frozenCent": 0,
    "overdraftLimitCent": 0,
    "availableCent": 5000
  },
  "purses": [
    {
      "purseId": 30001,
      "purseType": "gift_credit",
      "balanceCent": 10000,
      "frozenCent": 0,
      "expireAt": "2026-12-31T23:59:59Z"
    }
  ],
  "totalAvailableCent": 15000
}
```
