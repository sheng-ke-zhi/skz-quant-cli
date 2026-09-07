---
title: 组合绩效报告
endpoint: GET /research/portfolios/{code}/report
source: https://docs.shengkezhi.com/api/research/get-portfolios-code-report
---

# 组合绩效报告

`GET /research/portfolios/{code}/report` — 组合绩效报告。

完整地址：`GET https://api.shengkezhi.com/open/v1/research/portfolios/{code}/report`

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| code | path | string | 是 | - | 组合编号 |

## 响应

该接口不返回 JSON：直接返回完整的组合回测报告 HTML 页面（Content-Type: text/html，实测约 5 MB，含图表与交互脚本），可直接用浏览器打开。组合元数据与净值序列请用 `GET /research/portfolios/{code}`。

## 调用示例

```bash
curl -X GET "https://api.shengkezhi.com/open/v1/research/portfolios/STRAT_MOMENTUM_001/report" \
  -H "Authorization: Bearer sk_xxx"
```

```html
<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><title>FP01 组合回测报告</title></head><body><!-- 完整回测报告页：净值曲线、回撤、持仓与交易明细（约 5 MB） --></body></html>
```
