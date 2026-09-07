---
title: 研究问题元数据
endpoint: GET /research/problems/meta
source: https://docs.shengkezhi.com/api/research/get-problems-meta
---

# 研究问题元数据

`GET /research/problems/meta` — 研究问题元数据。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/problems/meta`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| code_gen | CodeGen | 是 | code 生成规则元数据（供前端实时预览）。 |
| dataset_options | LabeledOption[] | 是 | 可选数据源选项。 |
| default_time_segments | Seg[] | 是 | 创建表单默认填充的时间段划分。 |
| freq_options | LabeledOption[] | 是 | 可选 K 线周期选项。 |
| max_time_segment_date | string | 是 | 创建研究问题时允许使用的最晚时间段日期，格式 YYYYMMDD。 |
| problem_types | ProblemTypeOption[] | 是 | 可选研究问题类型选项（驱动创建表单的类型切换与字段联动）。 |
| required_segments | string[] | 是 | 必需时间段名称清单（缺任一即校验失败）。 |

### CodeGen 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| pattern | string | 是 | code 生成模板，形如 前缀_PROBLEM_HASH。 |
| prefix_rule | string | 是 | 前缀拼接规则说明：dataset 首字母加类型缩写。 |
| note | string | 是 | 面向前端的补充说明（code 由后端生成，前端仅做预览）。 |
| dataset_letter | object（动态键，值为 string） | 是 | dataset 到首字母的映射，如 future 到 F。 |
| type_abbr | object（动态键，值为 string） | 是 | 问题类型到缩写的映射，如 TimeSeriesProblem 到 TS。 |
| valid_prefixes | string[] | 是 | 全部合法前缀集合，共 6 个，如 STS、FCS。 |

### LabeledOption 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| value | string | 是 | 选项取值（提交给后端的值）。 |
| label | string | 是 | 选项显示文案。 |

### Seg 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| name | string | 是 | 时间段名称，取自必需段之一，如 训练集A段、训练集、后置验证。 |
| sdt | string | 是 | 时间段起始日期，格式 YYYYMMDD，不得晚于 20250701。 |
| edt | string | 是 | 时间段结束日期，格式 YYYYMMDD，不得晚于 20250701。 |

### ProblemTypeOption 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| value | string | 是 | 问题类型取值，如 TimeSeriesProblem 或 CrossSectionalProblem。 |
| label | string | 是 | 问题类型中文标签，如 时序择时 或 截面多空。 |
| id_field | string | 是 | 该类型对应的标识输入字段名，当前均为 symbols。 |
| id_label | string | 是 | 标识字段的表单显示名，如 合约/标的代码。 |
| id_hint | string | 是 | 标识字段的填写提示（数量与选取建议）。 |

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/problems/meta" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "problem_types": [
      {
        "value": "TimeSeriesProblem",
        "label": "时序择时",
        "id_field": "symbols",
        "id_label": "合约/标的代码",
        "id_hint": "一般不超过 10 个，应为高相关品种"
      },
      {
        "value": "CrossSectionalProblem",
        "label": "截面多空",
        "id_field": "symbols",
        "id_label": "合约/标的代码",
        "id_hint": "最少 10 个标的，因子值需横截面可比"
      }
    ],
    "freq_options": [
      { "value": "15分钟", "label": "15分钟" },
      { "value": "60分钟", "label": "60分钟" },
      { "value": "120分钟", "label": "120分钟" }
    ],
    "dataset_options": [
      { "value": "future", "label": "future · 期货主力" },
      { "value": "etf", "label": "etf · ETF" },
      { "value": "stock", "label": "stock · 股票日线" }
    ],
    "required_segments": ["训练集A段", "训练集B段", "训练集C段"],
    "default_time_segments": [
      { "name": "训练集A段", "sdt": "20170101", "edt": "20190101" },
      { "name": "训练集B段", "sdt": "20190101", "edt": "20210101" },
      { "name": "训练集C段", "sdt": "20210101", "edt": "20230101" }
    ],
    "max_time_segment_date": "20250701",
    "code_gen": {
      "pattern": "{prefix}_PROBLEM_{HASH}",
      "prefix_rule": "prefix = dataset首字母 + 类型缩写",
      "note": "code 由后端自动生成，前端仅做预览；HASH 为语义字段内容哈希前 6 位大写 HEX（碰撞延长）",
      "dataset_letter": { "etf": "E", "future": "F", "stock": "S" },
      "type_abbr": { "CrossSectionalProblem": "CS", "TimeSeriesProblem": "TS" },
      "valid_prefixes": ["FTS", "ETS"]
    }
  }
}
```
