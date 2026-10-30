---
title: 删除因子路线
description: 胜可知开放平台 DELETE /research/factor-routes/{code}：删除因子路线。
source: https://docs.shengkezhi.com/api/research/delete-factor-routes-code
---
# 删除因子路线

**`DELETE /research/factor-routes/{code}`** — 删除因子路线。

需在请求头携带 `Authorization: Bearer sk_xxx`。

## 请求参数

| 参数 | 位置 | 类型 | 必填 | 默认值 | 说明 |
|---|---|---|:---:|---|---|
| `code` | path | string | 是 | `-` | 研究路线编码 |
| `force` | query | boolean | 否 | `-` | 越过两条软护栏：「名下仍有因子」与「挖掘执行目录最近仍有写入」。 |
| `dry_run` | query | boolean | 否 | `-` | 只预演不写：返回将删除的挖掘执行数与将永久删除的因子数（含已软删），不做任何修改。 |

## 响应 data

| 字段 | 类型 | 必填 | 说明 |
|---|---|:---:|---|
| `deleted` | boolean | 是 | 是否已实际删除；`dry_run=true` 时恒为 false。路线为物理删除，不做软删。 |
| `dry_run` | boolean | 是 | 本次是否为预演。 |
| `failed_mining_runs` | string[] | 是 | 未能删除的挖掘执行编号。完整实删回执中非空表示部分完成，不授权自动重发；须按[共同续作规则](../operating-contract.md#写操作重试与续作唯一规则)核验剩余影响并取得一次性明确续作授权，本工作流最多追加 1 次写调用，随后读回并停止。未知结果、核验失败或缺少授权时追加写调用为 0。 |
| `mining_runs` | integer | 是 | 挖掘执行数：预演时是「将删除」，实删时是「已成功删除」。 |
| `deleted_factors` | integer | 新版是 | 预演时预计、实删时实际永久删除的关联因子数量（含已软删）；旧响应缺失时 CLI 输出 null，数量未知，停止实删，不得按 0 或以 orphaned_factors 代替。 |
| `orphaned_factors` | integer | 是 | 兼容旧客户端；新版恒为 0，不能据此推断因子可保留。 |
| `route_code` | string | 是 | 被删除的研究路线编码，回显路径参数。 |

路线、全部关联因子、标签、评估与汇总在同一事务中物理删除，随后清理执行目录。先预演，展示身份、API 根地址、route_code、deleted_factors、mining_runs 和不可恢复范围，取得绑定本次影响的人工确认后实删。影响变化须重新确认。force 只越过护栏，不能保留因子。部分失败不会恢复已删因子，续作必须重新预演并遵循共同续作规则；未知结果不得重发。

兼容边界：user-research-backend #85 经 #89 合入 main（204aa5b7822d9aeeef7c01db4c344ff6169188de）；这不证明生产部署。不得根据旧文档承诺因子保留。

## 调用示例

```bash
curl -X DELETE "https://api.shengkezhi.com/open/v1/research/factor-routes/STRAT_MOMENTUM_001?dry_run=true" \
  -H "Authorization: Bearer sk_xxx"
```

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "deleted": false,
    "dry_run": true,
    "failed_mining_runs": [],
    "mining_runs": 1,
    "deleted_factors": 12,
    "orphaned_factors": 0,
    "route_code": "STS_BJ60MIN_LEADERS"
  }
}
```
