#!/usr/bin/env python3
"""Focused tests for release recovery and registry propagation handling."""

from __future__ import annotations

import importlib.util
import json
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "release" / "release_wsl.py"
sys.path.insert(0, str(SCRIPT.parent))


def load_release_module():
    spec = importlib.util.spec_from_file_location("release_wsl", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


release = load_release_module()


class ReleaseStateTests(unittest.TestCase):
    def test_release_archives_include_apache_license(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            for target in release.TARGETS:
                filename = "skz.exe" if target == release.WINDOWS_TARGET else "skz"
                binary = output / "binaries" / target / filename
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"binary")
            plugins = output / "plugins"
            plugins.mkdir()
            (plugins / "manifest.json").write_text("{}\n")

            def fake_capture(command: list[str], *, check: bool = True) -> str:
                if command[:2] == ["git", "show"]:
                    return "1704067200"
                return json.dumps({"cli": "1.2.3"})

            with patch.object(release, "capture", side_effect=fake_capture):
                release.prepare_release_assets(output, "1.2.3")

            tar_path = output / "github-release" / "skz-x86_64-unknown-linux-musl.tar.gz"
            with tarfile.open(tar_path) as bundle:
                self.assertEqual(bundle.extractfile("LICENSE").read(), (ROOT / "LICENSE").read_bytes())

            zip_path = output / "github-release" / f"skz-{release.WINDOWS_TARGET}.zip"
            with zipfile.ZipFile(zip_path) as bundle:
                self.assertEqual(bundle.read("LICENSE"), (ROOT / "LICENSE").read_bytes())

    def test_state_roundtrip_and_tamper_detection(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            assets_dir = output / "github-release"
            package_dir = output / "npm" / "main"
            assets_dir.mkdir(parents=True)
            package_dir.mkdir(parents=True)
            asset = assets_dir / "SHA256SUMS"
            payload = assets_dir / "skz-test.tar.gz"
            payload.write_bytes(b"archive")
            asset.write_text(f"{release.sha256(payload)}  {payload.name}\n")
            (package_dir / "package.json").write_text('{"name":"test"}\n')

            with patch.object(release, "capture", return_value="abc123"):
                release.write_release_state(
                    output, "1.2.3", [asset, payload], output / "npm"
                )
                assets, packages = release.load_release_state(output, "1.2.3")
            self.assertEqual([path.name for path in assets], ["SHA256SUMS", payload.name])
            self.assertEqual(packages, output / "npm")

            payload.write_bytes(b"changed")
            with patch.object(release, "capture", return_value="abc123"):
                with self.assertRaisesRegex(SystemExit, "SHA256 不匹配"):
                    release.load_release_state(output, "1.2.3")

    def test_npm_visibility_is_retried(self) -> None:
        responses = [["missing"], ["missing"], []]
        with (
            patch.object(release, "missing_npm_versions", side_effect=responses),
            patch.object(release.time, "sleep") as sleep,
            patch.object(release, "NPM_VISIBILITY_ATTEMPTS", 3),
            patch.object(release, "NPM_VISIBILITY_DELAY_SECONDS", 0),
            patch.object(release.tempfile, "TemporaryDirectory") as temporary,
        ):
            temporary.return_value.__enter__.side_effect = SystemExit("smoke reached")
            with self.assertRaisesRegex(SystemExit, "smoke reached"):
                release.verify_npm("1.2.3")
        self.assertEqual(sleep.call_count, 2)

    def test_existing_release_mismatch_requires_explicit_rebuild(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            asset = Path(temp) / "asset.zip"
            asset.write_bytes(b"local")
            with patch.object(
                release,
                "remote_release_assets",
                return_value={"asset.zip": "sha256:different"},
            ):
                with self.assertRaisesRegex(SystemExit, "--resume --rebuild"):
                    release.verify_remote_release_assets("v1.2.3", [asset])


if __name__ == "__main__":
    unittest.main()
