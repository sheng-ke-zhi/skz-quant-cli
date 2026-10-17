"""离线模拟安全契约；CLI 全部替换，任何遗漏的 subprocess 默认禁止。"""
import copy
import fcntl
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import pipeline as p


class FakeCLI:
    def __init__(self):
        self.calls, self.writes, self.runs = [], [], {'mine': [], 'explore': []}
        self.stock, self.retained, self.failures = 20, 60, 0
        self.routes = [{'code': 'route-a', 'name': '模拟路线'}]
        self.write_timeout = False
        self.poll_override = None
        self.read_only = False

    def call(self, args, *, write=False, body=None, receipt=None):
        self.calls.append(list(args))
        kind = args[0]
        if write:
            if kind == 'route':
                self.writes.append(list(args))
                self.routes.append({'code': 'created-route', **body})
                d = {'routeCode': 'created-route'}
                p.save(receipt, {'stdout': json.dumps(d)})
                return d
            self.writes.append(list(args))
            d = {'fcRunId': f'id-{len(self.writes)}', 'routeCode': args[args.index('--route') + 1],
                 'status': 'queued', 'done': False, 'ok': False}
            if kind == 'explore':
                d['problemCode'] = args[args.index('--problem') + 1]
            self.runs[kind].append(d)
            if self.write_timeout:
                p.save(receipt, {'exception': '模拟超时'})
                raise p.Pause('模拟提交超时')
            p.save(receipt, {'stdout': json.dumps(d)})
            return copy.deepcopy(d)
        if args == ['auth', 'status']:
            return {'readOnly': self.read_only}
        if args == ['whoami']:
            return {'user_id': 'test-user'}
        if args[:2] == ['wallet', 'check']:
            qty = int(args[-1])
            unit = 100 if args[2] == 'mine' else 20
            return {'qty': qty, 'currency': 'CNY', 'unitPriceCent': unit,
                    'requiredCent': qty * unit, 'availableCent': 100000, 'affordable': True}
        if args == ['factor-routes', 'list']:
            return copy.deepcopy(self.routes)
        if args[:2] == ['problem', 'get']:
            return {'code': args[2], 'time_segments': [], 'symbols': ['SIM'], 'freq': '模拟'}
        if len(args) > 1 and args[1] == 'runs':
            rr = copy.deepcopy(self.runs[kind])
            if '--status' in args:
                rr = [r for r in rr if r['status'] in p.ACTIVE]
            return {'items': rr, 'total': len(rr)}
        if len(args) > 1 and args[1] == 'poll':
            row = next(r for r in self.runs[kind] if r['fcRunId'] == args[2])
            if self.poll_override:
                row.update(self.poll_override)
            elif self.failures:
                self.failures -= 1
                row.update(status='failed', done=True, ok=False, errorCode='TASK_EXECUTION_FAILED')
            elif row['status'] != 'failed':
                row.update(status='succeeded', done=True, ok=True)
            return [copy.deepcopy(row)]
        if args[:2] == ['mining', 'overview']:
            return {'kpi': {'retained': self.retained, 'total_candidates': 100, 'retain_rate': .6},
                    'funnel': [], 'elimination_breakdown': []}
        if args == ['factor', 'summary']:
            return {'route_distribution': [{'route_code': r['code'], 'route_name': r['name'],
                                           'factor_count': self.stock} for r in self.routes]}
        if args == ['experiment', 'list']:
            return {'items': [{'id': r['fcRunId'], 'problem_code': r.get('problemCode'),
                               'n_backtests': 1, 'passed': 0, 'pass_rate': 0,
                               'strategy_count': 0, 'total_elapsed': 1}
                              for r in self.runs['explore'] if r['status'] == 'succeeded']}
        if args[:2] == ['experiment', 'get']:
            return {'overview': {'passed': 0}}
        if kind == 'experiment' and args[1] in ('strategies', 'review-matrix'):
            return {'items': []}
        raise AssertionError(f'未覆盖模拟 CLI：{args}')


class Tests(unittest.TestCase):
    def exploration_intent(self, worker):
        worker.preflight()
        task = {'route': 'route-a', 'problem': 'problem-a', 'status': 'submitting',
                'attempts': [{'fcRunId': None, 'status': 'submitting', 'reserved_cent': 20,
                              'receipt': 'raw/explore-intent.json'}]}
        worker.st['explore'].append(task)
        return task

    def test_exploration_keys_distinguish_problems_but_reject_duplicate_pair(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'],
                    'problems': ['problem-a', 'problem-b'], 'max_total_cent': 100}
        worker = self.worker()
        worker.preflight()
        for problem in self.cfg['problems']:
            worker.start('explore', {'route': 'route-a', 'problem': problem, 'attempts': []})
        worker.validate_state()
        worker.st['explore'].append(copy.deepcopy(worker.st['explore'][0]))
        with self.assertRaisesRegex(p.Pause, '重复逻辑任务'):
            worker.validate_state()
        self.assertEqual(len(self.cli.writes), 2)

    def test_close_creation_times_without_receipt_never_adopt_or_replay(self):
        worker = self.worker()
        task = self.exploration_intent(worker)
        self.cli.runs['explore'] = [
            {'fcRunId': run_id, 'routeCode': 'route-a', 'problemCode': 'problem-a',
             'status': 'running', 'done': False, 'ok': False, 'createdAt': p.now()}
            for run_id in ('nearby-1', 'nearby-2')]
        for unused in range(2):
            with self.assertRaisesRegex(p.Pause, 'intent 无可信 ID'):
                worker.reconcile('explore', task)
        self.assertIsNone(task['attempts'][0]['fcRunId'])
        self.assertEqual(worker.spent(), 20)
        self.assertEqual(self.cli.writes, [])

    def test_explore_receipt_requires_matching_problem_and_route(self):
        worker = self.worker()
        task = self.exploration_intent(worker)
        p.save(self.wd / 'raw/explore-intent.json', {'stdout': json.dumps({'fcRunId': 'trusted'})})
        row = {'fcRunId': 'trusted', 'routeCode': 'route-a', 'problemCode': 'problem-a',
               'status': 'running', 'done': False, 'ok': False}
        self.cli.poll_override = dict(row)
        for field, value in (('routeCode', 'other-route'), ('problemCode', 'other-problem')):
            with self.subTest(field=field):
                self.cli.runs['explore'] = [{**row, field: value}]
                with self.assertRaisesRegex(p.Pause, '任务台逻辑身份不匹配'):
                    worker.reconcile('explore', task)
        self.assertEqual(self.cli.writes, [])

    def test_explore_poll_cannot_override_receipt_identity(self):
        worker = self.worker()
        task = self.exploration_intent(worker)
        p.save(self.wd / 'raw/explore-intent.json', {'stdout': json.dumps({'fcRunId': 'trusted'})})
        row = {'fcRunId': 'trusted', 'routeCode': 'route-a', 'problemCode': 'problem-a',
               'status': 'running', 'done': False, 'ok': False}
        for field, value in (('routeCode', 'other-route'), ('problemCode', 'other-problem'),
                             ('fcRunId', 'other-id')):
            with self.subTest(field=field):
                self.cli.runs['explore'] = [dict(row)]
                self.cli.poll_override = {field: value}
                with self.assertRaises(p.Pause):
                    worker.reconcile('explore', task)
        self.assertEqual(worker.spent(), 20)
        self.assertEqual(self.cli.writes, [])

    def test_explore_optional_problem_requires_poll_evidence(self):
        worker = self.worker()
        task = self.exploration_intent(worker)
        p.save(self.wd / 'raw/explore-intent.json', {'stdout': json.dumps({'fcRunId': 'trusted'})})
        row = {'fcRunId': 'trusted', 'routeCode': 'route-a', 'status': 'running',
               'done': False, 'ok': False}
        for problem in ('problem-a', None, ''):
            for present in (True, False):
                with self.subTest(problem=problem, present=present):
                    candidate = {**row, 'problemCode': problem} if present else dict(row)
                    self.cli.runs['explore'] = [candidate]
                    self.cli.poll_override = {'status': 'running', 'done': False, 'ok': False}
                    if present and problem == 'problem-a':
                        worker.reconcile('explore', task)
                    else:
                        with self.assertRaisesRegex(p.Pause, 'poll 逻辑身份不匹配'):
                            worker.reconcile('explore', task)
                    self.cli.poll_override['problemCode'] = 'problem-a'
                    worker.reconcile('explore', task)
        self.assertEqual(self.cli.writes, [])

    def persisted_exploration(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100}
        worker = self.worker()
        worker.preflight()
        self.cli.poll_override = {'status': 'queued', 'done': False, 'ok': False}
        worker.start('explore', {'route': 'route-a', 'problem': 'problem-a', 'attempts': []})
        return copy.deepcopy(self.cli.runs['explore'][0])

    def test_reloaded_explore_rejects_explicit_inventory_identity_conflict(self):
        row = self.persisted_exploration()
        for status in ('queued', 'succeeded'):
            for field, value in (('problemCode', 'other-problem'), ('routeCode', 'other-route')):
                with self.subTest(status=status, field=field):
                    self.cli.runs['explore'] = [{**row, field: value, 'status': status,
                                                 'done': status == 'succeeded',
                                                 'ok': status == 'succeeded'}]
                    self.cli.poll_override = dict(row)
                    worker = self.worker()
                    task = worker.st['explore'][0]
                    self.assertEqual(task['attempts'][-1]['fcRunId'], row['fcRunId'])
                    with self.assertRaisesRegex(p.Pause, '任务台逻辑身份不匹配'):
                        worker.preflight()
                        worker.poll('explore', task)
                    self.assertEqual(worker.spent(), 20)
                    self.assertEqual(len(self.cli.writes), 1)

    def test_reloaded_explore_missing_inventory_fields_allow_trusted_poll_evidence(self):
        row = self.persisted_exploration()
        for value in (None, ''):
            for present in (False, True):
                with self.subTest(value=value, present=present):
                    candidate = {key: item for key, item in row.items()
                                 if key not in ('problemCode', 'routeCode')}
                    if present:
                        candidate.update(problemCode=value, routeCode=value)
                    self.cli.runs['explore'] = [candidate]
                    self.cli.poll_override = dict(row)
                    worker = self.worker()
                    worker.preflight()
                    result = worker.poll('explore', worker.st['explore'][0])
                    self.assertEqual(result['problemCode'], 'problem-a')
                    self.assertEqual(result['routeCode'], 'route-a')
                    self.assertEqual(worker.spent(), 20)
                    self.assertEqual(len(self.cli.writes), 1)

    def test_reloaded_explore_missing_poll_identity_still_pauses(self):
        row = self.persisted_exploration()
        for field in ('problemCode', 'routeCode'):
            with self.subTest(field=field):
                self.cli.runs['explore'] = [{key: item for key, item in row.items() if key != field}]
                self.cli.poll_override = {'status': 'queued', 'done': False, 'ok': False}
                worker = self.worker()
                worker.preflight()
                with self.assertRaisesRegex(p.Pause, 'poll 逻辑身份不匹配'):
                    worker.poll('explore', worker.st['explore'][0])
                self.assertEqual(worker.spent(), 20)
                self.assertEqual(len(self.cli.writes), 1)

    def test_unknown_cloud_status_blocks_preflight_without_writes(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100}
        worker = self.worker()
        worker.preflight()
        self.cli.runs['explore'].append({'fcRunId': 'outside-ledger', 'routeCode': 'route-a',
                                         'problemCode': 'problem-a', 'status': 'unrecognized_status',
                                         'done': False, 'ok': False})
        with self.assertRaises(p.Pause):
            worker.preflight()
        task = {'route': 'route-a', 'problem': 'problem-a', 'attempts': []}
        with self.assertRaises(p.Pause):
            worker.start('explore', task)
        self.assertEqual(self.cli.writes, [])
        self.assertEqual(task['attempts'], [])
        self.assertEqual(worker.spent(), 0)

    def test_contradictory_cloud_success_blocks_new_write(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100}
        worker = self.worker()
        worker.preflight()
        self.cli.runs['explore'].append({'fcRunId': 'outside-ledger', 'routeCode': 'route-a',
                                         'problemCode': 'problem-a', 'status': 'succeeded',
                                         'done': False, 'ok': False})
        task = {'route': 'route-a', 'problem': 'problem-a', 'attempts': []}
        with self.assertRaises(p.Pause):
            worker.start('explore', task)
        self.assertEqual(self.cli.writes, [])
        self.assertEqual(worker.spent(), 0)

    def test_cloud_flag_inconsistencies_block_new_writes(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100}
        worker = self.worker()
        worker.preflight()
        cases = [('succeeded', False, False), ('succeeded', True, False),
                 ('succeeded', None, True), ('succeeded', True, None),
                 ('succeeded', 1, True), ('succeeded', True, 1),
                 ('failed', False, False), ('failed', True, True),
                 ('failed', True, None), ('failed', None, False),
                 ('queued', None, False), ('dispatching', True, False),
                 ('running', False, True), ('running', False, None)]
        for kind in ('mine', 'explore'):
            for status, done, ok in cases:
                with self.subTest(kind=kind, status=status, done=done, ok=ok):
                    self.cli.runs = {'mine': [], 'explore': []}
                    row = {'fcRunId': 'outside-ledger', 'routeCode': 'route-a',
                           'problemCode': 'problem-a', 'status': status}
                    if done is not None:
                        row['done'] = done
                    if ok is not None:
                        row['ok'] = ok
                    self.cli.runs[kind].append(row)
                    task = {'route': 'route-a', 'problem': 'problem-a', 'attempts': []}
                    with self.assertRaisesRegex(p.Pause, 'done/ok'):
                        worker.start('explore', task)
                    self.assertEqual(self.cli.writes, [])
                    self.assertEqual(task['attempts'], [])
                    self.assertEqual(worker.spent(), 0)

    def test_valid_cloud_terminal_history_allows_approved_write(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100}
        for kind in ('mine', 'explore'):
            for status in ('succeeded', 'failed'):
                self.cli.runs[kind].append({'fcRunId': f'history-{kind}-{status}',
                                           'routeCode': 'route-a', 'problemCode': 'problem-a',
                                           'status': status, 'done': True, 'ok': status == 'succeeded'})
        worker = self.worker()
        worker.preflight()
        worker.start('explore', {'route': 'route-a', 'problem': 'problem-a', 'attempts': []})
        self.assertEqual(self.cli.writes, [['explore', 'start', '--route', 'route-a', '--problem', 'problem-a']])

    def test_write_transport_timeout_is_recorded_without_replay(self):
        receipt = self.wd / 'raw/write-timeout.json'
        client = p.CLI(self.wd, 'NEVER_REAL_SKZ')
        timeout = subprocess.TimeoutExpired('NEVER_REAL_SKZ', 180)
        with patch.object(p.subprocess, 'run', side_effect=timeout) as transport:
            with self.assertRaises(p.Pause):
                client.call(['mine', 'start', '--route', 'route-a'], write=True, receipt=receipt)
        self.assertEqual(transport.call_count, 1)
        self.assertIn('exception', p.load(receipt))

    def test_read_transport_retry_is_bounded(self):
        client = p.CLI(self.wd, 'NEVER_REAL_SKZ')
        timeout = subprocess.TimeoutExpired('NEVER_REAL_SKZ', 180)
        with patch.object(p.subprocess, 'run', side_effect=timeout) as transport, patch.object(p.time, 'sleep'):
            with self.assertRaises(p.Pause):
                client.call(['whoami'])
        self.assertEqual(transport.call_count, 3)

    def test_report_does_not_infer_risk_neutrality_from_net_exposure(self):
        import gen_report
        report = gen_report.build_html({}, '模拟报告', 'B')
        self.assertIn('净敞口低不证明市场风险中性', report)
        self.assertNotIn('平台模型多空对称', report)

    def test_report_validation_evidence_is_conservative(self):
        import gen_report
        cases = [
            ('20230101', '20240101', '2025-01-01', '止于样本外起点或之前'),
            ('20230101', '20250101', '2025-01-01', '止于样本外起点或之前'),
            ('20240101', '20260101', '2025-01-01', '跨越样本外起点'),
            ('20250101', '20260101', '2025-01-01', '跨越样本外起点'),
            ('20260101', '20270101', '2025-01-01', '训练/筛选使用情况未知'),
            ('20260101', '20270101', None, '尚不能确认真正样本外'),
            (None, None, '2025-01-01', '尚不能确认真正样本外'),
            ('20260101', None, '2025-01-01', '尚不能确认真正样本外'),
            ('invalid', '20270101', '2025-01-01', '日期无法核实'),
            ('20270101', '20260101', '2025-01-01', '验证区间无效'),
        ]
        for start, end, boundary, expected in cases:
            with self.subTest(start=start, end=end, boundary=boundary):
                problem = {'code': 'problem-a', 'time_segments': [
                    {'name': '后置验证', 'sdt': start, 'edt': end}],
                    'description': '后置验证不参与训练/选品/调参/模型选择'}
                data = {'problems': [problem], 'strategy_nav': {'candidate-a': {'oos_start': boundary}},
                        'experiments': [{'id': 'exp-a', 'metadata': {'problem_code': 'problem-a'},
                                         'strategies': [{'code': 'candidate-a', 'metrics': {'夏普比率': 99}}],
                                         'review_matrix': {'items': [
                                             {'strategy': 'candidate-a', 'segment_name': '后置验证', '夏普比率': .31}]}}]}
                report = gen_report.build_html(data, '测试报告', 'B')
                self.assertIn(expected, report)
                self.assertIn('未知：未核实训练、选品、调参及模型选择是否使用该区间', report)
                self.assertIn('尚不能确认真正样本外', report)
                self.assertIn('⑤ 候选明细与后置验证', report)
                self.assertNotIn('⑤ 候选明细与样本外', report)
                self.assertNotIn('止期早于当前日期', report)
                if not boundary:
                    self.assertIn('未知：候选阶段未取得 nav.oos_start', report)
                if not start or not end:
                    self.assertIn('未知：时间分段缺失或不完整', report)

    def test_report_missing_segments_and_metrics_do_not_fall_back_to_full_period(self):
        import gen_report
        data = {'experiments': [{'strategies': [
            {'code': 'full-only', 'metrics': {'夏普比率': 99}},
            {'code': 'validated', 'metrics': {'夏普比率': 1}}],
            'review_matrix': {'items': [
                {'strategy': 'validated', 'segment_name': '后置验证', '夏普比率': .31}]}}]}
        candidates = gen_report.build_candidates(data)
        self.assertEqual([candidate['code'] for candidate in candidates], ['validated', 'full-only'])
        self.assertIsNone(candidates[1]['sharpe_oos'])
        report = gen_report.build_html(data, '测试报告', 'B')
        self.assertIn('未知：时间分段缺失或不完整', report)
        self.assertIn('未知：候选阶段未取得 nav.oos_start', report)
        self.assertIn('尚不能确认真正样本外', report)

    def test_read_only_identity_blocks_paid_run_before_writes(self):
        self.cli.read_only = True
        worker = self.worker()
        with self.assertRaisesRegex(p.Pause, '只读'):
            worker.preflight()
        self.assertEqual(self.cli.writes, [])

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.wd = Path(self.tmp.name).resolve()
        (self.wd / 'raw').mkdir()
        (self.wd / 'fetched').mkdir()
        self.cfg = {'scenario': 'A', 'route': 'route-a', 'problems': ['problem-a', 'problem-b'],
                    'times': 2, 'threshold': 50, 'max_total_cent': 400, 'poll_seconds': .001}
        self.cli = FakeCLI()
        self.guard = patch.object(p.subprocess, 'run', side_effect=AssertionError('禁止未隔离 subprocess'))
        self.guard.start()
        self.addCleanup(self.guard.stop)
        quiet = patch('builtins.print')
        quiet.start()
        self.addCleanup(quiet.stop)
        self.addCleanup(self.tmp.cleanup)

    def worker(self, paid=True, retry=False):
        p.save(self.wd / 'config.json', self.cfg)
        a = SimpleNamespace(config=str(self.wd / 'config.json'), workdir=str(self.wd),
                            approval=str(self.wd / 'approval.json'), budget_cent=self.cfg['max_total_cent'],
                            retry_authorization=str(self.wd / 'retry.json') if retry else None,
                            yes_paid=paid, create_route=bool(self.cfg.get('route_json')), skz_bin='NEVER_REAL_SKZ')
        w = p.Pipeline(a, self.cli)
        selection = {'problems': [self.cli.call(['problem', 'get', c]) for c in self.cfg['problems']],
                     'routes': [] if self.cfg.get('route_json') else
                     [r for code in self.cfg.get('routes', [self.cfg.get('route')])
                      for r in self.cli.routes if r['code'] == code]}
        approval = {'approved': True, 'authorization_source': '隔离测试人工授权夹具', 'confirmed_at': p.now(),
                    'scope_workdir': str(self.wd), 'user_id': 'test-user', 'config_digest': p.digest(self.cfg),
                    'selection_digest': p.digest(selection), 'max_total_cent': self.cfg['max_total_cent'],
                    'route_create_approved': True, 'seven_fields_shown': True,
                    'route_json_digest': p.digest(self.cfg.get('route_json')),
                    'mechanism_review': {'duplicate_found': False, 'rationale': '模拟去重',
                                         'routes_snapshot_digest': p.digest(self.cli.routes)}}
        p.save(self.wd / 'approval.json', approval)
        p.save(self.wd / 'retry.json', {**approval, 'authorized': True,
                                      'max_retries_per_logical_task': 1, 'preserve_original_plan_budget': True,
                                      'execution_failure_codes': ['TASK_EXECUTION_FAILED']})
        return w

    def render(self, w, success):
        # 直接调用同一现有纯 Python 报告库；不启动外部 CLI。
        import gen_report
        def local_render(args, **kwargs):
            self.assertEqual(Path(args[1]).name, 'gen_report.py')
            d = p.load(w.wd / 'fetched' / 'summary.json')
            text = gen_report.build_html(d, '模拟报告', w.c['scenario'])
            (w.wd / 'report.html').write_text(text, encoding='utf-8')
            return SimpleNamespace(returncode=0, stdout='模拟本地生成', stderr='')
        with patch.object(p.subprocess, 'run', side_effect=local_render):
            w.fetch_report(success)

    def full(self, w):
        routes = w.preflight()
        w.approval()
        w.budget()
        w.route(routes)
        w.mine()
        result = w.explore()
        self.render(w, result)
        return result

    def test_full_order_and_resume_report(self):
        w = self.worker()
        self.assertTrue(self.full(w))
        cmds = self.cli.calls
        mines = [i for i, c in enumerate(cmds) if c[:2] == ['mine', 'start']]
        explores = [i for i, c in enumerate(cmds) if c[:2] == ['explore', 'start']]
        fetches = [i for i, c in enumerate(cmds) if c[:2] == ['experiment', 'get']]
        self.assertLess(max(mines), min(explores))
        self.assertLess(max(explores), min(fetches))
        events = [json.loads(line)['event'] for line in (self.wd / 'events.ndjson').read_text().splitlines()]
        self.assertLess(events.index('fetch_completed'), events.index('final_completed'))
        self.assertEqual(w.st['phase'], 'completed')
        before = len(self.cli.writes)
        self.assertEqual(self.worker().run(), 0)
        self.assertEqual(len(self.cli.writes), before)

    def seed(self, w, status='queued', fc='seed'):
        w.preflight()
        task = {'round': 1, 'route': 'route-a', 'status': status, 'attempts': [
            {'attempt_no': 1, 'fcRunId': fc, 'status': status, 'reserved_cent': 100,
             'createdAt': p.now(), 'receipt': 'raw/seed.json'}]}
        w.st['mine'].append(task)
        if fc:
            self.cli.runs['mine'].append({'fcRunId': fc, 'routeCode': 'route-a',
                                          'status': status, 'done': False, 'ok': False})
        w.persist()
        return task

    def test_resume_queued_dispatching_running(self):
        for status in ('queued', 'dispatching', 'running'):
            with self.subTest(status=status):
                w = self.worker()
                t = self.seed(w, status)
                self.cfg['times'] = w.c['times']
                w.mine()
                self.assertEqual(len([c for c in self.cli.writes if c[:2] == ['mine', 'start']]), 1)
                self.assertEqual(t['status'], 'ok')
                # 每个子场景用独立状态和模拟云端。
                w.st['mine'] = []
                w.st['mine_complete'] = False
                w.persist()
                self.cli.runs['mine'] = []
                self.cli.writes = []

    def test_monitor_window_pause_resumes_same_cloud_id_without_write(self):
        worker = self.worker()
        task = self.seed(worker, 'running', 'cloud-still-running')
        task['attempts'][0]['wait_started_epoch'] = 1
        self.cli.poll_override = {'status': 'running', 'done': False, 'ok': False}
        with self.assertRaisesRegex(p.Pause, '暂停而非取消'), patch.object(p.time, 'time', return_value=1000000):
            worker.wait('mine', task)
        state = p.load(self.wd / 'pipeline_state.json')
        self.assertEqual(state['mine'][0]['attempts'][0]['fcRunId'], 'cloud-still-running')
        self.assertEqual(state['mine'][0]['status'], 'running')
        resumed = self.worker()
        self.cli.poll_override = {'status': 'succeeded', 'done': True, 'ok': True}
        resumed.wait('mine', resumed.st['mine'][0])
        polls = [call for call in self.cli.calls if call[:2] == ['mine', 'poll']]
        self.assertEqual(polls, [['mine', 'poll', 'cloud-still-running']] * 2)
        self.assertEqual(self.cli.writes, [])

    def test_intent_without_id_pauses(self):
        w = self.worker()
        self.seed(w, 'submitting', None)
        with self.assertRaisesRegex(p.Pause, 'intent 无可信 ID'):
            w.preflight()
        self.assertEqual(self.cli.writes, [])
        self.assertEqual(w.spent(), 100)

    def test_unknown_timeout_one_write_only(self):
        w = self.worker()
        w.preflight()
        self.cli.write_timeout = True
        with self.assertRaisesRegex(p.Pause, 'intent 无可信 ID'):
            w.mine()
        self.assertEqual(len(self.cli.writes), 1)
        self.assertEqual(w.spent(), 100)
        with self.assertRaises(p.Pause):
            w.preflight()
        self.assertEqual(len(self.cli.writes), 1)

    def test_receipt_id_readback_adopts_without_write(self):
        w = self.worker()
        t = self.seed(w, 'queued', 'seed')
        t['attempts'][0]['fcRunId'] = None
        p.save(self.wd / 'raw' / 'seed.json', {'stdout': json.dumps({'fcRunId': 'seed'})})
        w.reconcile('mine', t)
        self.assertEqual(t['attempts'][0]['fcRunId'], 'seed')
        self.assertEqual(self.cli.writes, [])

    def test_authorized_retry_once_and_failed_budget(self):
        w = self.worker(retry=True)
        w.preflight()
        self.cli.failures = 1
        w.mine()
        t = w.st['mine'][0]
        self.assertEqual(len(t['attempts']), 2)
        self.assertEqual(t['attempts'][0]['last_poll']['status'], 'failed')
        self.assertEqual(w.spent(), 300)
        with self.assertRaisesRegex(p.Pause, '已补跑一次'):
            w.retry('mine', t)
        self.assertEqual(len(self.cli.writes), 3)

    def test_default_no_retry(self):
        w = self.worker()
        w.preflight()
        self.cli.failures = 1
        with self.assertRaisesRegex(p.Pause, '默认不补跑'):
            w.mine()
        self.assertEqual(len(self.cli.writes), 1)

    def test_retry_budget_preserves_remaining_plan(self):
        self.cfg['max_total_cent'] = 240
        w = self.worker(retry=True)
        w.preflight()
        self.cli.failures = 1
        with self.assertRaisesRegex(p.Pause, '预算不足'):
            w.mine()
        self.assertEqual(w.spent(), 100)
        self.assertEqual(len(self.cli.writes), 1)

    def test_low_retained_is_successful_early_stop(self):
        self.cli.retained = 1
        w = self.worker(retry=True)
        self.assertTrue(self.full(w))
        self.assertEqual(len(w.st['mine']), 1)
        self.assertEqual(len(w.st['mine'][0]['attempts']), 1)
        self.assertEqual(w.st['mine'][0]['status'], 'low_retain')
        self.assertEqual(len(w.st['explore']), 2)

    def test_zero_inventory_reports_failure_without_explore(self):
        self.cli.stock = 0
        w = self.worker()
        self.assertFalse(self.full(w))
        self.assertEqual(w.st['phase'], 'completed_failed')
        self.assertFalse(any(c[:2] == ['explore', 'start'] for c in self.cli.writes))
        self.assertIn('零库存', (self.wd / 'report.html').read_text())

    def test_inventory_errors_block(self):
        w = self.worker()
        w.preflight()
        original = self.cli.call
        def unknown(args, **kwargs):
            return {} if args == ['factor', 'summary'] else original(args, **kwargs)
        with patch.object(self.cli, 'call', side_effect=unknown), self.assertRaises(p.Pause):
            w.inventory('route-a')
        self.assertEqual(self.cli.writes, [])

    def test_done_false_and_timeout_never_retry(self):
        w = self.worker(retry=True)
        policy = w.retry_policy() if w.st.get('user_id') else None
        w.preflight()
        policy = w.retry_policy()
        for status, done in [('failed', False), ('running', False), ('dispatching', False), ('timeout', True)]:
            self.assertFalse(w.execution_failure({'status': status, 'done': done, 'ok': False,
                                                 'errorCode': 'TASK_EXECUTION_FAILED'}, policy))
        for msg in ('permission denied', 'config invalid', 'no_valid_factors', 'billing unknown'):
            self.assertFalse(w.execution_failure({'status': 'failed', 'done': True, 'ok': False,
                                                 'errorCode': 'TASK_EXECUTION_FAILED', 'errorMessage': msg}, policy))

    def test_readback_running_blocks_retry(self):
        w = self.worker(retry=True)
        t = self.seed(w)
        t['attempts'][0]['last_poll'] = {'status': 'failed', 'done': True, 'ok': False,
                                        'errorCode': 'TASK_EXECUTION_FAILED'}
        self.cli.poll_override = {'status': 'running', 'done': False, 'ok': False}
        with self.assertRaisesRegex(p.Pause, '不是授权'):
            w.retry('mine', t)
        self.assertEqual(self.cli.writes, [])

    def test_scene_b_multi_route_one_problem(self):
        self.cli.routes.append({'code': 'route-b', 'name': '另一个模拟机制'})
        self.cfg = {'scenario': 'B', 'routes': ['route-a', 'route-b'], 'problems': ['problem-a'],
                    'max_total_cent': 80, 'poll_seconds': .001}
        w = self.worker()
        self.assertTrue(self.full(w))
        self.assertFalse(w.st['mine'])
        self.assertEqual(len(w.st['explore']), 2)
        self.assertIn('route-b', (self.wd / 'report.html').read_text())

    def test_scene_a_approved_one_problem_one_round_stays_small(self):
        self.cfg.update(problems=['problem-a'], times=1, threshold=10, max_total_cent=120)
        worker = self.worker()
        self.assertTrue(self.full(worker))
        self.assertEqual([call[:2] for call in self.cli.writes],
                         [['mine', 'start'], ['explore', 'start']])
        self.assertEqual(len(worker.st['mine']), 1)
        self.assertEqual(len(worker.st['explore']), 1)
        self.assertEqual(worker.spent(), 120)
        self.assertIsNone(worker.st['route_intent'])
        self.assertEqual(self.worker().run(), 0)
        self.assertEqual(len(self.cli.writes), 2)

    def test_scene_b_one_route_ten_problems_explore_only(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'],
                    'problems': [f'problem-{i}' for i in range(10)],
                    'max_total_cent': 200, 'max_submissions': 10,
                    'expected_unit_prices': {'explore': 20}, 'poll_seconds': .001}
        w = self.worker()
        approval = p.load(self.wd / 'approval.json')
        approval['max_submissions'] = 10
        p.save(self.wd / 'approval.json', approval)
        self.assertTrue(self.full(w))
        self.assertEqual(len(self.cli.writes), 10)
        self.assertTrue(all(c[:2] == ['explore', 'start'] for c in self.cli.writes))
        self.assertFalse(w.st['mine'])
        self.assertIsNone(w.st['route_intent'])
        self.assertEqual(w.spent(), 200)
        self.assertEqual(p.load(self.wd / 'preflight.json')['explore_combinations'], 10)
        self.assertEqual(self.worker().run(), 0)
        self.assertEqual(len(self.cli.writes), 10)

    def test_scene_b_rejects_mining_fields(self):
        for field, value in [('times', 1), ('route_json', {'name': '禁止创建'}), ('route', 'route-a')]:
            with self.subTest(field=field), self.assertRaises(p.Pause):
                p.validate_config({'scenario': 'B', 'routes': ['route-a'], 'problems': ['p'],
                                   'max_total_cent': 20, field: value})

    def test_expected_price_change_before_first_preflight_blocks(self):
        self.cfg['expected_unit_prices'] = {'explore': 21}
        with self.assertRaisesRegex(p.Pause, '批准价格'):
            self.worker().preflight()
        self.assertFalse(self.cli.writes)

    def test_missing_experiment_list_uses_exact_get(self):
        w = self.worker()
        w.preflight()
        w.mine()
        w.explore()
        original = self.cli.call
        def missing(args, **kwargs):
            if args == ['experiment', 'list']:
                return {'items': []}
            if args[:2] == ['experiment', 'get']:
                return {'overview': {'n_backtests': 3, 'passed': 1, 'pass_rate': .333,
                                     'strategy_count': 0, 'total_elapsed': 2}}
            return original(args, **kwargs)
        with patch.object(self.cli, 'call', side_effect=missing):
            self.render(w, True)
        summary = p.load(self.wd / 'fetched' / 'summary.json')
        self.assertEqual(w.st['phase'], 'completed')
        for exp in summary['experiments']:
            self.assertIsNone(exp['list_row'])
            self.assertEqual(exp['metadata']['n_backtests'], 3)
            self.assertEqual(exp['field_sources']['n_backtests'], 'experiment get.overview')
        self.assertEqual(len(self.cli.writes), 4)

    def test_missing_metrics_and_read_errors_are_partial_not_zero(self):
        w = self.worker()
        w.preflight()
        w.mine()
        w.explore()
        original = self.cli.call
        def missing(args, **kwargs):
            if args == ['experiment', 'list']:
                return {'items': []}
            if args[:2] == ['experiment', 'get']:
                return {'overview': {'remaining_count': 9, 'remaining_passed': 9}}
            if args[:2] == ['experiment', 'strategies']:
                raise p.Pause('模拟只读失败')
            return original(args, **kwargs)
        with patch.object(self.cli, 'call', side_effect=missing):
            self.render(w, True)
        summary = p.load(self.wd / 'fetched' / 'summary.json')
        self.assertEqual(w.st['phase'], 'completed_partial')
        self.assertEqual(summary['pipeline']['outcome'], 'partial')
        self.assertNotIn('strategy_count', summary['experiments'][0]['metadata'])
        self.assertIsNone(summary['experiments'][0]['strategies_read_count'])
        report = (self.wd / 'report.html').read_text()
        self.assertIn('累计回测 <b>—</b>', report)
        self.assertIn('读取不完整', report)
        before = len(self.cli.writes)
        self.assertEqual(self.worker().run(), 1)
        self.assertEqual(len(self.cli.writes), before)

    def test_submission_cap_blocks_retry_even_with_authorization(self):
        self.cfg = {'scenario': 'B', 'routes': ['route-a'], 'problems': ['problem-a'],
                    'max_total_cent': 100, 'max_submissions': 1, 'poll_seconds': .001}
        w = self.worker(retry=True)
        approval = p.load(self.wd / 'approval.json')
        approval['max_submissions'] = 1
        p.save(self.wd / 'approval.json', approval)
        w.preflight()
        self.cli.failures = 1
        with self.assertRaisesRegex(p.Pause, '提交次数上限'):
            w.explore()
        self.assertEqual(len(self.cli.writes), 1)
        self.assertEqual(w.spent(), 20)

    def test_missing_oos_candidate_is_partial(self):
        w = self.worker()
        w.preflight()
        w.mine()
        w.explore()
        original = self.cli.call
        def missing(args, **kwargs):
            if args[:2] == ['experiment', 'strategies']:
                return {'items': [{'code': 'missing-oos', 'metrics': {}, 'verdict': {}}]}
            return original(args, **kwargs)
        with patch.object(self.cli, 'call', side_effect=missing):
            self.render(w, True)
        self.assertEqual(w.st['phase'], 'completed_partial')
        self.assertTrue(any('missing_oos_candidates' in g for g in w.st['data_gaps']))

    def test_route_json_rejects_describe(self):
        del self.cfg['route']
        self.cfg['route_json'] = {'name': '模拟新机制', 'key_inspect': '模拟内容', 'economic_logic': '模拟逻辑',
                                  'why_effective': '模拟原因', 'market_mechanism': '行为偏差',
                                  'failure_scenarios': ['模拟失效'], 'tags': ['模拟'],
                                  'compute_engine': 'TimeSeriesAstEngine'}
        for value in ('模拟研究要求', '', None):
            with self.subTest(value=value):
                self.cfg['route_json']['describe'] = value
                with self.assertRaisesRegex(p.Pause, '没有 describe/description 字段.*请移除 describe，研究要求写入 key_inspect'):
                    p.validate_config(self.cfg)
        self.assertEqual(self.cli.calls, [])
        self.assertEqual(self.cli.writes, [])

    def test_route_json_rejects_description(self):
        del self.cfg['route']
        self.cfg['route_json'] = {'name': '模拟新机制', 'key_inspect': '模拟内容', 'economic_logic': '模拟逻辑',
                                  'why_effective': '模拟原因', 'market_mechanism': '行为偏差',
                                  'failure_scenarios': ['模拟失效'], 'tags': ['模拟'],
                                  'compute_engine': 'TimeSeriesAstEngine'}
        for value in ('模拟研究要求', '', None):
            with self.subTest(value=value):
                self.cfg['route_json']['description'] = value
                with self.assertRaisesRegex(p.Pause, '没有 describe/description 字段.*请移除 description，研究要求写入 key_inspect'):
                    p.validate_config(self.cfg)
        self.assertEqual(self.cli.calls, [])
        self.assertEqual(self.cli.writes, [])

    def test_route_json_eight_fields_pass_and_unknown_field_rejected(self):
        del self.cfg['route']
        self.cfg['route_json'] = {'name': '模拟新机制', 'key_inspect': '模拟内容', 'economic_logic': '模拟逻辑',
                                  'why_effective': '模拟原因', 'market_mechanism': '行为偏差',
                                  'failure_scenarios': ['模拟失效'], 'tags': ['模拟'],
                                  'compute_engine': 'TimeSeriesAstEngine'}
        self.assertEqual(len(self.cfg['route_json']), 8)
        self.assertIsNone(p.validate_config(self.cfg))
        self.cfg['route_json']['unexpected'] = '模拟未知字段'
        with self.assertRaisesRegex(p.Pause, '只允许七个核心字段及可选 compute_engine'):
            p.validate_config(self.cfg)
        self.assertEqual(self.cli.calls, [])
        self.assertEqual(self.cli.writes, [])

    def test_route_create_and_strict_readback(self):
        del self.cfg['route']
        self.cfg['route_json'] = {'name': '模拟新机制', 'key_inspect': '模拟内容', 'economic_logic': '模拟逻辑',
                                  'why_effective': '模拟原因', 'market_mechanism': '行为偏差',
                                  'failure_scenarios': ['模拟失效'], 'tags': ['模拟']}
        w = self.worker()
        rr = w.preflight()
        w.route(rr)
        self.assertEqual(w.st['routes'], ['created-route'])
        self.assertEqual(len(self.cli.writes), 1)
        self.cli.routes[-1]['key_inspect'] = '模拟平台改写'
        with self.assertRaisesRegex(p.Pause, 'readback 不一致'):
            w.route(rr)
        self.assertEqual(len(self.cli.writes), 1)

    def test_dry_run_and_missing_approval_no_writes(self):
        w = self.worker(paid=False)
        self.assertEqual(w.run(), 0)
        self.assertFalse(self.cli.writes)
        w.a.yes_paid = True
        w.a.approval = None
        with self.assertRaisesRegex(p.Pause, '人审'):
            w.run()
        self.assertFalse(self.cli.writes)

    def test_unknown_cloud_dispatching_blocked(self):
        self.cli.runs['mine'].append({'fcRunId': 'external', 'routeCode': 'route-a',
                                      'status': 'dispatching', 'done': False, 'ok': False})
        with self.assertRaisesRegex(p.Pause, '未纳入'):
            self.worker().preflight()
        self.assertFalse(self.cli.writes)

    def test_paid_run_awaits_full_chain(self):
        w = self.worker()
        with patch.object(w, 'fetch_report', side_effect=lambda success: self.render(w, success)):
            # 绑定原始方法避免 mock 递归。
            original = p.Pipeline.fetch_report
            def render_original(success):
                import gen_report
                def local(args, **kwargs):
                    d = p.load(self.wd / 'fetched' / 'summary.json')
                    (self.wd / 'report.html').write_text(gen_report.build_html(d, '模拟完整调用', 'A'), encoding='utf-8')
                    return SimpleNamespace(returncode=0, stdout='', stderr='')
                with patch.object(p.subprocess, 'run', side_effect=local):
                    return original(w, success)
            w.fetch_report.side_effect = render_original
            self.assertEqual(w.run(), 0)
        self.assertEqual(w.st['phase'], 'completed')
        self.assertEqual(len(w.st['explore']), 2)

    def test_intent_fsync_precedes_write(self):
        w = self.worker()
        w.preflight()
        original = self.cli.call
        def inspect(args, write=False, **kwargs):
            if write:
                state = p.load(self.wd / 'pipeline_state.json')
                aa = [a for kind in ('mine', 'explore') for task in state[kind] for a in task['attempts']]
                self.assertTrue(any(a['status'] == 'submitting' and a['fcRunId'] is None for a in aa))
                self.assertGreater(sum(a['reserved_cent'] for a in aa), 0)
            return original(args, write=write, **kwargs)
        with patch.object(self.cli, 'call', side_effect=inspect):
            w.mine()

    def test_price_drift_and_approval_scope_block_writes(self):
        w = self.worker()
        w.preflight()
        original = self.cli.call
        def drift(args, **kwargs):
            d = original(args, **kwargs)
            if args[:2] == ['wallet', 'check']:
                d['unitPriceCent'] += 1
                d['requiredCent'] = d['unitPriceCent'] * d['qty']
            return d
        with patch.object(self.cli, 'call', side_effect=drift), self.assertRaisesRegex(p.Pause, '单价变化'):
            w.mine()
        ap = p.load(self.wd / 'approval.json')
        ap['scope_workdir'] = '/wrong'
        p.save(self.wd / 'approval.json', ap)
        with self.assertRaisesRegex(p.Pause, '人审'):
            w.mine()
        self.assertFalse(self.cli.writes)

    def test_corrupt_local_success_is_not_silently_skipped(self):
        w = self.worker()
        t = self.seed(w)
        t.update(status='ok', retained=1)
        with self.assertRaisesRegex(p.Pause, '缺少成功读回'):
            w.validate_state()
        self.assertFalse(self.cli.writes)

    def test_report_integrity_and_workdir_lock(self):
        w = self.worker()
        self.full(w)
        (self.wd / 'report.html').write_text('损坏', encoding='utf-8')
        with self.assertRaises(p.Pause):
            w.run()
        with (self.wd / 'pipeline.lock').open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.assertEqual(p.main(['--config', str(self.wd / 'config.json'), '--workdir', str(self.wd)]), 2)


if __name__ == '__main__':
    unittest.main(verbosity=2)
