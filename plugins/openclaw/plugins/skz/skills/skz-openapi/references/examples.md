---
title: 使用示例
source: https://docs.shengkezhi.com/api/examples
---

# 使用示例

以下示例均以 Base URL = `https://api.shengkezhi.com/open/v1`、认证头 `Authorization: Bearer sk_xxx` 为前提。

## curl

```bash
KEY="sk_xxx"     # 你的 Key
BASE="https://api.shengkezhi.com/open/v1"

# 1) 市场列表
curl "$BASE/market/markets" -H "Authorization: Bearer $KEY"

# 2) 标的查询
curl "$BASE/market/symbols?market=stock&keyword=平安&size=5" -H "Authorization: Bearer $KEY"

# 3) 交易日历
curl "$BASE/market/trading-calendar?exchange=SSE&start=2026-01-01&end=2026-01-31&onlyOpen=true" \
  -H "Authorization: Bearer $KEY"

# 4) 创建因子路线
curl -X POST "$BASE/strategy/routes" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "放量突破后的短期动量",
    "key_inspect": "标的在放量突破关键价位后，未来 5 个交易日内是否延续上涨动量",
    "economic_logic": "突破行为往往伴随资金共识形成",
    "why_effective": "放量突破释放了新的信息",
    "market_mechanism": "趋势跟踪",
    "failure_scenarios": ["市场整体处于持续下跌通道"]
  }'

# 5) 创建因子挖掘任务
curl -X POST "$BASE/strategy/miner/runs" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"routeCode":"RT_20260724_001"}'

# 6) 批量查询挖掘进度
runId=$(curl -s -X POST "$BASE/strategy/miner/runs" -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" -d '{"routeCode":"RT_20260724_001"}' | jq -r '.fcRunId')
curl -X POST "$BASE/strategy/miner/poll" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d "{\"runIds\":[\"$runId\"]}"
```

## Python（requests）

```python
import requests

KEY  = "sk_xxx"   # 你的 Key
BASE = "https://api.shengkezhi.com/open/v1"
H = {"Authorization": f"Bearer {KEY}"}

# 1) 数据集
markets = requests.get(f"{BASE}/market/markets", headers=H).json()
print(markets)   # [{'market': 'stock', 'count': 5464}, ...]

# 2) 搜标的
r = requests.get(f"{BASE}/market/symbols", headers=H,
                 params={"market": "stock", "keyword": "平安", "size": 5})
print(r.json()["items"])

# 3) 交易日历
r = requests.get(f"{BASE}/market/trading-calendar", headers=H,
                 params={"exchange": "SSE", "start": "2026-01-01", "end": "2026-01-31", "onlyOpen": "true"})
open_days = [d["calDate"] for d in r.json()]
print(len(open_days), "个交易日")

# 4) 创建因子路线
route = requests.post(f"{BASE}/strategy/routes", headers=H, json={
    "name": "放量突破后的短期动量",
    "key_inspect": "标的在放量突破关键价位后，未来 5 个交易日内是否延续上涨动量",
    "economic_logic": "突破行为往往伴随资金共识形成",
    "why_effective": "放量突破释放了新的信息",
    "market_mechanism": "趋势跟踪",
    "failure_scenarios": ["市场整体处于持续下跌通道"]
}).json()
route_code = route["routeCode"]

# 5) 创建因子挖掘任务
run = requests.post(f"{BASE}/strategy/miner/runs", headers=H,
                    json={"routeCode": route_code}).json()
print(run["fcRunId"], run["status"])

# 6) 批量查询挖掘进度
poll = requests.post(f"{BASE}/strategy/miner/poll", headers=H,
                     json={"runIds": [run["fcRunId"]]}).json()
print(poll)

# 错误处理示例
resp = requests.get(f"{BASE}/market/markets", headers=H)
if resp.status_code == 200:
    data = resp.json()
elif resp.status_code == 429:
    print("请求频率超限，请降低频率")      # RATE_LIMITED
elif resp.status_code == 403 and resp.json().get("errorCode") == "TOO_MANY_IPS":
    print("当日 IP 数超限")
else:
    print("错误:", resp.json())
```

## Node.js（fetch）

```javascript
const KEY  = "sk_xxx"
const BASE = "https://api.shengkezhi.com/open/v1"
const H = { headers: { Authorization: `Bearer ${KEY}` } }

const markets = await (await fetch(`${BASE}/market/markets`, H)).json()
const symbols = await (await fetch(`${BASE}/market/symbols?market=stock&keyword=平安&size=5`, H)).json()
console.log(markets, symbols.items)

// 创建因子路线与因子挖掘任务
const route = await (await fetch(`${BASE}/strategy/routes`, {
  ...H,
  method: 'POST',
  headers: { ...H.headers, 'Content-Type': 'application/json' },
  body: JSON.stringify({
    name: '放量突破后的短期动量',
    key_inspect: '标的在放量突破关键价位后，未来 5 个交易日内是否延续上涨动量',
    economic_logic: '突破行为往往伴随资金共识形成',
    why_effective: '放量突破释放了新的信息',
    market_mechanism: '趋势跟踪',
    failure_scenarios: ['市场整体处于持续下跌通道']
  })
})).json()

const run = await (await fetch(`${BASE}/strategy/miner/runs`, {
  ...H,
  method: 'POST',
  headers: { ...H.headers, 'Content-Type': 'application/json' },
  body: JSON.stringify({ routeCode: route.routeCode })
})).json()
console.log(route.routeCode, run.fcRunId, run.status)
```
