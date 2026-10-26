#!/usr/bin/env python3
"""通用、默认只读的 skz 全流程 driver；状态和预算一律失败关闭。"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from html.parser import HTMLParser

ACTIVE = {'queued', 'dispatching', 'running'}
EXECUTION_CODES = {'TASK_EXECUTION_FAILED', 'EXECUTION_FAILED', 'SKZ_EXECUTION_ERROR'}
ROUTE_FIELDS = {'name', 'key_inspect', 'economic_logic', 'why_effective',
                'market_mechanism', 'failure_scenarios', 'tags'}
MECHANISMS = {'错误定价', '风险补偿', '行为偏差', '流动性溢价', '制度性套利',
              '自我实现预言', '微观结构', '信息扩散', '趋势跟踪', '套保压力'}
TZ = timezone(timedelta(hours=8))


class Pause(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise Pause(message)


def now():
    return datetime.now(TZ).isoformat(timespec='seconds')


def digest(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True, ensure_ascii=False,
                                     separators=(',', ':')).encode()).hexdigest()


def load(path, default=None):
    p = Path(path)
    return json.loads(p.read_text(encoding='utf-8')) if p.exists() else default


def save(path, obj):
    p = Path(path)
    tmp = p.with_suffix(p.suffix + '.tmp')
    with tmp.open('w', encoding='utf-8') as f:
        json.dump(obj, f, ensure_ascii=False, indent=2)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, p)
    fd = os.open(str(p.parent), os.O_RDONLY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def rows(d):
    if isinstance(d, list):
        result = d
    elif isinstance(d, dict) and isinstance(d.get('items'), list):
        result = d['items']
    else:
        raise Pause('列表响应结构未知，不能按空清单继续')
    require(all(isinstance(r, dict) for r in result), '列表包含非对象元素')
    return result


def validate_config(c):
    require(isinstance(c, dict) and c.get('scenario') in ('A', 'B'), 'scenario 必须为 A/B')
    require(type(c.get('max_total_cent')) is int and c['max_total_cent'] > 0, '必须指定正整数预算分')
    problems = c.get('problems')
    require(isinstance(problems, list) and problems and all(isinstance(x, str) and x for x in problems)
            and len(set(problems)) == len(problems), 'problems 必须为不重复的已确认 code 列表')
    if c['scenario'] == 'A':
        require(type(c.get('times')) is int and c['times'] > 0, 'times 必须为正整数')
        require(type(c.get('threshold')) is int and c['threshold'] >= 0, 'threshold 必须为非负整数')
        require(bool(c.get('route')) != bool(c.get('route_json')), '场景 A：route 与 route_json 二选一')
        if c.get('route_json'):
            r = c['route_json']
            require(isinstance(r, dict), 'route_json 必须为对象')
            forbidden = set(r) & {'describe', 'description'}
            require(not forbidden, f'本 driver 的路线创建字段没有 describe/description 字段；'
                    f'请移除 {"、".join(sorted(forbidden))}，研究要求写入 key_inspect')
            require(ROUTE_FIELDS <= r.keys(), 'route_json 缺少七字段')
            require(set(r) <= ROUTE_FIELDS | {'compute_engine'},
                    'route_json 只允许七个核心字段及可选 compute_engine，含未知字段')
            require(all(isinstance(r[k], str) and r[k].strip() for k in
                        ('name', 'key_inspect', 'economic_logic', 'why_effective')), '路线文本不能为空')
            require(r['market_mechanism'] in MECHANISMS, 'market_mechanism 枚举非法')
            for k in ('failure_scenarios', 'tags'):
                require(isinstance(r[k], list) and r[k] and all(isinstance(x, str) and x for x in r[k]),
                        f'{k} 必须为非空字符串数组')
    else:
        require(isinstance(c.get('routes'), list) and c['routes'],
                '场景 B：至少一条 routes × 至少一个 problems，逐组合探索')
        require(not any(k in c for k in ('route_json', 'route', 'times', 'threshold')),
                '场景 B 只复用既有路线探索，不创建路线、不挖掘')
        qty = len(c['routes']) * len(problems)
        if 'max_submissions' in c:
            require(type(c['max_submissions']) is int and c['max_submissions'] == qty,
                    'max_submissions 必须等于已确认路线×问题组合数量')
    if 'expected_unit_prices' in c:
        prices = c['expected_unit_prices']
        require(isinstance(prices, dict) and prices and set(prices) <= {'mine', 'explore'}
                and all(type(v) is int and v > 0 for v in prices.values()), '预期单价必须为正整数分')
    rr = c.get('routes') if c['scenario'] == 'B' else ([c['route']] if c.get('route') else [])
    require(all(isinstance(x, str) and x for x in rr) and len(set(rr)) == len(rr), '路线 code 非法或重复')
    require(type(c.get('poll_seconds', 60)) in (int, float) and c.get('poll_seconds', 60) > 0,
            'poll_seconds 必须大于零')
    require(type(c.get('max_wait_minutes', 1500)) in (int, float)
            and c.get('max_wait_minutes', 1500) > 0, '等待窗口必须大于零')
    require(type(c.get('batch_size', 8)) is int and 1 <= c.get('batch_size', 8) <= 8,
            'batch_size 必须为 1..8')
    require(c.get('theme', 'scarlet') in ('scarlet', 'crimson', 'vermilion', 'coral', 'brick'), '未知报告主题')


class CLI:
    def __init__(self, wd, binary):
        self.wd, self.binary = wd, binary

    def call(self, args, *, write=False, body=None, receipt=None):
        # 临时读故障由 CLI 自带的重试处理，外层重放会放大请求且绕过 action 分类。
        path = receipt if write else self.wd / 'raw' / f'{time.time_ns()}-read.json'
        raw = {'at': now(), 'args': args, 'write': write}
        try:
            p = subprocess.run([self.binary, *args],
                               input=json.dumps(body, ensure_ascii=False) if body is not None else None,
                               capture_output=True, text=True, timeout=180)
            raw.update(returncode=p.returncode, stdout=p.stdout, stderr=p.stderr)
            save(path, raw)
            require(p.returncode == 0, f'CLI exit={p.returncode}，见回执 {path}')
            d = json.loads(p.stdout)
            require(not (isinstance(d, dict) and d.get('error')), f'CLI error，见回执 {path}')
            return d
        except (OSError, subprocess.TimeoutExpired, ValueError, Pause) as e:
            raw['exception'] = repr(e)
            for key in ('stdout', 'stderr'):
                value = getattr(e, key, None)
                if value is not None:
                    raw[key] = value.decode(errors='replace') if isinstance(value, bytes) else value
            save(path, raw)
            raise Pause(f'CLI 未确认成功；禁止盲重发，见 {path}') from e


class ReportParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.links, self.scripts = set(), [], 0
        self.duplicate_ids = []
        self.nav = self.main = self.footer = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get('id'):
            if attrs['id'] in self.ids:
                self.duplicate_ids.append(attrs['id'])
            self.ids.add(attrs['id'])
        if tag == 'a' and attrs.get('href', '').startswith('#'):
            self.links.append(attrs['href'][1:])
        if tag == 'script':
            self.scripts += 1
        self.nav |= tag == 'nav'
        self.main |= tag == 'main'
        self.footer |= tag == 'footer'


def verify_report(path):
    p = Path(path)
    raw = p.read_bytes()
    text = raw.decode('utf-8')
    require(raw.startswith(b'<!DOCTYPE html>') and b'charset="utf-8"' in raw[:1024], '报告编码/doctype 不合格')
    parser = ReportParser()
    parser.feed(text)
    require(parser.nav and parser.main and parser.footer and not parser.duplicate_ids
            and {f's{i}' for i in range(1, 9)} <= parser.ids
            and all(x in parser.ids for x in parser.links) and text.rstrip().endswith('</html>'),
            '报告缺少完整章节/目录/尾部')
    require(parser.scripts == 0, '报告新增脚本，需要额外运行时验证，不能仅做静态检查')
    return hashlib.sha256(raw).hexdigest()


class Pipeline:
    def __init__(self, args, cli=None):
        self.a = args
        self.wd = Path(args.workdir).resolve()
        self.c = load(args.config)
        validate_config(self.c)
        self.hash = digest(self.c)
        self.cli = cli or CLI(self.wd, args.skz_bin)
        self.st = load(self.wd / 'pipeline_state.json')
        if self.st is None:
            require(not any((self.wd / n).exists() for n in ('mine_seq.json', 'explore_batch.json', 'route_intent.json')),
                    '发现旧工作目录账本，禁止静默迁移；使用独立目录并人工核验接管')
            self.st = {'schema': 1, 'config_digest': self.hash, 'mine': [], 'explore': [],
                       'route_intent': None, 'routes': [], 'phase': 'preflight',
                       'mine_complete': self.c['scenario'] == 'B', 'mine_stop_reason': None,
                       'prices': {}, 'created_at': now()}
        require(self.st.get('schema') == 1 and self.st.get('config_digest') == self.hash,
                '账本版本/配置摘要不同，禁止继续写入')

    def call(self, args, **kwargs):
        return self.cli.call(args, **kwargs)

    def persist(self):
        self.st['updated_at'] = now()
        save(self.wd / 'pipeline_state.json', self.st)

    def event(self, name, **data):
        event = {'at': now(), 'event': name, 'config_digest': self.hash, **data}
        line = json.dumps(event, ensure_ascii=False)
        with (self.wd / 'events.ndjson').open('a', encoding='utf-8') as f:
            f.write(line + '\n')
            f.flush()
            os.fsync(f.fileno())
        print(line, flush=True)

    def ledger(self):
        return [a for k in ('mine', 'explore') for t in self.st[k] for a in t['attempts']]

    def tasks(self, kind, active=False):
        result, page = [], 1
        while True:
            args = [kind, 'runs', '--page', str(page), '--size', '100']
            if active:
                args += ['--status', 'active']
            d = self.call(args)
            chunk = rows(d)
            require(isinstance(d, dict) and type(d.get('total')) is int and d['total'] >= 0,
                    '任务分页缺少 total，不能确认完整性')
            result.extend(chunk)
            if len(result) >= d['total']:
                require(len(result) == d['total'], '任务分页 total 不一致')
                break
            require(chunk, '任务分页中途为空')
            page += 1
        ids = [r.get('fcRunId') for r in result]
        require(all(isinstance(x, str) and x for x in ids) and len(set(ids)) == len(ids), '任务分页 ID 缺失/重复')
        return result

    def identity(self):
        auth = self.call(['auth', 'status'])
        require(isinstance(auth, dict) and type(auth.get('readOnly')) is bool,
                '身份写策略缺失，暂停核验')
        require(not self.a.yes_paid or auth['readOnly'] is False,
                '当前身份或机器为只读；停止写操作，不切换身份或绕过 CLI')
        who = self.call(['whoami'])
        uid = who.get('user_id') if isinstance(who, dict) else None
        require(uid, 'whoami 缺少用户标识')
        require(not self.st.get('user_id') or self.st['user_id'] == uid, '账号与原账本不一致')
        self.st['user_id'] = uid
        return who

    def check_run_identity(self, kind, task, run):
        require(run.get('routeCode') in (None, '', task['route']) and
                (kind == 'mine' or run.get('problemCode') in (None, '', task['problem'])),
                '任务台逻辑身份不匹配；暂停核验')

    def cloud_gate(self):
        known = {a.get('fcRunId') for a in self.ledger() if a.get('fcRunId')}
        selected = set(self.st['routes'])
        if self.c.get('route'):
            selected.add(self.c['route'])
        selected.update(self.c.get('routes', []))
        snapshot = {}
        for kind in ('mine', 'explore'):
            known_tasks = {attempt['fcRunId']: task for task in self.st[kind]
                           for attempt in task['attempts'] if attempt.get('fcRunId')}
            # active 别名可能漏 dispatching；再全量读取任务台并核对。
            active = self.tasks(kind, active=True)
            all_runs = self.tasks(kind)
            snapshot[kind] = all_runs
            for r in active + all_runs:
                if r['fcRunId'] in known_tasks:
                    self.check_run_identity(kind, known_tasks[r['fcRunId']], r)
                status = r.get('status')
                require(isinstance(status, str) and status in ACTIVE | {'succeeded', 'failed', 'timeout'},
                        '云端任务状态未知，不能确认安全；暂停核验')
                if status in ('succeeded', 'failed'):
                    require(r.get('done') is True and r.get('ok') is (status == 'succeeded'),
                            '云端终态 done/ok 矛盾或缺失；暂停核验')
                if status == 'timeout' and (r.get('routeCode') in selected or r['fcRunId'] in known):
                    raise Pause('同路线存在 timeout，不能证明计算已停止')
                if status in ACTIVE:
                    require(r.get('done') is False and r.get('ok') is False,
                            '在途行 done/ok 矛盾或缺失')
                    require(r['fcRunId'] in known, '账号存在未纳入本账本的在途任务；人工核验后接管，不新发')
                elif r in active:
                    raise Pause('active 返回异常终态；暂停核验')
        return snapshot

    def validate_state(self):
        seen_ids, seen_keys = set(), set()
        for kind in ('mine', 'explore'):
            for task in self.st[kind]:
                key = (kind, task.get('route'), task.get('round') if kind == 'mine' else task.get('problem'))
                require(key not in seen_keys, '重复逻辑任务')
                seen_keys.add(key)
                require(task.get('route') in self.st['routes'], '账本路线超出计划')
                if kind == 'mine':
                    require(self.c['scenario'] == 'A' and type(task.get('round')) is int
                            and 1 <= task['round'] <= self.c['times'], '挖掘轮次超出计划')
                else:
                    require(task.get('problem') in self.c['problems'], '探索问题超出计划')
                require(task.get('status') in ACTIVE | {'submitting', 'succeeded', 'failed', 'ok', 'low_retain'},
                        '本地任务状态未知，暂停核验')
                aa = task.get('attempts')
                require(isinstance(aa, list) and 1 <= len(aa) <= 2, '尝试次数非法')
                if task.get('status') in ('ok', 'low_retain'):
                    last = aa[-1].get('last_poll', {})
                    require(kind == 'mine' and last.get('status') == 'succeeded'
                            and last.get('done') is True and last.get('ok') is True
                            and type(task.get('retained')) is int and task['retained'] >= 0,
                            '本地结算状态缺少成功读回/retained，不能跳过')
                for i, attempt in enumerate(aa):
                    require(attempt.get('attempt_no') == i + 1 and type(attempt.get('reserved_cent')) is int
                            and attempt['reserved_cent'] > 0, '尝试账本金额/编号异常')
                    require(attempt['reserved_cent'] == self.st['prices'].get(kind), '尝试保留费用与预检单价不符')
                    fc = attempt.get('fcRunId')
                    require(fc is None or (isinstance(fc, str) and fc), '云端 ID 类型非法')
                    if fc:
                        require(fc not in seen_ids, '重复云端 ID')
                        seen_ids.add(fc)
                    require(i == len(aa) - 1 or (fc and attempt.get('last_poll', {}).get('status') == 'failed'
                            and attempt['last_poll'].get('done') is True
                            and attempt['last_poll'].get('ok') is False), '历史失败未确认')
        rounds = [t['round'] for t in self.st['mine']]
        require(rounds == list(range(1, len(rounds) + 1)), '挖掘轮次不连续')
        require(self.spent() <= self.c['max_total_cent'], '已有失败/未知/补跑保留费用超预算')

    def approval(self):
        p = load(self.a.approval) if self.a.approval else None
        require(isinstance(p, dict) and p.get('approved') is True and p.get('authorization_source')
                and p.get('confirmed_at') and p.get('scope_workdir') == str(self.wd)
                and p.get('config_digest') == self.hash and p.get('user_id') == self.st['user_id']
                and p.get('max_total_cent') == self.c['max_total_cent'], '缺少匹配的人审批准记录')
        require(self.a.yes_paid and self.a.budget_cent == self.c['max_total_cent'],
                '需要显式 --yes-paid 与匹配的 --budget-cent')
        if 'max_submissions' in self.c:
            require(p.get('max_submissions') == self.c['max_submissions'], '批准提交数量与计划不符')
        selection = {'problems': self.st['problem_details'], 'routes': self.st['selected_route_details']}
        require(p.get('selection_digest') == digest(selection), '问题/路线读回与人审快照不符，需重新确认')
        denied = self.st.get('not_submitted', [])
        require(not denied or p.get('resume_not_submitted') == digest(denied[-1]),
                '本地拒绝后需重新确认；批准记录 resume_not_submitted 必须匹配最新拒绝摘要')
        return p

    def retry_policy(self):
        p = load(self.a.retry_authorization) if self.a.retry_authorization else None
        require(isinstance(p, dict) and p.get('authorized') is True and p.get('authorization_source')
                and p.get('confirmed_at') and p.get('scope_workdir') == str(self.wd)
                and p.get('config_digest') == self.hash and p.get('user_id') == self.st['user_id']
                and p.get('max_total_cent') == self.c['max_total_cent']
                and p.get('max_retries_per_logical_task') == 1
                and p.get('preserve_original_plan_budget') is True,
                '默认不补跑；缺少匹配的付费补跑授权')
        codes = p.get('execution_failure_codes')
        require(isinstance(codes, list) and codes and set(codes) <= EXECUTION_CODES, '补跑错误码白名单非法')
        return p

    def quote(self, kind, qty):
        d = self.call(['wallet', 'check', kind, '--qty', str(qty)])
        require(isinstance(d, dict) and d.get('currency') == 'CNY' and d.get('qty') == qty
                and type(d.get('unitPriceCent')) is int and d['unitPriceCent'] > 0
                and d.get('requiredCent') == qty * d['unitPriceCent']
                and type(d.get('availableCent')) is int and d['availableCent'] >= 0,
                '钱包价格/数量/币种契约不完整')
        return d

    def spent(self):
        # 即使失败或提交未知也不退款、不释放；不是实际扣费账单。
        return sum(a['reserved_cent'] for a in self.ledger())

    def remaining(self):
        mines = 0 if self.st['mine_complete'] else self.c['times'] - len(self.st['mine'])
        explores = len(self.st['routes']) * len(self.c['problems']) - len(self.st['explore'])
        return {'mine': mines, 'explore': explores}

    def budget(self, retry_kind=None, paid=True):
        counts = self.remaining()
        if retry_kind:
            counts[retry_kind] += 1
        checks = {k: self.quote(k, n) for k, n in counts.items() if n}
        for kind, d in checks.items():
            old = self.st['prices'].get(kind)
            expected = self.c.get('expected_unit_prices', {}).get(kind)
            require(expected is None or expected == d['unitPriceCent'], '单价与用户批准价格不同，暂停重新确认')
            require(old is None or old == d['unitPriceCent'], '动态单价变化，暂停重新确认预算')
            self.st['prices'][kind] = d['unitPriceCent']
        remaining = sum(d['requiredCent'] for d in checks.values())
        total = self.spent() + remaining
        if paid:
            require(total <= self.c['max_total_cent'], '预算不足：含失败/未知/补跑，并保留未提交原计划')
            require(all(d.get('affordable') is True for d in checks.values())
                    and (not checks or min(d['availableCent'] for d in checks.values()) >= remaining),
                    '完整剩余计划的合计钱包余额不足')
        self.st['budget_snapshot'] = {'at': now(), 'reserved_cent': self.spent(),
                                      'remaining_cent': remaining, 'worst_case_cent': total,
                                      'cap_cent': self.c['max_total_cent'], 'checks': checks}
        self.persist()
        return self.st['budget_snapshot']

    def preflight(self):
        self.identity()
        self.validate_state()
        if self.st['route_intent'] is not None:
            self.recover_not_submitted('route', self.st['route_intent'])
        # 先只读恢复提交未知，再查全量云端在途，不从历史时间窗猜 ID。
        for kind in ('mine', 'explore'):
            for t in self.st[kind]:
                if not t['attempts'][-1].get('fcRunId'):
                    self.reconcile(kind, t)
        cloud = self.cloud_gate()
        routes = rows(self.call(['factor-routes', 'list']))
        require(len({r.get('code') for r in routes}) == len(routes), '路线全集 code 重复')
        problems = [self.call(['problem', 'get', code]) for code in self.c['problems']]
        require(all(isinstance(p, dict) and p.get('code') == code for p, code in zip(problems, self.c['problems'])),
                '问题 readback code 不匹配')
        wanted = self.c.get('routes', []) if self.c['scenario'] == 'B' else ([self.c['route']] if self.c.get('route') else [])
        self.st['problem_details'] = problems
        self.st['selected_route_details'] = [r for code in wanted for r in routes if r.get('code') == code]
        require(len(self.st['selected_route_details']) == len(wanted), '既有路线缺失')
        if wanted:
            require(not self.st['routes'] or self.st['routes'] == wanted, '本地路线与配置不一致')
            self.st['routes'] = wanted
        elif not self.st['routes']:
            # 创建前暂用占位数量估算一次路线 × 问题的预算；绝不把占位提交给 CLI。
            self.st['routes'] = ['__pending_route__']
        self.st['routes_snapshot_digest'] = digest(routes)
        snapshot = self.budget(paid=False)
        pf = {'at': now(), 'config_digest': self.hash, 'scope_workdir': str(self.wd),
              'user_id': self.st['user_id'], 'routes': routes, 'cloud': cloud,
              'selection_digest': digest({'problems': problems, 'routes': self.st['selected_route_details']}),
              'routes_snapshot_digest': digest(routes), 'route_json_digest': digest(self.c.get('route_json')),
              'budget': snapshot, 'max_total_cent': self.c['max_total_cent'],
              'explore_combinations': len(self.st['routes']) * len(self.c['problems'])}
        save(self.wd / 'preflight.json', pf)
        self.event('preflight', **{k: pf[k] for k in ('selection_digest', 'routes_snapshot_digest',
                   'route_json_digest', 'budget')})
        return routes

    def route(self, routes):
        if not self.c.get('route_json'):
            return
        body = self.c['route_json']
        intent = self.st['route_intent']
        p = self.approval()
        require(self.a.create_route and p.get('route_create_approved') is True
                and p.get('route_json_digest') == digest(body) and p.get('seven_fields_shown') is True,
                '路线创建需显式 --create-route、七字段展示及人审内容摘要')
        if intent is None:
            self.identity()
            self.cloud_gate()
            self.budget()
            routes = rows(self.call(['factor-routes', 'list']))
            require(p.get('mechanism_review', {}).get('duplicate_found') is False
                    and p['mechanism_review'].get('routes_snapshot_digest') == digest(routes)
                    and p['mechanism_review'].get('rationale'), '缺少覆盖当前路线全集的人工机制去重结论')
            self.st['route_intent'] = intent = {'status': 'submitting', 'body': body,
                                                'receipt': f'raw/route-create-{time.time_ns()}.json', 'at': now()}
            self.persist()
            try:
                response = self.call(['route', 'create'], write=True, body=body,
                                     receipt=self.wd / intent['receipt'])
                intent['response'] = response
                if isinstance(response, dict):
                    intent['routeCode'] = response.get('routeCode')
                self.persist()
            except Pause as e:
                intent['submission_error'] = str(e)
                self.persist()
        self.recover_not_submitted('route', intent)
        code = intent.get('routeCode')
        if not code:
            receipt = load(self.wd / intent['receipt'], {})
            try:
                response = json.loads(receipt.get('stdout', ''))
            except (TypeError, ValueError):
                response = {}
            code = response.get('routeCode') if isinstance(response, dict) else None
        latest = rows(self.call(['factor-routes', 'list']))
        intent['readback'] = [r for r in latest if r.get('code') == code]
        self.persist()
        require(isinstance(code, str) and code and len(intent['readback']) == 1,
                'route intent 缺 ID/落库未知，暂停；禁止重复 create')
        require(all(intent['readback'][0].get(k) == v for k, v in body.items()),
                '七字段或额外提交字段 readback 不一致，禁止静默放宽')
        intent.update(status='verified', routeCode=code)
        self.st['routes'] = [code]
        self.persist()

    def recover_not_submitted(self, kind, intent, task=None):
        raw = load(self.wd / intent['receipt'], {})
        args = ['route', 'create'] if task is None else [kind, 'start', '--route', task['route']]
        if kind == 'explore':
            args += ['--problem', task['problem']]
        try:
            response = json.loads(raw.get('stderr', ''))
        except (TypeError, ValueError):
            return
        error = response.get('error') if isinstance(response, dict) else None
        # 只有匹配本次命令的完整本地拒绝回执，才能证明目标请求未发出。
        if not (raw.get('returncode') == 8 and raw.get('write') is True and raw.get('args') == args
                and isinstance(raw.get('stdout'), str) and not raw['stdout'].strip()
                and isinstance(error, dict) and error.get('kind') == 'config'
                and error.get('action') == 'not_permitted' and error.get('retryable') is False
                and isinstance(error.get('message'), str) and error['message'].strip()
                and error.get('status') is None and not intent.get('fcRunId') and not intent.get('routeCode')):
            return
        intent['status'] = 'not_submitted'
        record = {'kind': kind, 'args': args, 'intent': intent, 'round': task.get('round') if task else None}
        self.st.setdefault('not_submitted', []).append(record)
        if task is None:
            self.st['route_intent'] = None
        else:
            task['attempts'].pop()
            if task['attempts']:
                task['status'] = task['attempts'][-1]['status']
            else:
                self.st[kind].remove(task)
        self.persist()  # 拒绝证据与回退一起落盘，不丢失旧回执或已失败的付费尝试。
        token = digest(record)
        self.event('not_submitted', receipt=intent['receipt'], resume_not_submitted=token,
                   reserved_cent=self.spent())
        raise Pause(f'请求确定未发出，已退回待提交；交给用户解决本地拒绝并重新确认；resume_not_submitted={token}')

    def reconcile(self, kind, task):
        a = task['attempts'][-1]
        self.recover_not_submitted(kind, a, task)
        receipt = load(self.wd / a['receipt'], {})
        try:
            response = json.loads(receipt.get('stdout', ''))
        except (TypeError, ValueError):
            response = {}
        fc = response.get('fcRunId') if isinstance(response, dict) else None
        matches = [r for r in self.tasks(kind) if fc and r.get('fcRunId') == fc]
        a['reconciliation'] = {'at': now(), 'receipt_id': fc, 'task_matches': matches, 'write_retried': False}
        self.persist()
        require(isinstance(fc, str) and fc and len(matches) == 1,
                'intent 无可信 ID，不能确认无落库；暂停，不释放费用、不重发')
        self.check_run_identity(kind, task, matches[0])
        require(fc not in {x.get('fcRunId') for x in self.ledger() if x is not a}, '核验 ID 与其它尝试重复')
        a['fcRunId'] = fc
        self.poll(kind, task)
        self.persist()

    def poll(self, kind, task):
        a = task['attempts'][-1]
        d = rows(self.call([kind, 'poll', a['fcRunId']]))
        require(len(d) == 1 and d[0].get('fcRunId') == a['fcRunId'], 'poll ID 缺失/重复/错配')
        one = d[0]
        require(one.get('routeCode') == task['route'] and
                (kind == 'mine' or one.get('problemCode') == task['problem']), 'poll 逻辑身份不匹配')
        a.update(last_poll=one, status=one.get('status'), last_poll_at=now())
        task['status'] = a['status']
        self.persist()
        if kind == 'explore':
            self.event('explore_progress', route=task['route'], problem=task['problem'],
                       fcRunId=a['fcRunId'], status=a['status'], percent=one.get('percent'))
        if a['status'] in ACTIVE:
            require(one.get('done') is False and one.get('ok') is not True, '在途 done/ok 矛盾或缺失')
        elif a['status'] == 'succeeded':
            require(one.get('done') is True and one.get('ok') is True, '成功终态缺少 done=true/ok=true')
        elif a['status'] == 'failed':
            require(one.get('done') is True and one.get('ok') is False, '失败终态缺少 done=true/ok=false')
        else:
            raise Pause('timeout 或未知终态；不能假定计算停止，不补跑')
        return one

    def execution_failure(self, one, policy):
        blocked = ('permission', 'denied', 'unauthor', 'config', 'no_valid_factors',
                   'billing', 'charge', 'wallet', 'timeout', '权限', '配置', '无因子', '扣费', '超时')
        return (one.get('status') == 'failed' and one.get('done') is True and one.get('ok') is False
                and one.get('errorCode') in policy['execution_failure_codes']
                and not any(x in str(one.get('errorMessage', '')).lower() for x in blocked))

    def retry(self, kind, task):
        policy = self.retry_policy()
        require(len(task['attempts']) == 1, '逻辑任务已补跑一次，不能再次补跑')
        one = self.poll(kind, task)
        require(self.execution_failure(one, policy), '不是授权的明确执行失败；不补跑')
        all_runs = self.tasks(kind)
        old = [r for r in all_runs if r.get('fcRunId') == one['fcRunId']]
        require(len(old) == 1 and old[0].get('status') == 'failed'
                and old[0].get('routeCode') == task['route'], '任务台未确认 failed 停止状态')
        for r in all_runs:
            if r.get('routeCode') == task['route']:
                # runs 的 problemCode 可能缺失；同路线一律保守阻断。
                require(r.get('status') not in ACTIVE | {'timeout'}, '同逻辑存在在途/timeout，禁止补跑')
        task['retry_readback'] = {'at': now(), 'poll': one, 'task_row': old[0]}
        self.persist()
        self.start(kind, task, retry=True)

    def inventory(self, route):
        fs = self.call(['factor', 'summary'])
        save(self.wd / 'factor-summary-latest.json', fs)
        rr = fs.get('route_distribution') if isinstance(fs, dict) else None
        require(isinstance(rr, list), '库存结构未知，阻断探索')
        matches = [r for r in rr if r.get('route_code') == route]
        require(len(matches) == 1 and type(matches[0].get('factor_count')) is int
                and matches[0]['factor_count'] >= 0, '库存缺失/未知，阻断探索')
        return matches[0]

    def gate(self, kind, retry=False):
        self.identity()
        self.approval()
        self.validate_state()
        require(all(a.get('fcRunId') for a in self.ledger()), '未解决的 intent 阻断所有新写')
        if not retry:
            require(not any(t.get('status') == 'failed' for k in ('mine', 'explore') for t in self.st[k]),
                    '先处理已有失败，不发新任务')
        self.cloud_gate()
        if 'max_submissions' in self.c:
            require(len(self.ledger()) < self.c['max_submissions'], '达到批准提交次数上限，不再提交')
        self.budget(retry_kind=kind if retry else None)
        if kind == 'explore':
            require(self.st['mine_complete'], '挖掘尚未正常结束，不探索')

    def start(self, kind, task, retry=False):
        self.gate(kind, retry)
        if retry:
            require(len(task['attempts']) == 1 and self.execution_failure(
                    task['attempts'][-1]['last_poll'], self.retry_policy()) and task.get('retry_readback'),
                    '补跑证据/授权不完整')
            latest = self.tasks(kind)
            require(not any(r.get('routeCode') == task['route'] and r.get('status') in ACTIVE | {'timeout'}
                            for r in latest), '补跑写前再次发现同逻辑在途/timeout，暂停')
        if kind == 'explore':
            require(self.inventory(task['route'])['factor_count'] > 0, '零库存，禁止 explore start')
        n = len(task['attempts']) + 1
        name = digest([kind, task['route'], task.get('round'), task.get('problem'), n])
        a = {'attempt_no': n, 'status': 'submitting', 'fcRunId': None, 'createdAt': now(),
             'reserved_cent': self.st['prices'][kind], 'receipt': f'raw/{name}-{time.time_ns()}-start.json'}
        task['attempts'].append(a)
        task['status'] = 'submitting'
        if not retry:
            self.st[kind].append(task)
        self.persist()  # 写前 fsync intent 与费用保留；崩溃即进入只读核验。
        args = [kind, 'start', '--route', task['route']]
        if kind == 'explore':
            args += ['--problem', task['problem']]
        self.event('submission_intent', kind=kind, route=task['route'], problem=task.get('problem'),
                   round=task.get('round'), attempt=n, reserved_cent=self.spent())
        try:
            response = self.call(args, write=True, receipt=self.wd / a['receipt'])
            a['response'] = response
            a['fcRunId'] = response.get('fcRunId') if isinstance(response, dict) else None
            a['status'] = response.get('status') if isinstance(response, dict) else 'unknown'
            self.persist()
        except Pause as e:
            a['submission_error'] = str(e)
            self.persist()
        if not a.get('fcRunId'):
            self.reconcile(kind, task)
        require(isinstance(a['fcRunId'], str) and a['fcRunId'], '非法提交 ID')
        self.validate_state()
        # 立即 poll，以权威身份/状态核对写响应；不信 start 返回等于完成。
        self.poll(kind, task)

    def wait(self, kind, task):
        a = task['attempts'][-1]
        a.setdefault('wait_started_epoch', time.time())
        self.persist()
        while True:
            one = self.poll(kind, task)
            if one['status'] == 'succeeded':
                return
            if one['status'] == 'failed':
                self.retry(kind, task)
                a = task['attempts'][-1]
                a.setdefault('wait_started_epoch', time.time())
                self.persist()
                continue
            require(time.time() - a['wait_started_epoch'] < self.c.get('max_wait_minutes', 1500) * 60,
                    '超出监控窗口，暂停而非取消；云端可能仍在计算')
            time.sleep(self.c.get('poll_seconds', 60))

    def mine(self):
        while not self.st['mine_complete']:
            pending = [t for t in self.st['mine'] if t.get('status') not in ('ok', 'low_retain')]
            require(len(pending) <= 1, '串行挖掘出现多条未结算任务')
            if pending:
                task = pending[0]
            elif len(self.st['mine']) >= self.c['times']:
                self.st.update(mine_complete=True, mine_stop_reason=f"跑满 {self.c['times']} 轮")
                self.persist()
                break
            else:
                task = {'round': len(self.st['mine']) + 1, 'route': self.st['routes'][0], 'attempts': []}
                self.start('mine', task)
            self.wait('mine', task)
            a = task['attempts'][-1]
            ov = self.call(['mining', 'overview', a['fcRunId']])
            kpi = ov.get('kpi', {}) if isinstance(ov, dict) else {}
            ret = kpi.get('retained')
            require(type(ret) is int and ret >= 0, 'retained 缺失或非法，不触发下一轮')
            task.update(retained=ret, kpi=kpi, total_candidates=kpi.get('total_candidates'),
                        retain_rate=kpi.get('retain_rate'), funnel=ov.get('funnel'),
                        elimination_breakdown=ov.get('elimination_breakdown'),
                        status='low_retain' if ret < self.c['threshold'] else 'ok')
            self.event('mine_round_completed', round=task['round'], times=self.c['times'], retained=ret)
            if ret < self.c['threshold']:
                self.st.update(mine_complete=True, mine_stop_reason=f"第 {task['round']} 轮 retained={ret} 低于 {self.c['threshold']}，正常早停")
                self.event('early_stop', round=task['round'], retained=ret, threshold=self.c['threshold'])
            self.persist()
        self.st['phase'] = 'exploring'
        self.persist()

    def explore(self):
        zero = [r for r in self.st['routes'] if self.inventory(r)['factor_count'] == 0]
        if zero:
            self.st.update(phase='failed_zero_inventory', failure_reason='零库存，跳过全部探索：' + ','.join(zero))
            self.persist()
            return False
        # 续跑先接管所有已有提交（包括 queued/dispatching/failed/succeeded），再发新任务。
        for task in self.st['explore']:
            self.wait('explore', task)
        known = {(t['route'], t['problem']) for t in self.st['explore']}
        todo = [(r, p) for r in self.st['routes'] for p in self.c['problems'] if (r, p) not in known]
        # 批次内每次提交前轮询已有在途：已失败者不允许 shell 继续串接新 start。
        size = self.c.get('batch_size', 8)
        for offset in range(0, len(todo), size):
            batch = []
            for route, problem in todo[offset:offset + size]:
                for current in batch:
                    if current['status'] in ACTIVE:
                        one = self.poll('explore', current)
                        require(one['status'] != 'failed', '批内发现失败，暂停新提交；续跑先核验补跑')
                task = {'route': route, 'problem': problem, 'attempts': []}
                self.start('explore', task)
                batch.append(task)
            for task in batch:
                self.wait('explore', task)
        require(len(self.st['explore']) == len(self.st['routes']) * len(self.c['problems'])
                and all(t['attempts'][-1].get('last_poll', {}).get('ok') is True
                        and t['attempts'][-1]['last_poll'].get('done') is True for t in self.st['explore']),
                '探索未全部终态成功，不能宣布完成')
        return True

    def fetch_report(self, success):
        self.st['phase'] = 'fetching'
        self.persist()
        fs = self.call(['factor', 'summary'])
        require(isinstance(fs, dict) and isinstance(fs.get('route_distribution'), list), '报告库存取数失败')
        lookup = {r['id']: r for r in rows(self.call(['experiment', 'list']))}
        out = {'generated_at': now(), 'workdir': str(self.wd), 'experiments': [],
               'routes': [r for r in fs['route_distribution'] if r.get('route_code') in self.st['routes']],
               'explore_state': {'items': self.st['explore']}, 'problems': self.st['problem_details'],
               'pipeline': {'outcome': 'success' if success else 'failed',
                            'failure_reason': self.st.get('failure_reason'),
                            'reserved_cent': self.spent(), 'cap_cent': self.c['max_total_cent'],
                            'attempt_ledger': self.ledger(), 'config_digest': self.hash}}
        if self.c['scenario'] == 'A':
            out['mine'] = {'route': self.st['routes'][0], 'rounds': self.st['mine'],
                           'threshold': self.c['threshold'], 'times': self.c['times'],
                           'stopped_reason': self.st['mine_stop_reason']}
        gaps = []
        for task in self.st['explore']:
            a = task['attempts'][-1]
            require(a.get('last_poll', {}).get('status') == 'succeeded'
                    and a['last_poll'].get('done') is True and a['last_poll'].get('ok') is True,
                    '只抓取确认成功的当前尝试；失败历史独立保留')
            fc = a['fcRunId']
            original = lookup.get(fc)
            exp = {'id': fc, 'route_code': task['route'], 'list_row': original,
                   'read_errors': [], 'field_sources': {}}
            for key, cmd in (('overview', 'get'), ('strategies', 'strategies'), ('review_matrix', 'review-matrix')):
                try:
                    value = self.call(['experiment', cmd, fc])
                    require(isinstance(value, dict) and isinstance(value.get('overview'), dict)
                            if key == 'overview' else isinstance(value, (dict, list)), '实验结果结构未知')
                    if key != 'overview':
                        rows(value)
                    exp[key] = value
                except Pause as e:
                    exp[key] = None
                    exp['read_errors'].append({'command': cmd, 'error': str(e)})
                    gaps.append({'id': fc, 'command': cmd, 'error': str(e)})
            overview = (exp.get('overview') or {}).get('overview') or {}
            meta = dict(original or {})
            for k in ('problem_code', 'problem_name', 'dataset', 'freq', 'symbols_count',
                      'n_backtests', 'passed', 'pass_rate', 'strategy_count', 'total_elapsed'):
                if meta.get(k) is not None:
                    exp['field_sources'][k] = 'experiment list'
                elif overview.get(k) is not None:
                    meta[k] = overview[k]
                    exp['field_sources'][k] = 'experiment get.overview'
            # 仅身份/问题配置可从已审批快照补齐；统计字段绝不推算或映射 remaining_count。
            problem = next(p for p in self.st['problem_details'] if p['code'] == task['problem'])
            for k, value in {'problem_code': task['problem'], 'problem_name': problem.get('name'),
                             'dataset': problem.get('dataset'), 'freq': problem.get('freq'),
                             'symbols_count': len(problem.get('symbols', []))}.items():
                if meta.get(k) is None and value is not None:
                    meta[k] = value
                    exp['field_sources'][k] = 'approved problem readback / task identity'
            require(meta['problem_code'] == task['problem'], '实验问题与精确任务 ID 不匹配')
            missing = [k for k in ('n_backtests', 'passed', 'pass_rate', 'strategy_count', 'total_elapsed')
                       if meta.get(k) is None]
            if missing:
                gaps.append({'id': fc, 'missing_fields': missing})
            if exp.get('strategies') is not None and exp.get('review_matrix') is not None:
                candidates = rows(exp['strategies'])
                matrix = rows(exp['review_matrix'])
                oos_codes = {r.get('strategy') or r.get('code') for r in matrix
                             if r.get('segment_name') == '后置验证' and r.get('夏普比率') is not None}
                absent = [s.get('code') for s in candidates if not s.get('code') or s['code'] not in oos_codes]
                if absent:
                    gaps.append({'id': fc, 'missing_oos_candidates': absent})
            exp['metadata'] = meta
            exp['strategies_read_count'] = len(rows(exp['strategies'])) if exp.get('strategies') is not None else None
            out['experiments'].append(exp)
        out['pipeline']['data_gaps'] = gaps
        outcome = 'partial' if success and gaps else ('success' if success else 'failed')
        out['pipeline']['outcome'] = outcome
        if gaps:
            out['pipeline']['failure_reason'] = '真实取数字段或接口缺失；缺失值保留为空，不编造数据'
        require(len(out['routes']) == len(self.st['routes']), '报告缺路线库存，不能宣布完整')
        save(self.wd / 'fetched' / 'summary.json', out)
        self.event('fetch_completed', experiments=len(out['experiments']))
        report = self.wd / 'report.html'
        process = subprocess.run([sys.executable, str(Path(__file__).with_name('gen_report.py')),
                                  '--workdir', str(self.wd), '--out', str(report),
                                  '--scenario', self.c['scenario'], '--theme', self.c.get('theme', 'scarlet'),
                                  '--title', self.c.get('title', 'skz 自动化投研汇总')],
                                 capture_output=True, text=True, timeout=180)
        save(self.wd / 'report-render.json', {'returncode': process.returncode,
                                            'stdout': process.stdout, 'stderr': process.stderr})
        require(process.returncode == 0, '报告生成失败，见 report-render.json')
        report_hash = verify_report(report)
        self.st.update(phase='completed_partial' if outcome == 'partial' else
                       ('completed' if success else 'completed_failed'), report=str(report),
                       data_gaps=gaps, report_sha256=report_hash, completed_at=now())
        self.persist()
        self.event('final_completed', outcome=outcome, report=str(report),
                   report_sha256=report_hash, reserved_cent=self.spent(), caller_must_present_files=True)

    def run(self):
        if self.st['phase'] in ('completed', 'completed_failed', 'completed_partial'):
            require(verify_report(self.st['report']) == self.st['report_sha256'], '已完成报告被修改/丢失')
            self.event('final_completed', outcome=('partial' if self.st['phase'] == 'completed_partial' else
                       ('success' if self.st['phase'] == 'completed' else 'failed')),
                       report=self.st['report'], caller_must_present_files=True, resumed=True)
            return 0 if self.st['phase'] == 'completed' else 1
        routes = self.preflight()
        if not self.a.yes_paid:
            self.event('dry_run', message='仅只读预检；未创建路线、未提交付费任务')
            return 0
        self.approval()
        # 在 route create 前也验证完整计划预算，不能先写后发现预算不够。
        self.budget()
        self.route(routes)
        self.validate_state()
        self.mine()
        success = self.explore()
        self.fetch_report(success)
        return 0 if self.st['phase'] == 'completed' else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description='skz 全流程：默认只读预检；显式审批后挖掘→探索→抓取→报告')
    ap.add_argument('--config', required=True, help='已由用户确认的通用计划 JSON')
    ap.add_argument('--workdir', required=True, help='独占状态目录，不与其它 worker 共用')
    ap.add_argument('--yes-paid', action='store_true', help='与审批记录同时使用，才允许写操作')
    ap.add_argument('--approval', help='人工批准记录 JSON；不由脚本自批')
    ap.add_argument('--budget-cent', type=int, help='用户明确同意的预算上限（分），必须与计划/批准一致')
    ap.add_argument('--retry-authorization', help='可选单独补跑授权；默认不重试付费')
    ap.add_argument('--create-route', action='store_true', help='场景 A 可选创建已确认的 route_json')
    ap.add_argument('--skz-bin', default=os.environ.get('SKZ_BIN', 'skz'), help='skz 可执行文件')
    args = ap.parse_args(argv)
    wd = Path(args.workdir).resolve()
    if not wd.is_dir():
        wd.mkdir(parents=True, exist_ok=True)
    for name in ('raw', 'fetched'):
        if not (wd / name).is_dir():
            (wd / name).mkdir(exist_ok=True)
    with (wd / 'pipeline.lock').open('a+') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print(json.dumps({'event': 'paused', 'reason': '工作目录已被其它 worker 独占'}, ensure_ascii=False), flush=True)
            return 2
        worker = None
        try:
            worker = Pipeline(args)
            return worker.run()
        except (Pause, OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as e:
            if worker:
                worker.st.update(paused_at=now(), paused_error=str(e))
                worker.persist()
                worker.event('paused', reason=str(e), cloud_may_still_run=True)
            else:
                print(json.dumps({'event': 'paused', 'reason': str(e)}, ensure_ascii=False), flush=True)
            return 2


if __name__ == '__main__':
    sys.exit(main())
