#!/usr/bin/env python3
"""Golden and deterministic behavior tests for the generated skill bundle."""

from __future__ import annotations

import json
import os
import re
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SKILLS = ROOT / "skills"
AUTHORING = ROOT / "skill-src"
BOOKS = ("factor", "candidate", "strategy", "guide", "portfolio")
TARGETS = ("claude", "codex", "openclaw", "hermes")
SCRIPTS = AUTHORING / "common" / "scripts"
GOLDENS = json.loads((Path(__file__).parent / "golden_prompts.json").read_text(encoding="utf-8"))


def run_script(name: str, *args: str, stdin: object | None = None, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(SCRIPTS / name), *args],
        input=None if stdin is None else json.dumps(stdin, ensure_ascii=False),
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )


class SkillBundleTests(unittest.TestCase):
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

    def test_rendered_targets_are_identical_and_self_contained(self) -> None:
        for book in BOOKS:
            canonical = SKILLS / "codex" / f"skz-{book}"
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
                root = SKILLS / target / f"skz-{book}"
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

        self.assertIn("有实验没评审的 → 去 `skz skills candidate`", guide)
        self.assertIn("入库后再交给 `skz-strategy` 暂停观察", guide)

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
    data = {'cli': 'test', 'contract': '3.5'}
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
    data = {'items': [{'code': 'PF1', 'job_status': 'running'}]}
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
            self.assertEqual(resumed["pending_portfolios"][0]["code"], "PF1")

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


if __name__ == "__main__":
    unittest.main()
