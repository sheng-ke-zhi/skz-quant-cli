#!/usr/bin/env python3
"""Exercise release boundaries without publishing or compiling five platforms."""

from __future__ import annotations

import json
import os
import shutil
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/release"))
import build_target as builder
import common
import release_ci as ci


class ReleaseCITests(unittest.TestCase):
    def test_tag_requires_matching_version_and_successful_main_ci(self) -> None:
        version = common.cargo_field("version")
        with (
            patch.dict(os.environ, {"GITHUB_REF": f"refs/tags/v{version}", "GITHUB_REPOSITORY": "owner/repo"}),
            patch.object(ci.release, "validate_prepared_release"),
            patch.object(ci, "run"),
            patch.object(ci.time, "sleep") as sleep,
            patch.object(ci.release, "capture") as capture,
        ):
            pending = {"workflow_runs": [{"status": "in_progress"}]}
            success = {"workflow_runs": [{"status": "completed", "conclusion": "success", "html_url": "ci-url"}]}
            capture.side_effect = [f"v{version}", "commit", json.dumps(pending), json.dumps(success)]
            ci.validate_tag()
            sleep.assert_called_once_with(10)
            self.assertIn("head_sha=commit", capture.call_args.args[0])
            self.assertIn("branch=main", capture.call_args.args[0])
            failure = {"workflow_runs": [{"status": "completed", "conclusion": "failure", "html_url": "ci-url"}]}
            capture.side_effect = [f"v{version}", "commit", json.dumps(failure)]
            with self.assertRaisesRegex(SystemExit, "CI 未通过"):
                ci.validate_tag()
            capture.side_effect = [f"v{version}", "commit", '{"workflow_runs":[]}']
            with patch.object(ci.time, "monotonic", side_effect=[0, 601]):
                with self.assertRaisesRegex(SystemExit, "超时"):
                    ci.validate_tag()
            capture.side_effect = None
            capture.return_value = "v999.0.0"
            with self.assertRaisesRegex(SystemExit, "回退安装渠道"):
                ci.validate_tag()
            os.environ["GITHUB_REF"] = "refs/tags/v0.0.0"
            with self.assertRaisesRegex(SystemExit, "tag"):
                ci.validate_tag()

    def test_native_and_cross_linker_selection(self) -> None:
        cases = [
            ("Linux", "aarch64", "aarch64-unknown-linux-musl", "build", "musl-gcc"),
            ("Linux", "x86_64", "aarch64-unknown-linux-musl", "zigbuild", "zig"),
            ("Linux", "x86_64", "x86_64-unknown-linux-musl", "build", "musl-gcc"),
            ("Windows", "AMD64", "x86_64-pc-windows-gnu", "build", "gcc"),
            ("Linux", "x86_64", "x86_64-pc-windows-gnu", "build", "x86_64-w64-mingw32-gcc"),
            ("Darwin", "arm64", "aarch64-apple-darwin", "build", None),
            ("Linux", "x86_64", "aarch64-apple-darwin", "zigbuild", "zig"),
        ]
        for system, machine, target, command, linker in cases:
            with self.subTest(system=system, target=target), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                filename = "skz.exe" if "windows" in target else "skz"
                binary = root / "target" / target / "dist" / filename
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"binary")
                with (
                    patch.object(builder, "ROOT", root),
                    patch.object(builder.platform, "system", return_value=system),
                    patch.object(builder.platform, "machine", return_value=machine),
                    patch.object(builder, "_ensure_rust_target"),
                    patch.object(builder, "require_tool", side_effect=lambda tool, hint: tool),
                    patch.object(builder.shutil, "which", return_value="zig"),
                    patch.object(builder, "run") as run,
                ):
                    builder.build_target(target, root / "out")
                self.assertEqual(run.call_args.args[0][:2], ["cargo", command])
                if linker:
                    key = "CARGO_ZIGBUILD_ZIG_PATH" if command == "zigbuild" else f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER"
                    self.assertEqual(run.call_args.kwargs["env"][key], linker)

    def test_matrix_transport_packaging_and_locked_state_roundtrip(self) -> None:
        version = common.cargo_field("version")
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "release"

            def fake_build(target: str, destination: Path) -> dict[str, str]:
                filename = "skz.exe" if "windows" in target else "skz"
                relative = f"binaries/{target}/{filename}"
                binary = destination / relative
                binary.parent.mkdir(parents=True)
                binary.write_bytes(f"binary for {target}".encode())
                binary.chmod(0o755)
                return {"target": target, "binary": relative, "binary_sha256": common.sha256(binary)}

            def fake_capture(command: list[str], **kwargs) -> str:
                if command[:2] == ["git", "rev-parse"]:
                    return "commit"
                if command[:2] == ["git", "show"]:
                    return "1704067200"
                return json.dumps({"cli": version})

            installer = Path(temp) / f"SKZ-WorkBuddy-Setup-{version}.exe"
            installer.write_bytes(b"installer")
            with (
                patch.object(ci, "build_target", side_effect=fake_build),
                patch.object(common, "git_commit", return_value="commit"),
                patch.object(common, "git_dirty", return_value=False),
                patch.object(ci.release, "capture", side_effect=fake_capture),
                patch.object(ci.release, "build_installer", return_value=installer) as build_installer,
            ):
                for target in ci.release.TARGETS:
                    ci.build(output, target)
                downloads = output / "downloads"
                downloads.mkdir()
                for path in output.glob("build-*.tar.gz"):
                    shutil.move(path, downloads / path.name)
                shutil.rmtree(output / "binaries")
                shutil.rmtree(output / "manifests")
                ci.package(output)
                build_installer.assert_called_once_with(output, version)
                with tarfile.open(output / "release-payload.tar.gz") as archive:
                    restored = Path(temp) / "restored"
                    archive.extractall(restored, filter="data")
                assets, packages = ci.release.load_release_state(restored, version)
                self.assertEqual(len(assets), 7)
                self.assertEqual(len(list(packages.glob("*/package.json"))), 6)
                main = packages / "skz-quant-cli/bin/plugins"
                self.assertTrue((main / "claude/.claude-plugin/marketplace.json").is_file())
                self.assertTrue((main / "codex/.agents/plugins/marketplace.json").is_file())
                with tarfile.open(restored / "github-release/skz-x86_64-unknown-linux-musl.tar.gz") as archive:
                    self.assertEqual(archive.getmember("skz").mode & 0o111, 0o111)

                # Packaging must reject mixed commits and altered binaries before publication.
                path = output / "manifests" / f"{ci.release.TARGETS[0]}.json"
                original = path.read_text()
                manifest = json.loads(original)
                manifest["commit"] = "other-commit"
                path.write_text(json.dumps(manifest))
                with self.assertRaisesRegex(SystemExit, "构建版本、提交"):
                    ci.validate_builds(output)
                path.write_text(original)
                binary = output / manifest["artifacts"][0]["binary"]
                binary.write_bytes(b"tampered")
                with self.assertRaisesRegex(SystemExit, "SHA256"):
                    ci.validate_builds(output)
                path.unlink()
                with self.assertRaisesRegex(SystemExit, "manifest 不完整"):
                    ci.validate_builds(output)

    def test_publish_reuses_state_and_repository_token_on_failure(self) -> None:
        version = common.cargo_field("version")
        with (
            patch.dict(os.environ, {"GITHUB_REF": f"refs/tags/v{version}", "GH_TOKEN": "main-token"}),
            patch.object(ci.release, "load_release_state", return_value=([Path("asset")], Path("npm"))) as load,
            patch.object(ci.release, "capture", return_value=f"v{version}"),
            patch.object(ci.release, "publish_github_release") as github,
            patch.object(ci.release, "publish_npm") as npm,
            patch.object(ci, "run"),
            patch.object(ci.release, "sync_package_managers") as sync,
            patch.object(ci.release, "verify_publication") as verify,
            patch.object(ci, "build") as build,
            patch.object(ci, "package") as package,
        ):
            def fail_sync(*args, **kwargs):
                self.assertEqual(os.environ["GH_TOKEN"], "main-token")
                raise RuntimeError("push failed")

            sync.side_effect = fail_sync
            with self.assertRaisesRegex(RuntimeError, "push failed"):
                ci.publish(Path("output"))
            self.assertEqual(os.environ["GH_TOKEN"], "main-token")
            load.assert_called_once()
            github.assert_called_once()
            npm.assert_called_once()
            verify.assert_not_called()
            build.assert_not_called()
            package.assert_not_called()


if __name__ == "__main__":
    unittest.main()
