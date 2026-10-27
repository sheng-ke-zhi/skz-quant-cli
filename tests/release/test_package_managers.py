#!/usr/bin/env python3
"""在本地 Git remote 验证两份 metadata 的原子提交与重试边界。"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts/release"))
import update_package_managers as pm


class PackageManagerTests(unittest.TestCase):
    def test_one_commit_noop_downgrade_and_concurrent_push(self):
        real_run = subprocess.run
        with tempfile.TemporaryDirectory() as temp, patch.dict(os.environ, {
            "GIT_AUTHOR_NAME": "test", "GIT_AUTHOR_EMAIL": "test@example.com",
            "GIT_COMMITTER_NAME": "test", "GIT_COMMITTER_EMAIL": "test@example.com",
        }):
            root = Path(temp)
            remote, seed = root / "remote.git", root / "seed"

            def git(*args, cwd=seed):
                return real_run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True).stdout.strip()

            git("init", "--bare", "--initial-branch=main", str(remote), cwd=root)
            git("clone", str(remote), str(seed), cwd=root)
            (seed / "README.md").write_text("preserve source\n")
            git("add", ".")
            git("commit", "-m", "initial")
            git("push", "origin", "main")
            initial = git("rev-parse", "HEAD")
            sums = root / "SHA256SUMS"
            names = [f"skz-{target}.tar.gz" for target in pm.DARWIN_LINUX_TARGETS]
            names.append(f"skz-{pm.WINDOWS_TARGET}.zip")
            sums.write_text("".join(f"{'a' * 64}  {name}\n" for name in names))
            concurrent = False

            def process(command, **kwargs):
                if command[:3] == ["gh", "repo", "clone"]:
                    command = ["git", "clone", "--depth", "1", "--branch", "main", remote.as_uri(), command[4]]
                if concurrent and command == ["git", "push"]:
                    git("pull", "--ff-only")
                    (seed / "README.md").write_text("concurrent source\n")
                    git("add", "README.md")
                    git("commit", "-m", "concurrent")
                    git("push", "origin", "main")
                return real_run(command, **kwargs)

            with patch.object(pm.subprocess, "run", side_effect=process), patch.object(pm, "require_tool"):
                pm.sync_package_managers(sums, version="1.2.3", dry_run=False)
                published = git("rev-parse", "main", cwd=remote)
                self.assertEqual(git("rev-list", "--count", f"{initial}..main", cwd=remote), "1")
                self.assertEqual(set(git("diff", "--name-only", initial, published, cwd=remote).splitlines()), {"Formula/skz.rb", "bucket/skz.json"})
                self.assertEqual(git("show", "main:README.md", cwd=remote), "preserve source")
                manifest = json.loads(git("show", "main:bucket/skz.json", cwd=remote))
                self.assertEqual(manifest["version"], "1.2.3")
                self.assertEqual(manifest["hash"], f"sha256:{'a' * 64}")
                pm.sync_package_managers(sums, version="1.2.3", dry_run=False)
                self.assertEqual(git("rev-parse", "main", cwd=remote), published)
                with self.assertRaisesRegex(SystemExit, "拒绝回退"):
                    pm.sync_package_managers(sums, version="1.2.2", dry_run=False)
                concurrent = True
                with self.assertRaises(subprocess.CalledProcessError):
                    pm.sync_package_managers(sums, version="1.2.4", dry_run=False)
                self.assertEqual(git("show", "main:README.md", cwd=remote), "concurrent source")
                self.assertEqual(json.loads(git("show", "main:bucket/skz.json", cwd=remote))["version"], "1.2.3")


if __name__ == "__main__":
    unittest.main()
