---
title: 批量软删除因子
description: 胜可知开放平台 DELETE /research/factors：批量软删除并返回逐项结果。
---

# 批量软删除因子

**`DELETE /research/factors`**，请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `factor_names` | string[] | 是 | 原始数量 1–1000；重复名称按首次出现顺序去重。 |
| `reason` | string | 否 | 共用删除理由，省略为空字符串。 |

逐项独立写入，某项失败不会回滚已成功项。已软删除因子再次处理会更新理由并返回成功。

## 响应 data

| 字段 | 类型 | 说明 |
|---|---|---|
| `items` | object[] | 去重后按首次出现顺序排列的逐项结果。 |
| `items[].factor_name` | string | 因子名称。 |
| `items[].success` | boolean | 此项是否成功。 |
| `items[].code` | integer | 成功为 0，失败为现有 API 错误码，例如不存在为 40400。 |
| `items[].msg` | string | 此项的结果描述。 |
| `succeeded_count` | integer | 成功项数。 |
| `failed_count` | integer | 失败项数。 |

**部分或全部项目失败仍返回 HTTP 200、信封 code 0；必须检查逐项结果。** 请求体或数量无效返回 400；因子库或表不存在返回 404；写锁忙返回 409，锁不可用返回 503，批量写准备失败返回 500。传输失败时结果未知，先逐项查询因子详情核对删除标记和理由，不自动重试。

## CLI 示例

```bash
skz factor delete-batch < reviewed-factors.json
```

输入文件内容：

```json
{"factor_names":["FT_1","missing"],"reason":"逻辑重复"}
```

API 响应示例（CLI 只输出 data，exit 0）：

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "items": [
      {"factor_name":"FT_1","success":true,"code":0,"msg":"已软删除"},
      {"factor_name":"missing","success":false,"code":40400,"msg":"not found"}
    ],
    "succeeded_count": 1,
    "failed_count": 1
  }
}
```
