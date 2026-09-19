#!/usr/bin/env python3
"""Golden and deterministic behavior tests for the generated SKZ plugins."""

from __future__ import annotations

import importlib.util
import json
import os
import re
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PLUGINS = ROOT / "plugins"
AUTHORING = ROOT / "plugin-src"
BOOKS = tuple(name.removeprefix("skz-") for name in (AUTHORING / "skills.txt").read_text().splitlines())
TARGETS = ("claude", "codex", "openclaw", "hermes", "dsh", "workbuddy")
SCRIPTS = AUTHORING / "common" / "scripts"
GOLDENS = json.loads((Path(__file__).parent / "golden_prompts.json").read_text(encoding="utf-8"))
OPEN_API_ROUTES = set(json.loads((Path(__file__).parent / "open_api_routes.json").read_text(encoding="utf-8")))


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
    def test_openapi_sync_rewrites_links_for_the_offline_bundle(self) -> None:
        script = ROOT / "scripts/release/sync_openapi_refs.py"
        spec = importlib.util.spec_from_file_location("sync_openapi_refs", script)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

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

    def test_openapi_references_have_no_broken_relative_links(self) -> None:
        references = AUTHORING / "books/skz-openapi/references"
        for path in references.rglob("*.md"):
            text = path.read_text(encoding="utf-8")
            for target in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", text):
                target = target.strip()
                if target.startswith(("http://", "https://", "mailto:", "#")):
                    continue
                relative = target.split("#", 1)[0]
                self.assertTrue(
                    (path.parent / relative).is_file(),
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
        numerals = "一二三四五六七八九十"
        self.assertLessEqual(len(expected), len(numerals))
        heading = f"{numerals[len(expected) - 1]}册分工"
        named = re.compile(r"`(skz-[a-z0-9-]+)`")
        for book in BOOKS:
            skill = (AUTHORING / "books" / f"skz-{book}" / "SKILL.md").read_text(encoding="utf-8")
            matches = [line for line in skill.splitlines() if "册分工" in line]
            self.assertEqual(len(matches), 1, f"skz-{book} should have exactly one 册分工 line")
            self.assertTrue(
                matches[0].startswith(heading),
                f"skz-{book} heading should be {heading}",
            )
            found = set(named.findall(matches[0]))
            self.assertEqual(found, expected, f"skz-{book} division table drifted from skills.txt")

    def test_skill_install_targets_match_cli(self) -> None:
        rust = (ROOT / "src/plugin.rs").read_text(encoding="utf-8")
        cli_targets = tuple(re.findall(r'Self::\w+ => "([a-z]+)"', rust))
        self.assertEqual(cli_targets, TARGETS)
        expected = "|".join((*TARGETS, "all"))
        listed = re.compile(r"skz plugin install <([^>]+)>")
        found: set[str] = set()
        for book in BOOKS:
            skill = (AUTHORING / "books" / f"skz-{book}" / "SKILL.md").read_text(encoding="utf-8")
            matches = listed.findall(skill)
            for targets in matches:
                self.assertEqual(
                    targets,
                    expected,
                    f"skz-{book} install targets drifted from Target::ALL",
                )
                found.add(f"skz-{book}")
        self.assertGreaterEqual(
            found,
            {
                "skz-guide",
                "skz-factor",
                "skz-candidate",
                "skz-strategy",
                "skz-portfolio",
                "skz-create-problem",
            },
        )

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
        create_problem = (AUTHORING / "books/skz-create-problem/SKILL.md").read_text(encoding="utf-8")
        sections = (
            (guide.split("### ②", 1)[1].split("### ③", 1)[0], "skz route create", ("每条路线", "七个字段")),
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
        section = guide.split("### ②", 1)[1].split("### ③", 1)[0]

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
    data = {'total': 0, 'items': []}
elif args[:2] == ['explore', 'runs']:
    data = {'total': 1, 'items': [{'fcRunId': 'E1', 'status': 'running', 'done': False}]}
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

            verify = run_script("verify_write.py", "strategy.write", "--code", "S1", env=env)
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
            ("problem.create", ["--code", "P1"], 0, {"code": "P1"}, True, "present"),
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
                "code = int(os.environ['VERIFY_EXIT'])\n"
                "print(os.environ['VERIFY_DATA'], file=sys.stderr if code else sys.stdout)\n"
                "sys.exit(code)\n"
            )
            fake.chmod(0o755)
            for operation, args, code, data, confirmed, meaning in cases:
                with self.subTest(operation=operation, code=code, data=data):
                    result = run_script("verify_write.py", operation, *args, env={
                        **os.environ, "SKZ_BIN": str(fake), "VERIFY_EXIT": str(code),
                        "VERIFY_DATA": json.dumps(data),
                    })
                    self.assertEqual(result.returncode, 2 if confirmed is None else 0, result.stderr)
                    body = json.loads(result.stdout)
                    self.assertIs(body["confirmed"], confirmed)
                    self.assertEqual(body["meaning"], meaning)
                    self.assertFalse(body["safe_to_retry"])

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


if __name__ == "__main__":
    unittest.main()
