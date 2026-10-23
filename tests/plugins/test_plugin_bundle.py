#!/usr/bin/env python3
"""Golden and deterministic behavior tests for the generated SKZ plugins."""

from __future__ import annotations

import importlib.util
import io
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PLUGINS = ROOT / "plugins"
AUTHORING = ROOT / "plugin-src"
BOOKS = tuple(name.removeprefix("skz-") for name in (AUTHORING / "skills.txt").read_text().splitlines())
TARGETS = ("claude", "codex", "openclaw", "hermes", "dsh", "workbuddy")
SCRIPTS = AUTHORING / "common" / "scripts"
GOLDENS = json.loads((Path(__file__).parent / "golden_prompts.json").read_text(encoding="utf-8"))
OPEN_API_ROUTES = set(json.loads((Path(__file__).parent / "open_api_routes.json").read_text(encoding="utf-8")))


def load_openapi_sync():
    script = ROOT / "scripts/release/sync_openapi_refs.py"
    spec = importlib.util.spec_from_file_location("sync_openapi_refs", script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_script(name: str, *args: str, stdin: object | None = None, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(SCRIPTS / name), *args],
        input=None if stdin is None else json.dumps(stdin, ensure_ascii=False),
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )


class PluginBundleTests(unittest.TestCase):
    def test_openapi_classifies_effects_instead_of_http_methods(self) -> None:
        book = AUTHORING / 'books/skz-openapi'
        text = (book / 'SKILL.md').read_text(encoding='utf-8')
        for obsolete in ('写操作（POST/PATCH/DELETE）', '仅在需要转用 skz CLI 时读取',
                         '写操作（创建/导入/promote/打标/memo/状态/删除）一律'):
            self.assertNotIn(obsolete, text)
        for required in ('无论直接 API 还是 CLI', 'HTTP 方法不等于读写或确认策略',
                         '端点语义、费用或影响未知时先暂停核实', '不通过试调用探测'):
            self.assertIn(required, text)
        cases = (
            ('strategy/llm-config-update.md', 'PUT /strategy/llm-configs/{id}',
             ('写操作', '确认精确配置 ID', '只读策略下禁止执行', '不自动重发')),
            ('strategy/tasks-poll.md', 'POST /strategy/tasks/poll',
             ('只读查询', '不创建任务或发起扣费', '不因 POST 要求写授权')),
            ('payment/wallet-summary.md', 'POST /payment/wallet/summary',
             ('只读查询', '不发起扣费', '不因 POST 要求写授权')),
            ('strategy/tasks-preview.md', 'POST /strategy/tasks/preview',
             ('只读预估', '不创建任务或发起扣费', '不因 POST 要求写授权',
              '预估成功不代表已授权创建')),
            ('strategy/llm-config-probe.md', 'POST /strategy/llm-configs/probe',
             ('可能消耗模型额度', '调用前确认', '只读策略下停止', '不自动重试')),
        )
        for path, route, boundaries in cases:
            with self.subTest(route=route):
                reference = (book / 'references' / path).read_text(encoding='utf-8')
                self.assertIn(f'`{route}`', reference)
                self.assertIn(f'(references/{path})', text)
                rule = reference.split('## 副作用与授权', 1)[1].split('\n## ', 1)[0]
                self.assertIn('operating-contract.md#直接-api-调用边界', rule)
                for boundary in boundaries:
                    self.assertIn(boundary, rule)

    def test_direct_api_inherits_identity_and_unknown_write_boundaries(self) -> None:
        contract = (AUTHORING / 'common/references/operating-contract.md').read_text(encoding='utf-8')
        rule = contract.split('## 直接 API 调用边界', 1)[1].split('\n## ', 1)[0]
        for boundary in ('不能作为绕过 CLI 的通道', '不默认可写',
                         '不得自行换身份、换 key、提高权限', 'readOnly:true',
                         '平台 token 与用户模型 API Key', '命令参数或持久脚本',
                         'PUT 模型配置更新', '缺少费用依据先暂停',
                         '所有写请求不自动重试', '结果为“未知”',
                         '查不到或查询失败仍是未知', '不盲重发、不创建替代任务',
                         'HTTP 成功也须检查端点业务回执及批量逐项结果'):
            with self.subTest(boundary=boundary):
                self.assertIn(boundary, rule)
        examples = (AUTHORING / 'books/skz-openapi/references/examples.md').read_text(encoding='utf-8')
        for boundary in ('operating-contract.md#直接-api-调用边界', '安全注入当前身份凭据',
                         '不整段无确认运行', '不重新运行示例补取回执'):
            self.assertIn(boundary, examples)

    def test_long_task_waiting_uses_shared_host_lifecycle_contract(self):
        contract = (AUTHORING / 'common/references/operating-contract.md').read_text()
        rule = contract.split('## 长任务等待与宿主生命周期', 1)[1].split('\n## ', 1)[0]
        for required in ('云端业务任务', '本地监控进程', '宿主托管任务',
                         '默认使用前台阻塞等待', '持久托管和可靠完成唤醒',
                         '任一保证缺失或未知', '不得后台启动后结束回合',
                         'Multica', '不得后台启动后退出回合', '`nohup` 或裸 `&`',
                         '本地停止、工具超时或回合结束不等于云端取消',
                         '工具返回后台句柄本身不证明', '保存真实 `fcRunId`、配置、账本',
                         '监控暂停，云端状态待核验', '不为恢复监控重发 `start`',
                         '无可信 ID 则暂停', '同一配置和独占目录'):
            with self.subTest(required=required):
                self.assertIn(required, rule)
        for book in ('skz-guide', 'skz-auto-research'):
            text = (AUTHORING / 'books' / book / 'SKILL.md').read_text()
            with self.subTest(book=book):
                self.assertIn('references/operating-contract.md#长任务等待与宿主生命周期', text)
                self.assertNotIn('挂后台等待即可', text)
                self.assertIn('不重发任务', text)

    def test_research_identity_guidance_has_one_conservative_contract(self):
        contract = (AUTHORING / 'common/references/operating-contract.md').read_text()
        for text in ('(problemCode, routeCode)', '服务端实际并发锁粒度',
                     'createdAt', '缺失、null、空字符串均为未知', '没有可信 ID'):
            self.assertIn(text, contract)
        paths = ('skz-guide/SKILL.md', 'skz-auto-research/references/cli-contract.md',
                 'skz-auto-research/references/scenario-a-replay.md',
                 'skz-openapi/references/strategy/explore-create.md')
        for path in paths:
            with self.subTest(path=path):
                text = (AUTHORING / 'books' / path).read_text()
                self.assertIn('operating-contract.md#研究任务键与可信接管', text)
                for obsolete in ('同一条同时两个在跑', '同一条 route 同时两个在跑',
                                 '**不含 problemCode**', '按 `problem` 去重',
                                 '的 `createdAt` 确认已触发'):
                    self.assertNotIn(obsolete, text)

    def test_candidate_validation_terms_are_consistent(self) -> None:
        paths = (
            'skz-candidate/SKILL.md',
            'skz-strategy/SKILL.md',
            'skz-auto-research/SKILL.md',
            'skz-auto-research/references/report-spec.md',
            'skz-auto-research/references/scenario-a-replay.md',
            'skz-auto-research/references/scenario-b-newproblem.md',
        )
        for path in paths:
            with self.subTest(path=path):
                text = (AUTHORING / 'books' / path).read_text(encoding='utf-8')
                self.assertIn('后置验证', text)
                self.assertIn('未知', text)
                self.assertIn('nav.oos_start', text)
                for claim in ('按样本外夏普排序', '样本外只来自', '样本外只取',
                              '候选样本外站得住', '训练段之外的时段按样本外对待',
                              '后置验证段是否已过期', '样本外夏普中位 0.31',
                              '真held-out证据', '只看这条线之后'):
                    self.assertNotIn(claim, text)
        strategy = (AUTHORING / 'books/skz-strategy/SKILL.md').read_text(encoding='utf-8')
        for boundary in ('仅说明时间关系', '不能独立证明未参与训练/筛选',
                         '缺少实际执行证据时保留未知', '单独计算指标也不自动证明独立性'):
            self.assertIn(boundary, strategy)

    def test_miner_curl_example_creates_once_and_fails_closed(self) -> None:
        text = (AUTHORING / "books/skz-openapi/references/examples.md").read_text(encoding="utf-8")
        example = re.search(r"```bash\n(.*?)```", text, re.S).group(1)
        example = example[example.index("# 5)"):]
        harness = '''
KEY=mock
BASE=https://mock.invalid
curl() {
  printf '%s\\n' "$*" >> "$CALLS"
  case "$*" in
    *"$BASE/strategy/miner/runs"*)
      printf '%s\\n%s' "$RECEIPT" "$HTTP_STATUS"
      return "$CREATE_EXIT"
      ;;
    *"$BASE/strategy/miner/poll"*)
      printf '%s' "${!#}" > "$BODY"
      return "$POLL_EXIT"
      ;;
    *) return 99 ;;
  esac
}
'''
        valid = {"fcRunId": "fc-run-20260724-abc123", "status": "queued"}
        cases = [
            ("success", json.dumps(valid), "200", 0, 0, True),
            ("created", json.dumps(valid), "201", 0, 0, True),
            ("timeout", "", "000", 28, 0, False),
            ("lost-receipt", json.dumps(valid), "200", 18, 0, False),
            ("http-error", json.dumps(valid), "409", 22, 0, False),
            ("redirect", json.dumps(valid), "302", 0, 0, False),
            ("business-error", json.dumps({**valid, "errorCode": "DENIED"}), "200", 0, 0, False),
            ("wrong-status", json.dumps({**valid, "status": "failed"}), "200", 0, 0, False),
            ("missing-status", '{"fcRunId":"valid-id"}', "200", 0, 0, False),
            ("invalid-json", "{", "200", 0, 0, False),
            ("empty-response", "", "200", 0, 0, False),
            ("multiple", json.dumps(valid) + "\n" + json.dumps(valid), "200", 0, 0, False),
            ("array-response", json.dumps([valid]), "200", 0, 0, False),
            ("null-response", "null", "200", 0, 0, False),
            ("missing-id", '{"status":"queued"}', "200", 0, 0, False),
            ("poll-fails", json.dumps(valid), "200", 0, 22, True),
        ]
        for name, invalid_id in [("null", None), ("empty", ""), ("space", " "),
                                 ("newline", "run\nid"), ("quote", 'run"id'),
                                 ("number", 42), ("object", {}), ("array", [])]:
            cases.append((name, json.dumps({**valid, "fcRunId": invalid_id}), "200", 0, 0, False))
        with tempfile.TemporaryDirectory() as temporary:
            for name, receipt, http_status, create_exit, poll_exit, polls in cases:
                with self.subTest(name=name):
                    calls = Path(temporary) / f"{name}.calls"
                    body = Path(temporary) / f"{name}.body"
                    result = subprocess.run(
                        ["bash", "-c", harness + example], text=True, capture_output=True,
                        env={**os.environ, "RECEIPT": receipt, "HTTP_STATUS": http_status,
                             "CREATE_EXIT": str(create_exit), "POLL_EXIT": str(poll_exit),
                             "CALLS": str(calls), "BODY": str(body)}, check=False,
                    )
                    requests = calls.read_text().splitlines()
                    self.assertEqual(sum("/strategy/miner/runs" in request for request in requests), 1)
                    self.assertEqual(len(requests), 2 if polls else 1)
                    if polls:
                        self.assertIn("/strategy/miner/poll", requests[1])
                        self.assertEqual(json.loads(body.read_text()), {"runIds": [valid["fcRunId"]]})
                        self.assertEqual(result.returncode, poll_exit, result.stderr)
                    else:
                        self.assertFalse(body.exists())
                        self.assertNotEqual(result.returncode, 0)

    def test_memo_append_example_fails_closed(self) -> None:
        text = (AUTHORING / "books/skz-strategy/SKILL.md").read_text(encoding="utf-8")
        section = text.split("### 追加要先读回来（它是覆盖写）", 1)[1]
        example = re.search(r"```bash\n(.*?)```", section, re.S).group(1)
        entry = "2026-07-31 暂停：近 20 日回撤 -18%（阈值 -15%），等下周复盘"
        harness = '''
skz() {
  if [ "$2" = get ]; then
    printf '%s' "$READ_JSON"
    return "$READ_STATUS"
  fi
  printf 'write\\n' >> "$CALLS"
  cat > "$BODY"
  return "$WRITE_STATUS"
}
'''
        cases = [
            ("existing", json.dumps({"memo": "旧记录\n含 %s、反斜杠\\\n\n"}), 0, 0, True),
            ("empty", '{"memo":""}', 0, 0, True),
            ("auth", '{"memo":"旧记录"}', 4, 0, False),
            ("timeout", "", 7, 0, False),
            ("invalid-json", "{", 0, 0, False),
            ("no-json", "", 0, 0, False),
            ("missing", "{}", 0, 0, False),
            ("null", '{"memo":null}', 0, 0, False),
            ("number", '{"memo":42}', 0, 0, False),
            ("object", '{"memo":{}}', 0, 0, False),
            ("array", '{"memo":[]}', 0, 0, False),
            ("multiple", '{"memo":"a"}\n{"memo":"b"}', 0, 0, False),
            ("write-unknown", '{"memo":"旧记录"}', 0, 7, True),
        ]
        with tempfile.TemporaryDirectory() as temporary:
            for name, payload, read_status, write_status, writes in cases:
                with self.subTest(name=name):
                    calls = Path(temporary) / f"{name}.calls"
                    body = Path(temporary) / f"{name}.body"
                    result = subprocess.run(
                        ["bash", "-c", harness + example], text=True, capture_output=True,
                        env={**os.environ, "READ_JSON": payload, "READ_STATUS": str(read_status),
                             "WRITE_STATUS": str(write_status), "CALLS": str(calls), "BODY": str(body)},
                        check=False,
                    )
                    if writes:
                        self.assertEqual(calls.read_text(), "write\n")
                        self.assertEqual(body.read_text(), json.loads(payload)["memo"] + "\n" + entry + "\n")
                        self.assertEqual(result.returncode, write_status, result.stderr)
                    else:
                        self.assertFalse(calls.exists())
                        self.assertFalse(body.exists())
                        self.assertNotEqual(result.returncode, 0)

    def test_skill_local_python_caches_are_not_bundled(self) -> None:
        with patch.object(sys, "path", [str(ROOT / "scripts/release"), *sys.path]):
            import build_plugins
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "authoring"
            book = source / "books/skz-test"
            (book / "scripts/__pycache__").mkdir(parents=True)
            (book / "SKILL.md").write_text("test", encoding="utf-8")
            (book / "scripts/driver.py").write_text("print('test')", encoding="utf-8")
            (book / "scripts/__pycache__/driver.pyc").write_bytes(b"cache")
            (book / "scripts/driver.pyc").write_bytes(b"cache")
            for name in ("references", "scripts"):
                (source / "common" / name).mkdir(parents=True)
            with patch.object(build_plugins, "AUTHORING", source), patch.object(build_plugins, "BOOKS", ("test",)):
                build_plugins._copy_skills(root / "plugin", "codex")
            output = root / "plugin/skills/skz-test/scripts"
            self.assertTrue((output / "driver.py").is_file())
            self.assertFalse((output / "__pycache__").exists())
            self.assertFalse((output / "driver.pyc").exists())

    def test_auto_research_is_bundled_with_its_safe_driver(self) -> None:
        self.assertIn("auto-research", BOOKS)
        for target in TARGETS:
            root = PLUGINS / target / "plugins/skz/skills/skz-auto-research"
            for name in ("pipeline.py", "gen_report.py", "run_pipeline.sh"):
                self.assertTrue((root / "scripts" / name).is_file(), f"{target}: {name}")
            self.assertFalse((root / "scripts/orchestrate.py").exists())

    def test_auto_research_references_have_no_broken_relative_links(self) -> None:
        root = PLUGINS / "codex/plugins/skz/skills/skz-auto-research"
        for path in root.rglob("*.md"):
            for target in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
                if target.startswith(("http://", "https://", "mailto:", "#")):
                    continue
                self.assertTrue((path.parent / target.split("#", 1)[0]).is_file(), f"{path}: {target}")

    def test_openapi_sync_rewrites_links_for_the_offline_bundle(self) -> None:
        module = load_openapi_sync()

        source = Path("/docs/api")
        output = Path("/bundle/strategy/create.md")
        pages = {
            "https://docs.shengkezhi.com/api/strategy/list": Path("/bundle/strategy/list.md")
        }
        body = "[本地](./list) [带后缀](./list.md) [站外](../../pricing)"
        rendered = module.rewrite_links(
            body,
            module.source_url(source / "strategy/create.md", source),
            output,
            pages,
        )
        self.assertEqual(
            rendered,
            "[本地](list.md) [带后缀](list.md) [站外](https://docs.shengkezhi.com/pricing)",
        )

    def test_openapi_sync_normalizes_json_fences_before_distribution(self) -> None:
        module = load_openapi_sync()
        for ending in ("}", "]"):
            opening = "{" if ending == "}" else "["
            for newline in ("", "\n"):
                broken = f"```json\n{opening}\n{ending}```{newline}"
                expected = f"```json\n{opening}\n{ending}\n```{newline}"
                with self.subTest(ending=ending, newline=newline):
                    self.assertEqual(module.normalize_code_fences(broken), expected)
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            path = source / "response.md"
            path.write_text('---\ntitle: 示例\ndescription: 示例\n---\n```json\n{\n}```\n')
            rendered = module.render(path, source, source / "output.md", {})
            self.assertTrue(rendered.endswith("```json\n{\n}\n```\n"))
            self.assertEqual(module.normalize_code_fences(rendered), rendered)

    def test_openapi_fences_preserve_valid_examples_and_inline_backticks(self) -> None:
        normalize = load_openapi_sync().normalize_code_fences
        examples = (
            '行内 `}` 和 `` }``` `` 不变\n}```\n',
            '```bash\nprintf "}```"\n}```\n```\n',
            '```json\n{"text": "}```"}\n```\n',
            '````markdown\n```json\n}```\n````\n',
            '~~~json\n{}\n~~~~\n',
            '```json fragment\n{"data": ...}\n```\n',
            '```json placeholder\n<response>\n```\n',
        )
        for example in examples:
            with self.subTest(example=example):
                self.assertEqual(normalize(example), example)

    def test_openapi_resync_is_stable_and_bad_fences_do_not_replace_output(self) -> None:
        module = load_openapi_sync()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            path = source / "response.md"
            header = ('---\ntitle: 示例\ndescription: 示例\n---\n'
                      '<ApiDebugger method="GET" path="/example" />\n')
            path.write_text(header + '```json\n{\n}```\n', encoding="utf-8")
            output = root / "output"
            routes = root / "routes.json"
            with patch.object(module, "OUTPUT", output), patch.object(module, "ROUTES", routes), \
                    patch.object(sys, "argv", ["sync_openapi_refs.py", str(source)]), \
                    patch.object(sys, "stdout", io.StringIO()):
                module.main()
                expected = (output / path.name).read_bytes()
                expected_routes = routes.read_bytes()
                module.main()
                self.assertEqual((output / path.name).read_bytes(), expected)
                self.assertEqual(routes.read_bytes(), expected_routes)
                path.write_text(header + '```json\n{}\n', encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "response.md.*unclosed"):
                    module.main()
                self.assertEqual((output / path.name).read_bytes(), expected)
                self.assertEqual(routes.read_bytes(), expected_routes)

    def test_openapi_fences_reject_unclosed_blocks_and_invalid_complete_json(self) -> None:
        normalize = load_openapi_sync().normalize_code_fences
        examples = (
            '```json\n{}\n',
            '````json\n{}\n```\n',
            '```json\n{}\n~~~\n',
            '```json\n{}\n``` trailing\n',
            '```json\n{"data": ...}\n```\n',
            '```json\n{broken\n}```\n',
            '```bash\necho test\n',
        )
        for example in examples:
            with self.subTest(example=example):
                with self.assertRaises(ValueError):
                    normalize(example)

    def test_all_openapi_references_have_closed_fences_and_parseable_json(self) -> None:
        normalize = load_openapi_sync().normalize_code_fences
        references = AUTHORING / "books/skz-openapi/references"
        for path in sorted(references.rglob("*.md")):
            with self.subTest(path=path.relative_to(references)):
                text = path.read_text(encoding="utf-8")
                self.assertEqual(normalize(text), text)

    def test_openapi_references_have_no_broken_relative_links(self) -> None:
        references = AUTHORING / "books/skz-openapi/references"
        for path in references.rglob("*.md"):
            text = path.read_text(encoding="utf-8")
            for target in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", text):
                target = target.strip()
                if target.startswith(("http://", "https://", "mailto:", "#")):
                    continue
                relative = target.split("#", 1)[0]
                shared = AUTHORING / "common/references" / path.parent.relative_to(references) / relative
                self.assertTrue(
                    (path.parent / relative).is_file() or shared.resolve().is_file(),
                    f"broken link in {path.relative_to(AUTHORING)}: {target}",
                )

    def test_openapi_references_cover_the_public_route_manifest(self) -> None:
        routes = set()
        references = AUTHORING / "books/skz-openapi/references"
        for path in references.rglob("*.md"):
            text = path.read_text(encoding="utf-8")
            match = re.search(r"`((?:GET|POST|PUT|PATCH|DELETE) /[^` ]+)`", text)
            if match:
                self.assertNotIn(match.group(1), routes, f"duplicate route: {match.group(1)}")
                routes.add(match.group(1))
        self.assertEqual(routes, OPEN_API_ROUTES)

    def test_manifest_includes_concrete_client_paths(self) -> None:
        source = (ROOT / "src/client.rs").read_text(encoding="utf-8")
        normalize = lambda path: re.sub(r"\{[^}]*\}", "{}", path)
        known = {normalize(route.split(" ", 1)[1]) for route in OPEN_API_ROUTES}
        paths = re.findall(r'"[^"\n]*?(/(?:market|strategy|research|payment)/[^"\n\s]*)"', source)
        for path in paths:
            # 两个 URL path_segments_mut 构造器使用以 / 结尾的基础前缀，非独立端点。
            if path.endswith("/"):
                continue
            self.assertIn(normalize(path), known, f"client 实际调用路径未登记: {path}")

    def test_cli_client_declares_the_public_route_manifest(self) -> None:
        source = (ROOT / "src/client.rs").read_text(encoding="utf-8")
        routes = {
            match.rstrip("。；,.")
            for match in re.findall(
                r"(?:`)?((?:GET|POST|PUT|PATCH|DELETE) /(?:market|strategy|research|payment)/[^`\s，。；）]+)",
                source,
            )
        }
        self.assertEqual(routes, OPEN_API_ROUTES)

    def test_each_target_contains_one_native_skz_plugin(self) -> None:
        manifest = json.loads((PLUGINS / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["contract"], "4.3")
        self.assertEqual(manifest["plugin"], "skz")
        self.assertEqual(set(manifest["targets"]), set(TARGETS))
        names = [f"skz-{book}" for book in BOOKS]
        self.assertEqual(manifest["skills"], names)
        self.assertEqual(len(names), len(set(names)))
        self.assertTrue(all(re.fullmatch(r"skz-[a-z0-9-]+", name) for name in names))
        self.assertEqual(set(names), {p.name for p in (AUTHORING / "books").iterdir() if p.is_dir()})
        for target in TARGETS:
            root = PLUGINS / target / "plugins"
            self.assertEqual([path.name for path in root.iterdir() if path.is_dir()], ["skz"])
            self.assertEqual(set(names), {p.name for p in (root / "skz/skills").iterdir() if p.is_dir()})
        self.assertTrue((PLUGINS / "claude/plugins/skz/.claude-plugin/plugin.json").is_file())
        self.assertTrue((PLUGINS / "codex/plugins/skz/.codex-plugin/plugin.json").is_file())
        self.assertTrue((PLUGINS / "openclaw/.claude-plugin/marketplace.json").is_file())
        self.assertTrue((PLUGINS / "hermes/plugins/skz/plugin.yaml").is_file())
        self.assertTrue(
            (PLUGINS / "dsh/plugins/skz/skills/skz-guide/SKILL.md").is_file()
        )
        self.assertFalse((PLUGINS / "dsh/plugins/skz/plugin.yaml").exists())
        market = json.loads((PLUGINS / "workbuddy/.codebuddy-plugin/marketplace.json").read_text(encoding="utf-8"))
        self.assertEqual(market["name"], "skz")
        self.assertEqual(market["plugins"][0]["source"], "./plugins/skz")
        plugin = PLUGINS / "workbuddy" / market["plugins"][0]["source"]
        self.assertEqual(json.loads((plugin / ".codebuddy-plugin/plugin.json").read_text(encoding="utf-8"))["name"], "skz")

    def test_golden_prompt_set_covers_all_skills_and_boundaries(self) -> None:
        expected = {case["expected_skill"] for case in GOLDENS}
        self.assertEqual(expected, {None, *(f"skz-{book}" for book in BOOKS)})
        kinds = {case["kind"] for case in GOLDENS}
        self.assertTrue({"direct", "indirect", "negative", "safety"}.issubset(kinds))
        for case in GOLDENS:
            self.assertTrue(case["prompt"].strip())
            if case["expected_skill"]:
                skill = case["expected_skill"].removeprefix("skz-")
                frontmatter = (AUTHORING / "books" / f"skz-{skill}" / "SKILL.md").read_text(encoding="utf-8").split("---", 2)[1]
                self.assertIn(f"name: skz-{skill}", frontmatter)

    def test_skill_division_table_names_every_book(self) -> None:
        expected = {f"skz-{book}" for book in BOOKS}
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        division = contract.split("### 技能职责\n", 1)[1].split("\n## ", 1)[0]
        found = re.findall(r"^\| `(skz-[a-z0-9-]+)` \|", division, re.M)
        self.assertCountEqual(found, expected)
        for book in BOOKS:
            skill = (AUTHORING / "books" / f"skz-{book}" / "SKILL.md").read_text(encoding="utf-8")
            self.assertIn("references/operating-contract.md#公共安装工具与职责", skill)
            self.assertNotIn("册分工：", skill)
            self.assertNotIn("可执行工具均为只读或纯校验", skill)
            self.assertNotIn("scripts/preflight.py --operation read|write|paid", skill)
        for tool in ("preflight", "resume", "validate_plan", "verify_write"):
            self.assertEqual(contract.count(f"- `scripts/{tool}.py"), 1)
        for safety in ("approved:false", "绝不代替用户批准", "绝不重放写命令"):
            self.assertIn(safety, contract)

    def test_skill_install_targets_match_cli(self) -> None:
        rust = (ROOT / "src/plugin.rs").read_text(encoding="utf-8")
        cli_targets = tuple(re.findall(r'Self::\w+ => "([a-z]+)"', rust))
        self.assertEqual(cli_targets, TARGETS)
        expected = "|".join((*TARGETS, "all"))
        listed = re.compile(r"skz plugin install <([^>]+)>")
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        self.assertEqual(listed.findall(contract), [expected])
        for setting in ("needs_install", "stale", "skill-filesystem", "tool-skill"):
            self.assertIn(setting, contract)
        for book in BOOKS:
            skill = (AUTHORING / "books" / f"skz-{book}" / "SKILL.md").read_text(encoding="utf-8")
            self.assertEqual(listed.findall(skill), [])
            self.assertIn("references/operating-contract.md#公共安装工具与职责", skill)

    def test_research_examples_do_not_override_approved_plan_or_live_prices(self) -> None:
        references = AUTHORING / "books/skz-auto-research/references"
        replay = (references / "scenario-a-replay.md").read_text(encoding="utf-8")
        contract = (references / "cli-contract.md").read_text(encoding="utf-8")
        for obsolete in ("在 ≥5 个", "选定 ≥5 个", "不够 5 个时现场建", "for round in 1..10:",
                         "核对 9 个", "遵循 ≤10 个且高相关的契约", "times=10"):
            self.assertNotIn(obsolete, replay)
        for rule in ("用户可明确批准较小实验", "不是平台硬限制", "七个核心字段", "可选 `compute_engine`",
                     "不自动补建问题或增加轮次", "times（已批准轮次）", "retained < threshold"):
            self.assertIn(rule, replay)
        for document in (replay, contract):
            self.assertIn("skz-create-problem", document)
            self.assertNotIn('"time_segments":', document)
        self.assertIn("经验建议，不是平台硬限制", contract)
        self.assertIn("[billing.md](billing.md)", contract)
        self.assertIn("`unitPriceCent`、`requiredCent`、`availableCent`、`affordable`", contract)
        self.assertNotRegex(contract, r"¥\d|(?:3000|600|200) 分|\"unitPriceCent\":\d")

    def test_rendered_targets_are_identical_and_self_contained(self) -> None:
        for book in BOOKS:
            canonical = PLUGINS / "codex" / "plugins" / "skz" / "skills" / f"skz-{book}"
            expected = {
                path.relative_to(canonical): path.read_bytes()
                for path in canonical.rglob("*")
                if path.is_file()
            }
            self.assertIn(Path("SKILL.md"), expected)
            self.assertIn(Path("agents/openai.yaml"), expected)
            self.assertIn(Path("references/operating-contract.md"), expected)
            self.assertIn(Path("scripts/preflight.py"), expected)
            for target in TARGETS:
                root = PLUGINS / target / "plugins" / "skz" / "skills" / f"skz-{book}"
                actual = {
                    path.relative_to(root): path.read_bytes()
                    for path in root.rglob("*")
                    if path.is_file()
                }
                self.assertEqual(actual, expected, f"{target}/{book} drifted")

    def test_skill_metadata_and_progressive_disclosure(self) -> None:
        for book in BOOKS:
            root = AUTHORING / "books" / f"skz-{book}"
            skill = (root / "SKILL.md").read_text(encoding="utf-8")
            self.assertRegex(skill, rf"\A---\nname: skz-{book}\ndescription: .+\n---\n")
            self.assertIn("references/operating-contract.md", skill)
            self.assertIn("references/communication.md", skill)
            self.assertNotIn("## I/O 契约", skill)
            metadata = (root / "agents" / "openai.yaml").read_text(encoding="utf-8")
            self.assertRegex(metadata, r'display_name: ".+"')
            self.assertRegex(metadata, r'short_description: ".{25,64}"')
            self.assertIn(f"$skz-{book}", metadata)

    def test_candidate_and_registered_strategy_boundaries(self) -> None:
        candidate = (AUTHORING / "books/skz-candidate/SKILL.md").read_text(encoding="utf-8")
        strategy = (AUTHORING / "books/skz-strategy/SKILL.md").read_text(encoding="utf-8")
        guide = (AUTHORING / "books/skz-guide/SKILL.md").read_text(encoding="utf-8")

        self.assertIn("skz experiment review-matrix", candidate)
        self.assertIn("skz promote start", candidate)
        self.assertNotRegex(candidate, r"(?m)^skz strategy (status|register|metrics|nav|positions)")
        self.assertNotRegex(candidate, r"(?m)^skz gift ")

        self.assertIn("skz strategy recent-eval", strategy)
        self.assertIn("skz strategy status", strategy)
        self.assertNotRegex(strategy, r"(?m)^skz experiment ")
        self.assertNotRegex(strategy, r"(?m)^skz promote ")

        self.assertIn("有实验没评审的 → 去 `skz-candidate`", guide)
        self.assertIn("入库后再交给 `skz-strategy` 暂停观察", guide)

    def test_wallet_and_paid_workflow_boundaries(self) -> None:
        wallet = (AUTHORING / "books/skz-wallet/SKILL.md").read_text(encoding="utf-8")
        guide = (AUTHORING / "books/skz-guide/SKILL.md").read_text(encoding="utf-8")
        problem = (AUTHORING / "books/skz-create-problem/SKILL.md").read_text(encoding="utf-8")
        candidate = (AUTHORING / "books/skz-candidate/SKILL.md").read_text(encoding="utf-8")
        strategy = (AUTHORING / "books/skz-strategy/SKILL.md").read_text(encoding="utf-8")
        billing = (AUTHORING / "common/references/billing.md").read_text(encoding="utf-8")

        self.assertIn("skz wallet balance", wallet)
        self.assertIn("skz wallet costs", wallet)
        self.assertIn("skz wallet check", wallet)
        self.assertIn("不触发因子挖掘、策略探索、实盘更新或保存入库", wallet)
        self.assertIn("skz wallet check mine --qty 1", guide)
        self.assertIn("skz wallet check explore --qty 1", guide)
        self.assertIn("skz wallet check refresh --qty <去重数量>", strategy)
        self.assertIn("创建命令（写 · 不花钱 · ⚠️ 必须先问人）", problem)
        self.assertIn("保存入库（写 · 不花钱 · 必须先问人）", candidate)
        self.assertIn("直接登记 `register`（写 · 不花钱 · 必须先问人）", strategy)
        self.assertNotIn("wallet check save", candidate)
        self.assertNotIn("wallet check save", strategy)
        self.assertNotIn("| `save` |", billing)
        self.assertIn('pricingSource:"cli"', billing)

    def test_portfolio_agent_build_documents_frontend_flow(self) -> None:
        portfolio = (AUTHORING / "books/skz-portfolio/SKILL.md").read_text(encoding="utf-8")
        guide = (AUTHORING / "books/skz-portfolio/references/agent-build.md").read_text(encoding="utf-8")
        billing = (AUTHORING / "common/references/billing.md").read_text(encoding="utf-8")
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        self.assertIn("agent-build.md", portfolio)
        for term in ("portfolio build catalog", "portfolio build start", "portfolio build latest",
                     "portfolio build get", "portfolio build history", "needs_review", "requirements_unmet",
                     "configId", "selectionConstraints", "requiredMethod", "rebalanceRule", "goal"):
            self.assertIn(term, guide)
        for term in ("portfolio-build-byok", "portfolio-build", "pricingSource", "portfolio build start"):
            self.assertIn(term, billing)
        self.assertIn("`portfolio build start`", contract)
        self.assertIn("skz portfolio build latest", guide)
        self.assertIn("不重发", guide)

    def test_portfolio_missing_performance_does_not_authorize_refresh(self) -> None:
        portfolio = (AUTHORING / "books/skz-portfolio/SKILL.md").read_text(encoding="utf-8")
        missing = next(line for line in portfolio.splitlines() if line.startswith("- `has_performance: false`"))
        self.assertIn("只报告产物缺失，不自动刷新", missing)
        self.assertIn("用户只要求查看组合不代表同意后台计算", missing)
        self.assertNotIn("再执行", missing)
        refresh = portfolio.split("## 2)", 1)[1].split("## 3)", 1)[0]
        gate = refresh.index("取得绑定本次参数及费用的明确许可")
        self.assertLess(gate, refresh.index("skz portfolio refresh <code>"))
        self.assertIn("当前无组合报价能力时必须暂停刷新", refresh)
        self.assertIn("不能自动重试", refresh)

    def test_portfolio_unknown_fees_fail_closed_without_strategy_quote(self) -> None:
        billing = (AUTHORING / "common/references/billing.md").read_text(encoding="utf-8")
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        portfolio = (AUTHORING / "books/skz-portfolio/SKILL.md").read_text(encoding="utf-8")
        self.assertIn("`portfolio create` / `portfolio refresh`", contract)
        self.assertIn("也不能以跳过余额检查绕过费用核验", contract)
        self.assertIn("不核验组合费用，也不代表授权", contract)
        for scenario in ("没有组合专属报价能力", "报价失败", "费用无法核实"):
            with self.subTest(scenario=scenario):
                self.assertIn(scenario, billing)
        self.assertIn("暂停组合创建与刷新，只保留只读查询", billing)
        self.assertIn("都不能替代费用核验", billing)
        self.assertIn("本仓库尚未定义人工核价流程", billing)
        self.assertIn("服务端当前是否收费、计价单位和实际金额尚未核实", billing)
        self.assertIn("`wallet check refresh` 只对应策略更新，禁止用于组合", billing)
        self.assertIn("参数或费用变化必须重新确认", billing)
        self.assertIn("当前 CLI 没有组合报价项", portfolio)
        self.assertIn("必须暂停创建", portfolio)
        self.assertIn("以下第 2—4 步仅描述未来费用可核实且用户已明确授权后的流程", portfolio)
        for document in (portfolio, billing):
            for command_block in re.findall(r"```bash\n(.*?)```", document, re.S):
                self.assertNotRegex(command_block, r"wallet\s+check\s+(?:refresh|portfolio)")

    def test_generic_gifts_route_to_asset_skills_and_keep_asset_boundaries(self) -> None:
        problem = (AUTHORING / "books/skz-create-problem/SKILL.md").read_text(encoding="utf-8")
        factor = (AUTHORING / "books/skz-factor/SKILL.md").read_text(encoding="utf-8")
        strategy = (AUTHORING / "books/skz-strategy/SKILL.md").read_text(encoding="utf-8")

        problem_frontmatter = problem.split("---", 2)[1]
        factor_frontmatter = factor.split("---", 2)[1]
        self.assertIn("赠予/领取研究问题", problem_frontmatter)
        self.assertIn("因子路线赠予/领取", factor_frontmatter)

        self.assertIn("--asset-type problem", problem)
        self.assertIn("skz problem get <target_code>", problem)
        self.assertIn("没有 strategy status、memo 或 tags 语义", strategy)

        self.assertIn("--asset-type factor-route", factor)
        self.assertIn("skz factor list --route <target_code>", factor)
        self.assertIn("没有路线 memo/status", factor)

        for skill in (problem, factor, strategy):
            self.assertIn("claim_status", skill)
            self.assertIn("resumable", skill)
            self.assertIn("target_code", skill)

        strategy_gift = strategy.split("## 4) 投研资产赠予", 1)[1]
        self.assertIn("| `problem` |", strategy_gift)
        self.assertIn("| `factor_route` |", strategy_gift)
        self.assertIn("| `strategy` |", strategy_gift)
        self.assertIn("只有 strategy", strategy_gift)

    def test_route_and_problem_creation_require_user_review(self) -> None:
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        boundary = contract.split("## 结构化 I/O", 1)[0]
        autonomous = boundary.split("以下操作可以自主执行：", 1)[1]

        for command in ("route create", "factor-routes create", "problem create"):
            self.assertRegex(boundary, rf"\| [^\n]*`{re.escape(command)}`[^\n]* \|.*完整展示.*明确许可")
            self.assertNotIn(f"`{command}`", autonomous)

        guide = (AUTHORING / "books/skz-guide/SKILL.md").read_text(encoding="utf-8")
        create_route = (AUTHORING / "books/skz-create-route/SKILL.md").read_text(encoding="utf-8")
        create_problem = (AUTHORING / "books/skz-create-problem/SKILL.md").read_text(encoding="utf-8")
        sections = (
            (guide.split("### ②", 1)[1].split("### ③", 1)[0], "skz route create", ("每条路线", "七个字段")),
            (create_route.split("## 创建命令", 1)[1], "skz factor-routes create", ("每条路线", "七个字段")),
            (create_route.split("## 创建命令", 1)[1], "skz route create", ("每条路线", "七个字段")),
            (guide.split("### ④", 1)[1].split("### ⑤", 1)[0], "skz problem create", ("标的集合", "市场类型", "频率", "训练/验证时间分段")),
            (create_problem.split("## 创建命令", 1)[1], "skz problem create", ("标的集合", "市场类型", "频率", "训练/验证时间分段")),
        )
        for section, command, fields in sections:
            command_index = section.index(command, section.index("```bash"))
            fence_index = section.rfind("```bash", 0, command_index)
            self.assertNotEqual(fence_index, -1)
            before_command = section[:fence_index]
            self.assertIn("⚠️ 必须先问人", section)
            self.assertIn("完整展示", before_command)
            self.assertTrue(all(field in before_command for field in fields))
            self.assertIn("取得明确许可后才", before_command)
            self.assertIn("内容有修改后", before_command)
            self.assertIn("重新展示", before_command)
            self.assertIn("重新确认", before_command)
            self.assertIn("直接建", before_command)
            self.assertIn("不得跳过", before_command)

    def test_route_market_mechanism_enum_guardrail(self) -> None:
        guide = (AUTHORING / "books/skz-guide/SKILL.md").read_text(encoding="utf-8")
        create_route = (AUTHORING / "books/skz-create-route/SKILL.md").read_text(encoding="utf-8")
        section = create_route.split("## `market_mechanism`", 1)[1].split("\n## ", 1)[0]
        guide_section = guide.split("### ②", 1)[1].split("### ③", 1)[0]
        self.assertNotIn("套保压力", guide_section)
        self.assertNotIn("| 对人说 |", guide_section)

        # 完整、按既定顺序的 10 值清单（行尾锚定：新增第 11 个值或乱序都会失配）
        self.assertRegex(
            section,
            r"(?m)`错误定价` · `风险补偿` · `行为偏差` · `流动性溢价` · `制度性套利` · `自我实现预言` · `微观结构` · `信息扩散` · `趋势跟踪` · `套保压力`\s*$",
        )
        # 核心规则必须逐字完整保留，不能仅保留「逐字自检」等无约束措辞。
        self.assertIn(
            "**`market_mechanism` 必须逐字等于这 10 个值之一**，禁组合、禁加后缀、禁自创、禁近义改写：",
            section,
        )
        # 指定归并映射：口语化机制术语必须归并为枚举值
        self.assertRegex(section, r"投资者反应不足[^\n]*行为偏差")
        for rule in ("没有 meta 发现端点", "后端加了值它不会自己更新", "不扣费",
                     "skz factor-routes list", "以真实返回为准"):
            self.assertIn(rule, section)

    def test_route_entrypoints_share_one_ledger_and_idempotency_is_bounded(self) -> None:
        create_route = (AUTHORING / "books/skz-create-route/SKILL.md").read_text(encoding="utf-8")
        guide = (AUTHORING / "books/skz-guide/SKILL.md").read_text(encoding="utf-8")
        guide_section = guide.split("### ②", 1)[1].split("### ③", 1)[0]
        for fact in ("skz factor-routes create", "skz route create", "POST /research/factor-routes",
                     "POST /strategy/routes", "同一份路线账本", "单账号实测", "11/11", "差集为空",
                     "参与内容哈希的 5 个字段", "改了其中任何一个就是新路线", "不要重发",
                     "拿不准就把正文原样报给人", "比机制，不比名字", "route_precheck.py",
                     "approved:false", "approval_required:true", "绝不代替用户批准"):
            self.assertIn(fact, create_route)
        self.assertIn("skz-create-route", guide_section)

    def test_scripts_are_executable_and_validate_offline_plan(self) -> None:
        for path in SCRIPTS.glob("*.py"):
            self.assertTrue(path.stat().st_mode & stat.S_IXUSR, f"{path} is not executable")
        valid = run_script(
            "validate_plan.py",
            "--offline",
            stdin={
                "operation": "explore.start",
                "route": "R1",
                "problem": "P1",
                "assumption": "momentum persists",
                "failure_signal": "out-of-sample collapse",
            },
        )
        self.assertEqual(valid.returncode, 0, valid.stderr)
        body = json.loads(valid.stdout)
        self.assertTrue(body["valid"])
        self.assertFalse(body["approved"])
        self.assertTrue(body["approval_required"])

        invalid = run_script("validate_plan.py", "--offline", stdin={"operation": "mine.start"})
        self.assertEqual(invalid.returncode, 2)
        self.assertIn("missing or invalid route", json.loads(invalid.stdout)["errors"])

    def test_portfolio_plan_schema_boundaries(self) -> None:
        valid = {
            "operation": "portfolio.create", "portfolio_code": "PF_NEW",
            "candidate_strategies": ["S201"], "base_market": "stock",
            "rebalance_dates": ["2024-02-29", "2025-01-01"],
            "assumption": "a", "failure_signal": "b",
        }
        result = run_script("validate_plan.py", "--offline", stdin=valid)
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)
        self.assertTrue(output["valid"])
        self.assertEqual(output["validation_scope"], "schema_only")
        self.assertFalse(output["approved"])
        self.assertFalse(output["ready_to_submit"])
        self.assertFalse(output["billing_verified"])
        self.assertEqual(output["live_checks"], [])
        for key in ("portfolio_code", "candidate_strategies", "base_market", "rebalance_dates"):
            with self.subTest(missing=key):
                plan = {field: value for field, value in valid.items() if field != key}
                result = run_script("validate_plan.py", "--offline", stdin=plan)
                self.assertEqual(result.returncode, 2)
                self.assertIn(f"missing or invalid {key}", json.loads(result.stdout)["errors"])
        invalid_values = {
            "portfolio_code": [None, 1, [], "", " PF_NEW"],
            "candidate_strategies": [None, "S201", [], [1], [""], [" "], [" S201"]],
            "base_market": [None, [], 1, "", " "],
            "rebalance_dates": [None, "2025-01-01", [], [1], [""], [" "],
                                ["2025-02-29"], ["2025-13-01"], ["2025-04-31"],
                                ["20250101"], ["2025-1-1"], ["2025-01-01T00:00:00"],
                                ["2025-01-01", "invalid"]],
            "description": [None, 1], "base_freq": [[]],
            "price_field": [False], "rebalance_method": [{}], "operation": [[], {}],
        }
        for key, values in invalid_values.items():
            for value in values:
                with self.subTest(key=key, value=value):
                    result = run_script("validate_plan.py", "--offline", stdin={**valid, key: value})
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertFalse(json.loads(result.stdout)["valid"])

    def test_portfolio_plan_reads_complete_inventory_and_reports_unknown(self) -> None:
        with patch.object(sys, "path", [str(SCRIPTS), *sys.path]):
            spec = importlib.util.spec_from_file_location("validate_plan", SCRIPTS / "validate_plan.py")
            validator = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(validator)
        plan = {"operation": "portfolio.create", "portfolio_code": "PF_NEW",
                "candidate_strategies": ["S201"], "base_market": "stock",
                "rebalance_dates": ["2025-01-01"], "assumption": "a", "failure_signal": "b"}
        first = {"page": 1, "page_size": 200, "total": 201,
                 "items": [{"code": f"S{number}", "status": "实盘"} for number in range(1, 201)]}
        second = {"page": 2, "page_size": 200, "total": 201,
                  "items": [{"code": "S201", "status": "实盘"}]}
        cases = [
            ("page_201", first, second, True, "passed"),
            ("missing", first, {**second, "items": [{"code": "OTHER", "status": "实盘"}]}, False, "failed"),
            ("page_failed", first, None, False, "unknown"),
            ("total_changed", first, {**second, "total": 202}, False, "unknown"),
            ("short_page", {**first, "items": first["items"][:1]}, second, False, "unknown"),
            ("empty_page", first, {**second, "items": []}, False, "unknown"),
            ("duplicate_page", first, {**second, "items": first["items"]}, False, "unknown"),
            ("wrong_status", first, {**second, "items": [{"code": "S201", "status": "暂停"}]}, False, "unknown"),
            ("name_not_code", first, {**second, "items": [{"name": "S201", "status": "实盘"}]}, False, "unknown"),
            ("nested_code", first, {**second, "items": [{"code": "OTHER", "status": "实盘", "nested": {"code": "S201"}}]}, False, "failed"),
            ("duplicate_code", first, {**second, "items": [{"code": "S1", "status": "实盘"}]}, False, "unknown"),
            ("malformed_json", "not json", second, False, "unknown"),
            ("missing_items", {"page": 1, "page_size": 200, "total": 0}, second, False, "unknown"),
            ("non_object_row", {**first, "items": [None]}, second, False, "unknown"),
            ("too_many_items", {**first, "items": first["items"] + second["items"]}, second, False, "unknown"),
            ("total_exceeded", {**first, "total": 1}, second, False, "unknown"),
        ]
        for field, values in {"total": [None, -1, True, "201"], "page": [None, True, 2],
                              "page_size": [None, True, 100]}.items():
            for value in values:
                cases.append((f"invalid_{field}_{value}", {**first, field: value}, second, False, "unknown"))
        for name, page_one, page_two, valid, state in cases:
            calls = []

            def fake_skz(*args):
                calls.append(args)
                if args == ("auth", "status"):
                    return {"ok": True, "data": {"present": True, "readOnly": False}}
                if args == ("portfolio", "list"):
                    return {"ok": True, "data": {"items": []}}
                if args in [("strategy", "list", "--status", "实盘", "--page", str(page), "--page-size", "200")
                            for page in (1, 2)]:
                    data = page_one if args[5] == "1" else page_two
                    return {"ok": data is not None, "data": data}
                self.fail(f"unexpected or non-read command: {args}")

            with self.subTest(name=name), patch.object(validator, "run_skz", side_effect=fake_skz), \
                    patch.object(validator, "load_json", return_value=plan), \
                    patch.object(sys, "argv", ["validate_plan.py"]):
                output = io.StringIO()
                with patch.object(validator, "emit", side_effect=lambda value: json.dump(value, output)):
                    exit_code = validator.main()
                result = json.loads(output.getvalue())
                self.assertEqual(exit_code, 0 if valid else 2)
                self.assertEqual(result["valid"], valid)
                self.assertEqual(result["validation_scope"], "read_only_preflight")
                self.assertFalse(result["approved"])
                self.assertTrue(result["approval_required"])
                self.assertFalse(result["ready_to_submit"])
                self.assertFalse(result["billing_verified"])
                check = next(check for check in result["live_checks"] if check["name"] == "candidates_live")
                self.assertEqual(check["state"], state)
                if state == "unknown":
                    self.assertIsNone(check["ok"])
                    self.assertIn("reason", check["result"])
                if name == "page_201":
                    self.assertEqual([args[5] for args in calls if args[:2] == ("strategy", "list")], ["1", "2"])

    def test_portfolio_plan_strict_remote_identity_and_code_checks(self) -> None:
        with patch.object(sys, "path", [str(SCRIPTS), *sys.path]):
            spec = importlib.util.spec_from_file_location("validate_plan", SCRIPTS / "validate_plan.py")
            validator = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(validator)
        plan = {"operation": "portfolio.create", "portfolio_code": "PF_NEW",
                "candidate_strategies": ["S1"], "base_market": "stock",
                "rebalance_dates": ["2025-01-01"], "assumption": "a", "failure_signal": "b"}
        auth = {"ok": True, "data": {"present": True, "readOnly": False}}
        strategies = {"ok": True, "data": {"page": 1, "page_size": 200, "total": 1,
                                          "items": [{"code": "S1", "status": "实盘"}]}}
        portfolios = {"ok": True, "data": {"items": []}}
        cases = [("valid", auth, portfolios, True),
                 ("collision", auth, {"ok": True, "data": {"items": [{"code": "PF_NEW"}]}}, False),
                 ("exact_code_only", auth, {"ok": True, "data": {"items": [{"code": "OTHER", "name": "PF_NEW"}]}}, True),
                 ("auth_failed", {"ok": False}, portfolios, False),
                 ("readonly_missing", {"ok": True, "data": {"present": True}}, portfolios, False),
                 ("readonly_string", {"ok": True, "data": {"present": True, "readOnly": "false"}}, portfolios, False),
                 ("present_string", {"ok": True, "data": {"present": "true", "readOnly": False}}, portfolios, False)]
        for data in (None, {}, {"items": None}, {"items": [None]}, {"items": [{"name": "PF_NEW"}]},
                     {"items": [{"code": " PF_NEW"}]}):
            cases.append((f"invalid_portfolios_{data}", auth, {"ok": True, "data": data}, False))
        cases.append(("portfolio_read_failed", auth, {"ok": False}, False))
        for name, identity, inventory, valid in cases:
            with self.subTest(name=name), \
                    patch.object(validator, "run_skz", side_effect=[identity, strategies, inventory]) as reads, \
                    patch.object(validator, "load_json", return_value=plan), \
                    patch.object(validator, "emit") as emit, \
                    patch.object(sys, "argv", ["validate_plan.py"]):
                self.assertEqual(validator.main(), 0 if valid else 2)
                result = emit.call_args.args[0]
                self.assertEqual(result["valid"], valid)
                self.assertFalse(result["approved"])
                self.assertEqual([call.args[:2] for call in reads.call_args_list],
                                 [("auth", "status"), ("strategy", "list"), ("portfolio", "list")])
                if name.startswith("invalid_portfolios") or name == "portfolio_read_failed":
                    check = result["live_checks"][-1]
                    self.assertIsNone(check["ok"])
                    self.assertEqual(check["state"], "unknown")

    def test_scripts_only_issue_read_commands(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            log = tmp_path / "commands.jsonl"
            fake = tmp_path / "skz"
            fake.write_text(
                """#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
with open(os.environ['SKZ_TEST_LOG'], 'a', encoding='utf-8') as stream:
    stream.write(json.dumps(args, ensure_ascii=False) + '\\n')
if args == ['--version']:
    data = {'cli': 'test', 'contract': '4.0'}
elif args == ['auth', 'status']:
    data = {'present': True, 'active': 'test', 'account': 'test', 'writePolicy': 'allow', 'readOnly': False}
elif args == ['whoami']:
    data = {'user_id': 'u1'}
elif args[:2] == ['mine', 'runs']:
    data = {'page': 1, 'size': 100, 'total': 0, 'items': []}
elif args[:2] == ['explore', 'runs']:
    data = {'page': 1, 'size': 100, 'total': 1, 'items': [{'fcRunId': 'E1', 'status': 'running', 'done': False, 'ok': False}]}
elif args == ['mining', 'runs']:
    data = {'total': 1, 'items': [{'run_id': 'OTHER_RUN'}]}
elif args == ['portfolio', 'list']:
    data = {'items': [{'code': 'PF1', 'has_performance': False}]}
elif args == ['factor-routes', 'list']:
    data = {'total': 1, 'items': [{'code': 'R1', 'name': 'route'}]}
elif args == ['problem', 'get', 'P1']:
    data = {'code': 'P1'}
elif args == ['strategy', 'get', 'S1']:
    data = {'code': 'S1', 'status': '暂停'}
else:
    print(json.dumps({'error': {'action': 'fix_params', 'args': args}}), file=sys.stderr)
    raise SystemExit(2)
print(json.dumps(data, ensure_ascii=False))
""",
                encoding="utf-8",
            )
            fake.chmod(0o755)
            env = {**os.environ, "SKZ_BIN": str(fake), "SKZ_TEST_LOG": str(log)}

            preflight = run_script("preflight.py", "--operation", "paid", env=env)
            self.assertEqual(preflight.returncode, 0, preflight.stderr)
            self.assertTrue(json.loads(preflight.stdout)["ready"])

            resume = run_script("resume.py", env=env)
            self.assertEqual(resume.returncode, 0, resume.stderr)
            resumed = json.loads(resume.stdout)
            self.assertEqual(resumed["next_action"], "poll_exploration")
            self.assertEqual(
                resumed["portfolios_missing_performance"][0]["code"], "PF1"
            )

            plan = run_script(
                "validate_plan.py",
                env=env,
                stdin={
                    "operation": "explore.start",
                    "route": "R1",
                    "problem": "P1",
                    "assumption": "a",
                    "failure_signal": "b",
                },
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)
            self.assertTrue(json.loads(plan.stdout)["valid"])

            verify = run_script("verify_write.py", "strategy.write", "--code", "S1",
                                "--expected", '{"status":"暂停"}', env=env)
            self.assertEqual(verify.returncode, 0, verify.stderr)
            self.assertTrue(json.loads(verify.stdout)["confirmed"])

            verify_delete = run_script(
                "verify_write.py", "mining.delete-run", "--code", "RUN_1", env=env
            )
            self.assertEqual(verify_delete.returncode, 0, verify_delete.stderr)
            self.assertTrue(json.loads(verify_delete.stdout)["confirmed"])

            commands = [json.loads(line) for line in log.read_text(encoding="utf-8").splitlines()]
            prohibited = re.compile(r"(^|\.)(create|delete|start|register|claim)$")
            for command in commands:
                dotted = ".".join(arg for arg in command if not arg.startswith("--"))
                self.assertIsNone(prohibited.search(dotted), f"write command issued: {command}")

    def test_write_continuation_has_one_authority_and_no_automatic_exceptions(self) -> None:
        contract = (AUTHORING / "common/references/operating-contract.md").read_text(encoding="utf-8")
        rule = contract.split("### 写操作重试与续作唯一规则", 1)[1].split("常见读回路径", 1)[0]
        for requirement in ("确定业务拒绝", "确定部分完成", "传输结果未知", "自动追加写调用次数均为 **0**",
                            "一次性明确续作授权", "最多追加 **1 次**", "不循环", "不因重新 preview 而重置次数",
                            "核验失败", "授权缺失", "影响变化", "404", "原写永不落地",
                            "不是 `--dry-run`", "failed_mining_runs", "mining runs --route",
                            "pending + resumable=true", "done", "另一请求", "组合报价未知仍须暂停"):
            with self.subTest(requirement=requirement):
                self.assertIn(requirement, rule)
        for relative in ("skz-factor/SKILL.md", "skz-strategy/SKILL.md", "skz-create-problem/SKILL.md",
                         "skz-openapi/references/research/delete-factor-routes-code.md"):
            document = (AUTHORING / "books" / relative).read_text(encoding="utf-8")
            with self.subTest(document=relative):
                self.assertIn("operating-contract.md#写操作重试与续作唯一规则", document)
                self.assertIn("一次性明确续作授权", document)
                self.assertIn("最多追加 1 次", document)
                self.assertIn("随后读回并停止", document)
                self.assertIn("未知结果、核验失败或缺少授权时追加写调用为 0", document)
                for unsafe in ("重发同一条命令续删**即可", "退避几秒重发同一条命令即可",
                               "本次没落库也没占名额", "重试同一请求可续删", "可再次 claim 续作"):
                    self.assertNotIn(unsafe, document)

    def test_write_verification_preserves_unknown_and_never_authorizes_replay(self) -> None:
        cases = []
        for operation, args in (("route.create", ["--name", "route"]), ("problem.create", ["--code", "P1"])):
            for code in (2, 3, 5, 6):
                cases.append((operation, args, code, {}, None, "inconclusive"))
        for code, status, api_code, confirmed in ((2, 404, 40400, True), (2, 404, "40400", True),
                                                   (2, 400, 40000, None), (3, 401, 40100, None),
                                                   (5, 503, None, None), (2, 404, None, None)):
            cases.append(("problem.delete", ["--code", "P1"], code,
                          {"error": {"status": status, "code": api_code}}, confirmed,
                          "absent" if confirmed else "inconclusive"))
        cases.extend([
            ("route.create", ["--name", "route"], 0, {"total": 0, "items": []}, False, "absent"),
            ("problem.create", ["--code", "P1"], 2,
             {"error": {"status": 404, "code": "40400"}}, False, "absent"),
            ("problem.create", ["--code", "P1"], 0, {"code": "P1"}, True, "present"),
            ("mine.start", ["--code", "RUN_1"], 0, {"page": 1, "size": 100, "total": 0, "items": []}, None, "inconclusive"),
            ("portfolio.create", ["--code", "PF1"], 0, {"total": 0, "items": []}, False, "portfolio_not_found"),
            ("mining.delete-run", ["--code", "RUN_1"], 0,
             {"total": 1, "items": [{"run_id": "RUN_1"}]}, False, "run_present"),
            ("mining.delete-run", ["--code", "RUN_1"], 0,
             {"total": 1, "items": [{"run_id": "OTHER", "route_name": "RUN_1"}]}, True, "run_absent"),
            ("mining.delete-run", ["--code", "RUN_1"], 0,
             {"total": 0, "items": []}, True, "run_absent"),
            ("mining.delete-run", ["--code", "RUN_1"], 5, {}, None, "inconclusive"),
        ])
        for malformed in ({}, {"items": []}, {"total": 2, "items": [{"run_id": "OTHER"}]},
                          {"total": 1, "items": [{"code": "RUN_1"}]}, {"total": 0, "items": None}):
            cases.append(("mining.delete-run", ["--code", "RUN_1"], 0, malformed, None, "inconclusive"))
        with tempfile.TemporaryDirectory() as tmp:
            fake = Path(tmp) / "skz"
            fake.write_text(
                "#!/usr/bin/env python3\nimport json, os, sys\n"
                "with open(os.environ['VERIFY_CALLS'], 'a') as output:\n"
                "    output.write(json.dumps(sys.argv[1:]) + '\\n')\n"
                "code = int(os.environ['VERIFY_EXIT'])\n"
                "print(os.environ['VERIFY_DATA'], file=sys.stderr if code else sys.stdout)\n"
                "sys.exit(code)\n"
            )
            fake.chmod(0o755)
            for operation, args, code, data, confirmed, meaning in cases:
                with self.subTest(operation=operation, code=code, data=data):
                    calls = Path(tmp) / "calls.jsonl"
                    calls.write_text("", encoding="utf-8")
                    result = run_script("verify_write.py", operation, *args, env={
                        **os.environ, "SKZ_BIN": str(fake), "VERIFY_EXIT": str(code),
                        "VERIFY_DATA": json.dumps(data), "VERIFY_CALLS": str(calls),
                    })
                    self.assertEqual(result.returncode, 2 if confirmed is None else 0, result.stderr)
                    body = json.loads(result.stdout)
                    self.assertIs(body["confirmed"], confirmed)
                    self.assertEqual(body["meaning"], meaning)
                    self.assertFalse(body["safe_to_retry"])
                    commands = [json.loads(line) for line in calls.read_text(encoding="utf-8").splitlines()]
                    expected = {
                        "route.create": ["factor-routes", "list"],
                        "problem.create": ["problem", "get", "P1"],
                        "problem.delete": ["problem", "get", "P1"],
                        "mine.start": ["mine", "runs", "--page", "1", "--size", "100"],
                        "portfolio.create": ["portfolio", "list"],
                        "mining.delete-run": ["mining", "runs"],
                    }
                    self.assertEqual(commands, [expected[operation]])

    @unittest.skipUnless(os.name == "posix", "Unix umask and mode contract")
    def test_rendering_is_independent_of_umask(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            snapshots = []
            for mask in ("002", "077"):
                root = Path(tmp) / mask
                result = subprocess.run([
                    "python3", "-c",
                    "import os, sys; from pathlib import Path; "
                    "from build_plugins import sync_sources; "
                    "os.umask(int(sys.argv[2], 8)); sync_sources(Path(sys.argv[1]))",
                    str(root), mask,
                ], cwd=ROOT / "scripts/release", capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                files = {str(p.relative_to(root)): (p.read_bytes(), stat.S_IMODE(p.stat().st_mode))
                         for p in root.rglob("*") if p.is_file()}
                self.assertTrue(all(mode in (0o644, 0o755) for _, mode in files.values()))
                self.assertEqual(files["manifest.json"][1], 0o644)
                snapshots.append(files)
            self.assertEqual(*snapshots)

    def test_generated_plugins_are_synced(self) -> None:
        result = subprocess.run(
            ["python3", str(ROOT / "scripts" / "release" / "build_plugins.py"), "--check"],
            capture_output=True,
            text=True,
            cwd=ROOT,
            check=False,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"plugins/ is stale or hand-edited:\n{result.stdout}{result.stderr}",
        )


ROUTE_PRECHECK_SCRIPT = ROOT / "plugin-src/books/skz-create-route/scripts/route_precheck.py"
ROUTE_PRECHECK_COMMON = ROOT / "plugin-src/common/scripts/_skz_common.py"
ROUTE_PRECHECK_SAMPLE = {
    "name": "放量突破",
    "key_inspect": "突破前高且成交量上升后，观察收益是否延续",
    "economic_logic": "新信息被市场逐步接受，增量买盘持续进入并推高价格",
    "why_effective": "机构分批建仓和交易成本限制使套利无法立即抹平价格差异",
    "market_mechanism": "趋势跟踪",
    "failure_scenarios": ["震荡市场中放量突破后快速回落"],
    "tags": [],
}


class RoutePrecheckTests(unittest.TestCase):
    def run_check(self, route=ROUTE_PRECHECK_SAMPLE, *, raw=None, args=(), script=ROUTE_PRECHECK_SCRIPT, cwd=None):
        result = subprocess.run(
            [sys.executable, str(script), *args],
            input=json.dumps(route, ensure_ascii=False) if raw is None else raw,
            capture_output=True, text=True, check=False, cwd=cwd,
        )
        self.assertEqual(result.stderr, "")
        body = json.loads(result.stdout)
        self.assertFalse(body["approved"])
        self.assertTrue(body["approval_required"])
        self.assertIn("offline check only", body["note"])
        self.assertEqual(result.returncode, 0 if body["valid"] else 2)
        self.assertEqual("normalized" in body, body["valid"])
        return body

    def test_route_precheck_valid_body_preserves_hash_fields(self):
        self.assertTrue(ROUTE_PRECHECK_SCRIPT.stat().st_mode & stat.S_IXUSR)
        route = {**ROUTE_PRECHECK_SAMPLE, "name": "  放量突破  ", "compute_engine": "TimeSeriesAstEngine"}
        body = self.run_check(route)
        self.assertTrue(body["valid"])
        self.assertEqual(body["normalized"], route)
        self.assertEqual(body["warnings"], [])
        for mechanism in ("错误定价", "风险补偿", "行为偏差", "流动性溢价", "制度性套利",
                          "自我实现预言", "微观结构", "信息扩散", "趋势跟踪", "套保压力"):
            with self.subTest(mechanism=mechanism):
                self.assertTrue(self.run_check({**ROUTE_PRECHECK_SAMPLE, "market_mechanism": mechanism})["valid"])

    def test_route_precheck_rejects_missing_and_wrong_fields(self):
        for field in ROUTE_PRECHECK_SAMPLE:
            with self.subTest(missing=field):
                body = self.run_check({key: value for key, value in ROUTE_PRECHECK_SAMPLE.items() if key != field})
                self.assertFalse(body["valid"])
                self.assertTrue(any(field in error for error in body["errors"]))
        invalid = {
            **{field: [None, [], False, 1, "", "  "]
               for field in ("name", "key_inspect", "economic_logic", "why_effective", "compute_engine")},
            "market_mechanism": [None, [], False, "", "趋势跟踪+风险补偿", "趋势跟踪 "],
            "failure_scenarios": [None, "回撤", [], [""], ["  "], [1], [None]],
            "tags": [None, "趋势", [1], [None]],
        }
        for field, values in invalid.items():
            for value in values:
                with self.subTest(field=field, value=value):
                    body = self.run_check({**ROUTE_PRECHECK_SAMPLE, field: value})
                    self.assertFalse(body["valid"])
                    self.assertTrue(any(field in error for error in body["errors"]))
        self.assertTrue(self.run_check({**ROUTE_PRECHECK_SAMPLE, "tags": ["趋势", ""]})["valid"])

    def test_route_precheck_suggests_but_never_substitutes_mechanisms(self):
        for mechanism, suggestion in (("投资者反应不足", "行为偏差"), ("散户追涨", "行为偏差"),
                                      ("机构调仓被迫接盘", "流动性溢价"), ("套保盘必须出场", "套保压力")):
            with self.subTest(mechanism=mechanism):
                body = self.run_check({**ROUTE_PRECHECK_SAMPLE, "market_mechanism": mechanism})
                self.assertFalse(body["valid"])
                self.assertTrue(any(suggestion in warning and "由人判定" in warning
                                    for warning in body["warnings"]))

    def test_route_precheck_quality_and_unknown_fields_only_warn(self):
        route = {**ROUTE_PRECHECK_SAMPLE, "name": "名" * 31, "economic_logic": "理" * 14,
                 "why_effective": "效" * 14, "failure_scenarios": ["坏" * 5], "extra": {"keep": True}}
        body = self.run_check(route)
        self.assertTrue(body["valid"])
        self.assertEqual(len(body["warnings"]), 5)
        for field in ("name", "economic_logic", "why_effective", "failure_scenarios[0]", "extra"):
            self.assertTrue(any(field in warning for warning in body["warnings"]))
        self.assertEqual(body["normalized"], route)
        body = self.run_check({**ROUTE_PRECHECK_SAMPLE, "name": "名" * 30, "economic_logic": "理" * 15,
                               "why_effective": "效" * 15, "failure_scenarios": ["坏" * 6]})
        self.assertEqual(body["warnings"], [])

    def test_route_precheck_bad_json_file_input_and_deployed_layout(self):
        for raw in ("{", "", "null", "[]", "123", '"text"'):
            with self.subTest(raw=raw):
                self.assertFalse(self.run_check(raw=raw)["valid"])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            route_file = path / "route.json"
            route_file.write_text(json.dumps(ROUTE_PRECHECK_SAMPLE, ensure_ascii=False), encoding="utf-8")
            self.assertTrue(self.run_check(raw="ignored", args=("--file", str(route_file)))["valid"])
            self.assertFalse(self.run_check(args=("--file", str(path / "missing.json")))["valid"])
            deployed = path / "route_precheck.py"
            shutil.copy2(ROUTE_PRECHECK_SCRIPT, deployed)
            shutil.copy2(ROUTE_PRECHECK_COMMON, path / "_skz_common.py")
            before = sorted(item.name for item in path.iterdir())
            self.assertTrue(self.run_check(script=deployed, cwd=path)["valid"])
            self.assertEqual(sorted(item.name for item in path.iterdir()), before)

    def test_route_precheck_never_calls_network_or_skz(self):
        with patch.object(sys, "dont_write_bytecode", True):
            spec = importlib.util.spec_from_file_location("route_precheck", ROUTE_PRECHECK_SCRIPT)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
        with patch("socket.socket", side_effect=AssertionError("network access")), \
                patch("subprocess.run", side_effect=AssertionError("external command")), \
                patch.object(module, "load_json", return_value=ROUTE_PRECHECK_SAMPLE), \
                patch.object(module, "emit") as emit, \
                patch.object(sys, "argv", [str(ROUTE_PRECHECK_SCRIPT)]):
            self.assertEqual(module.main(), 0)
            self.assertTrue(emit.call_args.args[0]["valid"])
            self.assertFalse(emit.call_args.args[0]["approved"])



class RecoveryTests(unittest.TestCase):
    def run_mocked(self, script, args, responses):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fake = root / "skz"
            fake.write_text(
                "#!/usr/bin/env python3\nimport json, os, sys\n"
                "from pathlib import Path\n"
                "root = Path(os.environ['MOCK_ROOT'])\n"
                "with (root / 'calls').open('a') as output:\n"
                "    output.write(json.dumps(sys.argv[1:]) + '\\n')\n"
                "responses = json.loads((root / 'responses').read_text())\n"
                "key = ' '.join(sys.argv[1:])\n"
                "if key not in responses:\n"
                "    print('unexpected command: ' + key, file=sys.stderr)\n"
                "    sys.exit(99)\n"
                "code, data = responses[key]\n"
                "print(json.dumps(data), file=sys.stderr if code else sys.stdout)\n"
                "sys.exit(code)\n", encoding="utf-8",
            )
            fake.chmod(0o755)
            (root / "responses").write_text(json.dumps(responses), encoding="utf-8")
            result = run_script(script, *args, env={**os.environ, "SKZ_BIN": str(fake), "MOCK_ROOT": str(root)})
            calls = [json.loads(line) for line in (root / "calls").read_text().splitlines()]
            self.assertTrue(all(" ".join(call) in responses for call in calls), calls)
            self.assertFalse(any(word in {"start", "create", "delete", "memo", "refresh"}
                                 for call in calls for word in call), calls)
            return result, json.loads(result.stdout), calls

    @staticmethod
    def row(run_id="run-1", status="running", **fields):
        return {"fcRunId": run_id, "routeCode": "route-a", "statusText": status,
                "status": status, "done": status in {"succeeded", "failed"},
                "ok": status == "succeeded", "errorCode": None, "errorMessage": None,
                "resultPath": None, "createdAt": "2026-10-16T00:00:00+08:00", "finishedAt": None, **fields}

    @staticmethod
    def page(rows, total=None, page=1, size=100):
        return {"items": rows, "total": len(rows) if total is None else total, "page": page, "size": size}

    def test_task_verification_requires_receipt_and_same_row_target(self):
        rows = [self.row(), self.row("other", routeCode="unrelated")]
        cases = [([], None, "inconclusive"),
                 (["--code", "route-a"], None, "target_match_only"),
                 (["--code", "problem-a"], None, "inconclusive"),
                 (["--code", "missing"], None, "inconclusive"),
                 (["--run-id", "run-1"], True, "run_match"),
                 (["--run-id", "missing"], None, "inconclusive"),
                 (["--run-id", "run-1", "--code", "route-a"], True, "run_match"),
                 (["--run-id", "run-1", "--code", "problem-a"], True, "run_match"),
                 (["--run-id", "other", "--code", "route-a"], None, "inconclusive"),
                 (["--run-id", "other", "--code", "problem-a"], None, "inconclusive")]
        for command in ("mine", "explore"):
            for args, confirmed, meaning in cases:
                with self.subTest(command=command, args=args):
                    if command == "mine" and args == ["--run-id", "run-1", "--code", "problem-a"]:
                        confirmed, meaning = None, "inconclusive"
                    responses = {f"{command} runs --page 1 --size 100": [0, self.page(rows)]}
                    if command == "explore":
                        responses.update({
                            "explore poll run-1": [0, [{**rows[0], "problemCode": "problem-a"}]],
                            "explore poll other": [0, [{**rows[1], "problemCode": "other-problem"}]],
                        })
                    result, body, calls = self.run_mocked("verify_write.py", [command + ".start", *args], responses)
                    self.assertIs(body["confirmed"], confirmed)
                    self.assertEqual(body["meaning"], meaning)
                    self.assertEqual(result.returncode, 0 if confirmed else 2)
                    self.assertFalse(body["safe_to_retry"])
                    self.assertNotIn("--status", calls[0])

    def test_strategy_verification_compares_expected_fields_and_identity(self):
        data = {"code": "S1", "status": "live", "memo": "笔记", "tags": ["one"]}
        cases = [(None, data, None), ({"status": "live"}, data, True),
                 ({"status": "stopped"}, data, False), ({"memo": "笔记"}, data, True),
                 ({"memo": "旧笔记"}, data, False), ({"memo": ""}, {"code": "S1"}, False),
                 ({"status": "live"}, {**data, "code": "S2"}, None),
                 ({"tags": ["one"]}, data, True), ({"tags": ["two"]}, data, False),
                 ({"status": "live", "memo": "旧笔记"}, data, False),
                 ({"status": True}, {**data, "status": 1}, False)]
        for expected, observed, confirmed in cases:
            with self.subTest(expected=expected, observed=observed):
                args = ["strategy.write", "--code", "S1"]
                if expected is not None:
                    args += ["--expected", json.dumps(expected)]
                command = "strategy get S1"
                if expected:
                    command += "".join(f" --require-{field}" for field in ("memo", "tags") if field in expected)
                result, body, _ = self.run_mocked("verify_write.py", args, {command: [0, observed]})
                self.assertIs(body["confirmed"], confirmed)
                self.assertFalse(body["safe_to_retry"])
                self.assertEqual(result.returncode, 2 if confirmed is None else 0)

    def test_problem_evidence_must_be_readable_and_belong_to_same_run_and_route(self):
        correct = {**self.row(), "problemCode": "problem-a"}
        cases = [[5, {"error": "offline"}], [0, []], [0, [correct, correct]],
                 [0, [{**correct, "fcRunId": "other"}]], [0, [{**correct, "routeCode": "other-route"}]],
                 [0, [self.row()]], [0, [{**correct, "problemCode": "other-problem"}]],
                 [0, [{**correct, "status": "unknown"}]], [0, ["not a row"]]]
        for evidence in cases:
            with self.subTest(evidence=evidence):
                result, body, calls = self.run_mocked("verify_write.py", ["explore.start", "--run-id", "run-1",
                                                                          "--code", "problem-a"], {
                    "explore runs --page 1 --size 100": [0, self.page([self.row()])],
                    "explore poll run-1": evidence,
                })
                self.assertEqual(result.returncode, 2)
                self.assertIsNone(body["confirmed"])
                self.assertFalse(body["safe_to_retry"])
                self.assertEqual(calls[-1], ["explore", "poll", "run-1"])

    def recovery_responses(self, mine=None, explore=None):
        return {"auth status": [0, {"present": True}], "whoami": [0, {"user_id": "mock"}],
                "portfolio list": [0, {"items": []}],
                "mine runs --page 1 --size 100": [0, self.page([]) if mine is None else mine],
                "explore runs --page 1 --size 100": [0, self.page([]) if explore is None else explore]}

    def test_strategy_expected_cannot_only_prove_existence(self):
        for expected in ({}, {"code": "S1"}, {"status": "live", "name": "S1"}, [], "live"):
            with self.subTest(expected=expected):
                result = run_script("verify_write.py", "strategy.write", "--code", "S1",
                                    "--expected", json.dumps(expected), env={**os.environ, "SKZ_BIN": "/not-a-real-skz"})
                self.assertEqual(result.returncode, 2)
                self.assertIn("--expected", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_read_failures_preserve_unknown_without_write(self):
        for command in ("mine", "explore"):
            result, body, _ = self.run_mocked("verify_write.py", [command + ".start", "--run-id", "run-1"], {
                f"{command} runs --page 1 --size 100": [5, {"error": "offline"}],
            })
            self.assertEqual(result.returncode, 2)
            self.assertIsNone(body["confirmed"])
            self.assertFalse(body["safe_to_retry"])
        result, body, _ = self.run_mocked("verify_write.py", ["strategy.write", "--code", "S1",
                                                              "--expected", '{"memo":"笔记"}'], {
            "strategy get S1 --require-memo": [6, {"error": "memo missing"}],
        })
        self.assertEqual(result.returncode, 2)
        self.assertIsNone(body["confirmed"])
        self.assertFalse(body["safe_to_retry"])

    def test_recovery_and_verification_read_beyond_first_hundred(self):
        first = [self.row(f"run-{index}") for index in range(100)]
        last = self.row("last", "dispatching")
        responses = self.recovery_responses(self.page(first, 101), self.page(first, 101))
        for command in ("mine", "explore"):
            responses[f"{command} runs --page 2 --size 100"] = [0, self.page([last], 101, page=2)]
        result, body, calls = self.run_mocked("resume.py", [], responses)
        self.assertEqual(result.returncode, 0)
        self.assertTrue(body["ready"])
        self.assertEqual(body["complete"], {"mine": True, "explore": True})
        for key in ("active_mining", "active_exploration"):
            self.assertEqual(len(body[key]), 101)
            self.assertEqual(body[key][-1], last)
        self.assertFalse(any("--status" in call for call in calls))
        for command in ("mine", "explore"):
            result, body, _ = self.run_mocked("verify_write.py", [command + ".start", "--run-id", "last"], responses)
            self.assertTrue(body["confirmed"])
            self.assertFalse(body["safe_to_retry"])

    def test_recovery_rejects_invalid_totals_pages_and_duplicates(self):
        first = [self.row(f"run-{index}") for index in range(100)]
        malformed = [{}, self.page([], -1), self.page([], True), self.page([], "101"),
                     self.page([], 1.0), self.page([], 101), self.page([self.row()], 101),
                     self.page([self.row()], 0), self.page([self.row(), self.row()]),
                     self.page([{"fcRunId": None}]), self.page(["not a row"]),
                     {**self.page([]), "items": None}, {**self.page([]), "page": 2},
                     {**self.page([]), "size": 5}, {**self.page([]), "page": True}]
        for command in ("mine", "explore"):
            for data in malformed:
                with self.subTest(command=command, data=data):
                    responses = self.recovery_responses()
                    responses[f"{command} runs --page 1 --size 100"] = [0, data]
                    result, body, _ = self.run_mocked("resume.py", [], responses)
                    self.assertEqual(result.returncode, 2)
                    self.assertFalse(body["ready"])
                    self.assertFalse(body["complete"][command])
                    self.assertEqual(body["next_action"], "review_incomplete_recovery")
            for response in ([5, {"error": "offline"}], [0, self.page([], 101, page=2)],
                             [0, self.page([self.row("run-0")], 101, page=2)],
                             [0, self.page([self.row("last")], 102, page=2)],
                             [0, self.page([self.row("last")], 101, page=1)]):
                with self.subTest(command=command, response=response):
                    responses = self.recovery_responses()
                    responses[f"{command} runs --page 1 --size 100"] = [0, self.page(first, 101)]
                    responses[f"{command} runs --page 2 --size 100"] = response
                    result, body, _ = self.run_mocked("resume.py", [], responses)
                    self.assertFalse(body["ready"])
                    self.assertFalse(body["complete"][command])
                    self.assertEqual(len(body["active_mining" if command == "mine" else "active_exploration"]), 100)
                    result, body, _ = self.run_mocked("verify_write.py", [command + ".start", "--run-id", "run-0"], responses)
                    self.assertIsNone(body["confirmed"])
                    self.assertFalse(body["safe_to_retry"])

    def test_recovery_states_preserve_uncertainty(self):
        cases = [(self.row(status="queued"), True, True), (self.row(status="dispatching"), True, True),
                 (self.row(status="running"), True, True), (self.row(status="succeeded"), True, False),
                 (self.row(status="failed"), True, False), (self.row(status="timeout"), False, True),
                 (self.row(status="new-state"), False, True), (self.row(status="running", done=True), False, True),
                 (self.row(status="succeeded", ok=False), False, True),
                 (self.row(status="failed", done=None), False, True),
                 ({**self.row(), "status": {}}, False, True)]
        for row, ready, active in cases:
            with self.subTest(row=row):
                responses = self.recovery_responses(self.page([row]))
                result, body, _ = self.run_mocked("resume.py", [], responses)
                self.assertIs(body["ready"], ready)
                self.assertEqual(body["active_mining"], [row] if active else [])
                self.assertFalse(body["safe_to_retry"])
                result, body, _ = self.run_mocked("verify_write.py", ["mine.start", "--run-id", "run-1"], responses)
                self.assertIs(body["confirmed"], True if ready else None)
                self.assertFalse(body["safe_to_retry"])


if __name__ == "__main__":
    unittest.main()
