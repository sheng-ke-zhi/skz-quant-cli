#!/usr/bin/env python3
"""Regression tests for the npm postinstall adapter."""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class NpmInstallTests(unittest.TestCase):
    def test_installer_resolves_the_optional_dependency_by_alias(self) -> None:
        platform = f"{subprocess.check_output(['node', '-p', 'process.platform'], text=True).strip()}-{subprocess.check_output(['node', '-p', 'process.arch'], text=True).strip()}"
        aliases = {
            "darwin-arm64": ("skz-quant-cli-darwin-arm64", "skz"),
            "darwin-x64": ("skz-quant-cli-darwin-x64", "skz"),
            "linux-arm64": ("skz-quant-cli-linux-arm64", "skz"),
            "linux-x64": ("skz-quant-cli-linux-x64", "skz"),
            "win32-x64": ("skz-quant-cli-win32-x64", "skz.exe"),
        }
        alias, binary = aliases[platform]

        with tempfile.TemporaryDirectory() as temp:
            node_modules = Path(temp) / "node_modules"
            main = node_modules / "@shengkezhi-com" / "skz-quant-cli"
            native = node_modules / alias
            (main / "bin").mkdir(parents=True)
            (native / "bin").mkdir(parents=True)
            shutil.copy(ROOT / "npm" / "install.cjs", main / "install.cjs")
            (main / "bin" / "skz.exe").write_text("placeholder")
            (native / "bin" / binary).write_text("native-binary")
            (native / "package.json").write_text(json.dumps({"name": alias}))

            subprocess.run(["node", "install.cjs"], cwd=main, check=True)
            self.assertEqual((main / "bin" / "skz.exe").read_text(), "native-binary")


if __name__ == "__main__":
    unittest.main()
