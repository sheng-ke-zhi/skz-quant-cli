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
import hashlib
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
import build_windows_installer as windows_installer


class ReleaseStateTests(unittest.TestCase):
    def test_prepared_release_requires_annotated_tag(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "CHANGELOG.md").write_text("## v1.2.3 (2026-10-27)\n")

            def capture(command: list[str], **kwargs) -> str:
                if command[:2] == ["git", "status"]:
                    return ""
                if command[:2] == ["git", "cat-file"]:
                    return "commit"
                return "commit-sha"

            with (
                patch.object(release, "ROOT", root),
                patch.object(release, "capture", side_effect=capture),
                patch.object(release.subprocess, "run") as process,
            ):
                process.return_value.returncode = 0
                with self.assertRaisesRegex(SystemExit, "必须是 annotated tag"):
                    release.validate_prepared_release("1.2.3", "v1.2.3")

    def test_windows_installer_locks_payload_versions_and_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            binary = output / "binaries/x86_64-pc-windows-gnu/skz.exe"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"binary")
            plugins = output / "plugins"
            plugins.mkdir()
            (plugins / "manifest.json").write_text('{"cli":"1.2.3"}\n')

            def compile_installer(command: list[str], **kwargs):
                target = next(arg.removeprefix("-DOUTPUT=") for arg in command if arg.startswith("-DOUTPUT="))
                Path(target).write_bytes(b"installer")

            with (patch.object(windows_installer, "require_tool", return_value="makensis"),
                  patch.object(windows_installer.subprocess, "run", side_effect=compile_installer)):
                installer = windows_installer.build_installer(output, "1.2.3")
                self.assertTrue(installer.is_file())
                friendly = installer.with_name("胜可知WorkBuddy一键启动.exe")
                self.assertEqual(friendly.read_bytes(), installer.read_bytes())
                payload = output / "windows-installer/payload"
                manifest = json.loads((payload / "installer-manifest.json").read_text())
                self.assertEqual(manifest["cli"], "1.2.3")
                for file in manifest["files"]:
                    self.assertEqual(hashlib.sha256((payload / file["path"]).read_bytes()).hexdigest(), file["sha256"])
                for name in ("setup.ps1", "update.ps1"):
                    self.assertTrue((payload / name).read_bytes().startswith(b"\xef\xbb\xbf"))
                with self.assertRaisesRegex(SystemExit, "版本"):
                    windows_installer.build_installer(output, "1.2.4")

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

            installer = output / "windows-installer/SKZ-WorkBuddy-Setup-1.2.3.exe"
            installer.parent.mkdir()
            installer.write_bytes(b"installer")
            with (patch.object(release, "capture", side_effect=fake_capture),
                  patch.object(release, "build_installer", return_value=installer) as build):
                release.prepare_release_assets(output, "1.2.3")
            build.assert_called_once_with(output, "1.2.3")
            published = output / "github-release" / installer.name
            self.assertEqual(published.read_bytes(), b"installer")
            self.assertIn(installer.name, (output / "github-release/SHA256SUMS").read_text())

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

    def test_existing_release_mismatch_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            asset = Path(temp) / "asset.zip"
            asset.write_bytes(b"local")
            with patch.object(
                release,
                "remote_release_assets",
                return_value={"asset.zip": "sha256:different"},
            ):
                with self.assertRaisesRegex(SystemExit, "拒绝覆盖"):
                    release.verify_remote_release_assets("v1.2.3", [asset])

    def test_interrupted_release_upload_only_adds_missing_assets(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            first, second = Path(temp) / "first.zip", Path(temp) / "second.zip"
            first.write_bytes(b"first")
            second.write_bytes(b"second")
            partial = {first.name: f"sha256:{release.sha256(first)}"}
            complete = {**partial, second.name: f"sha256:{release.sha256(second)}"}
            with (
                patch.object(release.subprocess, "run") as process,
                patch.object(release, "remote_release_assets", side_effect=[partial, complete]),
                patch.object(release, "run") as run,
            ):
                process.return_value.returncode = 0
                release.publish_github_release("v1.2.3", [first, second])
            run.assert_called_once_with([
                "gh", "release", "upload", "v1.2.3", str(second), "-R", release.HOMEPAGE_REPO,
            ])

    def test_local_release_only_pushes_tag_without_building_or_publishing(self) -> None:
        for resume in (False, True):
            with (
                self.subTest(resume=resume),
                patch.object(sys, "argv", [str(SCRIPT), *(["--resume"] if resume else [])]),
                patch.object(release, "preflight"),
                patch.object(release, "prepare_release", return_value=("1.2.3", "v1.2.3")),
                patch.object(release, "validate_prepared_release"),
                patch.object(release, "run") as run,
                patch.object(release, "publish_github_release") as publish,
                patch.object(release, "prepare_release_assets") as package,
            ):
                release.main()
                commands = [call.args[0] for call in run.call_args_list]
                self.assertEqual(commands[-1], [
                    "git", "push", "--atomic", "origin", "HEAD:refs/heads/main", "refs/tags/v1.2.3",
                ])
                self.assertFalse(any(command[:2] == ["cargo", "build"] for command in commands))
                publish.assert_not_called()
                package.assert_not_called()


if __name__ == "__main__":
    unittest.main()
