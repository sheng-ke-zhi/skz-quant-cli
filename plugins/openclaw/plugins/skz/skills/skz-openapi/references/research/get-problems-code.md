---
title: 研究问题详情
endpoint: GET /research/problems/{code}
source: https://docs.shengkezhi.com/api/research/get-problems-code
---

# 研究问题详情

`GET /research/problems/{code}` — 研究问题详情。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/problems/{code}`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 研究问题编码 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code | string | 是 | 研究问题编号，后端按内容哈希生成，格式 前缀_PROBLEM_HASH。 |
| dataset | string | 是 | 数据源，future、etf 或 stock。 |
| description | string | 是 | 研究问题描述。 |
| editable | boolean | 是 | 是否可删除；workspace 中的问题均为 true。 |
| freq | string | 是 | K 线周期，如 日线、60分钟 等。 |
| name | string | 是 | 研究问题名称。 |
| problem_type | string | 是 | 研究问题类型枚举，TimeSeriesProblem 或 CrossSectionalProblem。 |
| source | string | 是 | 数据来源，builtin（预置）或 user（用户自定义）。 |
| symbols | string[] | 是 | 合约或标的代码列表。 |
| time_segments | Seg[] \| null | 否 | 时间段划分（训练与验证区间）；仅详情接口返回，列表项省略该字段。 |
| type_label | string | 是 | 问题类型中文标签，如 时序择时 或 截面多空。 |

### Seg 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| name | string | 是 | 时间段名称，取自必需段之一，如 训练集A段、训练集、后置验证。 |
| sdt | string | 是 | 时间段起始日期，格式 YYYYMMDD，不得晚于 20250701。 |
| edt | string | 是 | 时间段结束日期，格式 YYYYMMDD，不得晚于 20250701。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/problems/STS_QS_LEADERS" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "code": "STS_QS_LEADERS",
    "name": "券商龙头时序研究",
    "problem_type": "TimeSeriesProblem",
    "type_label": "时序择时",
    "description": "A 股券商板块龙头：中信证券，国泰海通，东方财富，中信建投，华泰证券",
    "freq": "日线",
    "dataset": "stock",
    "symbols": ["600030.SH", "601211.SH", "601066.SH"],
    "source": "user",
    "editable": true,
    "time_segments": [
      { "name": "训练集A段", "sdt": "20170101", "edt": "20190101" },
      { "name": "训练集B段", "sdt": "20190101", "edt": "20210101" },
      { "name": "训练集C段", "sdt": "20210101", "edt": "20230101" }
    ]
  }
}
```
