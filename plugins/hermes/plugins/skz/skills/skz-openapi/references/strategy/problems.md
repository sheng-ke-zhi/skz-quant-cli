---
title: 创建研究问题
description: 胜可知开放平台 POST /strategy/problems：创建研究问题并返回问题编码。
source: https://docs.shengkezhi.com/api/strategy/problems
---
# 创建研究问题

**`POST /strategy/problems`** — 创建研究问题，返回问题编码。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求体

研究问题常见字段如下：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | string | 是 | 问题名称 |
| `problem_type` | string | 是 | `TimeSeriesProblem` 或 `CrossSectionalProblem`；不接受旧字段 `type` |
| `description` | string | 是 | 研究问题说明 |
| `freq` | string | 是 | `15分钟` / `60分钟` / `120分钟` / `240分钟` / `日线` |
| `dataset` | string | 是 | `stock` / `etf` / `future` |
| `symbols` | array[string] | 视类型 | 时序问题必填；截面问题至少 10 个标的 |
| `time_segments` | array[object] | 是 | 必须包含 5 个规定时间段；日期格式 `YYYYMMDD` 且不得晚于 `20250701` |

:::info 字段说明
具体字段与校验规则以接口返回为准。若字段不满足要求，将返回 `400` 与对应错误信息。
:::

## 响应

成功返回 HTTP 201：`{code:0,msg,data:{code}}`，其中 `data.code` 为按内容生成的稳定问题编码。

## 实测

```bash
curl -X POST "https://api.shengkezhi.com/open/v1/strategy/problems" \
  -H "Authorization: Bearer sk_xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "银行股短期动量研究",
    "problem_type": "TimeSeriesProblem",
    "description": "研究银行股日线动量",
    "freq": "日线",
    "dataset": "stock",
    "symbols": ["000001.SZ"],
    "time_segments": [
      {"name":"训练集A段","sdt":"20170101","edt":"20190101"},
      {"name":"训练集B段","sdt":"20190101","edt":"20210101"},
      {"name":"训练集C段","sdt":"20210101","edt":"20230101"},
      {"name":"训练集","sdt":"20170101","edt":"20230101"},
      {"name":"后置验证","sdt":"20230101","edt":"20250101"}
    ]
  }'
```

```json
{"code":0,"msg":"创建成功","data":{"code":"STS_PROBLEM_4A18F0D2"}}
```
