#!/usr/bin/env python3
"""Tests for npm package assembly."""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
)


class PreparePackagesTests(unittest.TestCase):
    def test_all_packages_include_apache_license(self) -> None:
        cargo_toml = (ROOT / "Cargo.toml").read_text()
        version_match = re.search(r'^version\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
        assert version_match is not None
        version = version_match.group(1)

        with tempfile.TemporaryDirectory() as temp:
            workspace = Path(temp)
            artifacts = workspace / "artifacts"
            plugins = workspace / "plugins"
            packages = workspace / "packages"
            for target in TARGETS:
                filename = "skz.exe" if "windows" in target else "skz"
                binary = artifacts / target / filename
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"binary")
            plugins.mkdir()
            (plugins / "manifest.json").write_text(
                json.dumps({"cli": version, "files": [{"path": "skill.txt"}]})
            )
            (plugins / "skill.txt").write_text("skill\n")

            subprocess.run(
                [
                    "node",
                    "npm/prepare-packages.mjs",
                    str(artifacts),
                    str(plugins),
                    str(packages),
                ],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )

            expected_license = (ROOT / "LICENSE").read_bytes()
            package_dirs = [path for path in packages.iterdir() if path.is_dir()]
            self.assertEqual(len(package_dirs), 6)
            for package_dir in package_dirs:
                self.assertEqual((package_dir / "LICENSE").read_bytes(), expected_license)
                metadata = json.loads((package_dir / "package.json").read_text())
                self.assertEqual(metadata["license"], "Apache-2.0")


if __name__ == "__main__":
    unittest.main()
