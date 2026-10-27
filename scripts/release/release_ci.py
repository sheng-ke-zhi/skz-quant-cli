#!/usr/bin/env python3
"""Tag release stages; reuse the existing archive, registry and recovery code."""

from __future__ import annotations

import argparse
import json
import os
import re
import tarfile
import time
from pathlib import Path

import release_wsl as release
from build_plugins import build_bundle
from build_target import build_target
from common import DEFAULT_OUTPUT, cargo_field, run, sha256, write_manifest


def validate_ref(version: str) -> None:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version) or os.environ.get("GITHUB_REF") != f"refs/tags/v{version}":
        raise SystemExit("仅接受与 Cargo.toml version 一致的 vX.Y.Z tag")
    tags = release.capture(["git", "tag", "--list"]).splitlines()
    for tag in tags:
        if re.fullmatch(r"v\d+\.\d+\.\d+", tag) and tuple(map(int, tag[1:].split("."))) > tuple(map(int, version.split("."))):
            raise SystemExit(f"已有更新的 {tag}，拒绝重跑旧版本发布，以免回退安装渠道")


def validate_tag() -> None:
    version = cargo_field("version")
    tag = f"v{version}"
    validate_ref(version)
    # checkout 会把触发 tag 改成 commit ref；恢复远端 tag 对象后再严格校验。
    run(["git", "fetch", "origin", f"+refs/tags/{tag}:refs/tags/{tag}"])
    release.validate_prepared_release(version, tag)
    run(["git", "merge-base", "--is-ancestor", "HEAD", "origin/main"])
    commit = release.capture(["git", "rev-parse", "HEAD"])
    endpoint = f"repos/{os.environ['GITHUB_REPOSITORY']}/actions/workflows/ci.yml/runs"
    deadline = time.monotonic() + 600
    while True:
        runs = json.loads(release.capture([
            "gh", "api", "--method", "GET", endpoint,
            "-f", f"head_sha={commit}", "-f", "branch=main", "-f", "event=push",
            "-f", "per_page=1",
        ]))["workflow_runs"]
        if runs and runs[0]["status"] == "completed":
            if runs[0]["conclusion"] != "success":
                raise SystemExit(f"同一提交的 main CI 未通过：{runs[0]['html_url']}")
            print(f"复用已通过的 main CI：{runs[0]['html_url']}")
            return
        if time.monotonic() >= deadline:
            raise SystemExit("等待 main CI 超时；先让该提交的 ci.yml 通过，再重跑失败任务")
        print("等待同一提交的 main CI，无需重复运行测试……", flush=True)
        time.sleep(10)


def build(output: Path, target: str) -> None:
    record = build_target(target, output)
    binary = output / record["binary"]
    actual = json.loads(release.capture([str(binary), "--version"]))
    if actual.get("cli") != cargo_field("version"):
        raise SystemExit(f"{target} 发布二进制版本不匹配：{actual}")
    manifest = write_manifest(target, [record], output)
    # Tar preserves executable bits that Actions artifact upload/download loses.
    with tarfile.open(output / f"build-{target}.tar.gz", "w:gz", compresslevel=1) as archive:
        archive.add(binary, arcname=record["binary"])
        archive.add(manifest, arcname=f"manifests/{target}.json")


def validate_builds(output: Path) -> None:
    version = cargo_field("version")
    commit = release.capture(["git", "rev-parse", "HEAD"])
    manifests = {path.stem: path for path in (output / "manifests").glob("*.json")}
    if set(manifests) != set(release.TARGETS):
        raise SystemExit("五目标构建 manifest 不完整或包含额外目标")
    for target, path in manifests.items():
        manifest = json.loads(path.read_text())
        if (manifest.get("version"), manifest.get("commit"), manifest.get("dirty")) != (version, commit, False):
            raise SystemExit(f"{target} 的构建版本、提交或工作树状态不匹配")
        filename = "skz.exe" if target == release.WINDOWS_TARGET else "skz"
        relative = f"binaries/{target}/{filename}"
        binary = output / relative
        expected = {
            "target": target, "binary": relative,
            "binary_sha256": sha256(binary) if binary.is_file() else None,
        }
        if not binary.is_file() or manifest.get("artifacts") != [expected]:
            raise SystemExit(f"{target} 二进制缺失或 SHA256 不匹配")


def package(output: Path) -> None:
    for path in sorted((output / "downloads").glob("build-*.tar.gz")):
        with tarfile.open(path) as archive:
            archive.extractall(output, filter="data")
    validate_builds(output)
    version = cargo_field("version")
    build_bundle(output / "plugins")
    release.validate_artifacts(output, version)
    packages = release.prepare_npm_packages(output)
    assets = release.prepare_release_assets(output, version)
    release.write_release_state(output, version, assets, packages)
    with tarfile.open(output / "release-payload.tar.gz", "w:gz", compresslevel=1) as archive:
        for name in ("github-release", "npm", release.RELEASE_STATE):
            archive.add(output / name, arcname=name)


def publish(output: Path) -> None:
    version = cargo_field("version")
    validate_ref(version)
    assets, packages = release.load_release_state(output, version)
    tag = f"v{version}"
    release.publish_github_release(tag, assets)
    release.publish_npm(packages)
    run(["gh", "auth", "setup-git"])
    release.sync_package_managers(
        output / "github-release/SHA256SUMS", version=version, dry_run=False,
    )
    release.verify_publication(version, tag, assets, "origin")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("validate", "build", "package", "publish"))
    parser.add_argument("--target", choices=release.TARGETS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    output = args.output.resolve()
    if args.stage == "validate":
        validate_tag()
    elif args.stage == "build":
        if not args.target:
            parser.error("build 需要 --target")
        build(output, args.target)
    elif args.stage == "package":
        package(output)
    else:
        publish(output)


if __name__ == "__main__":
    main()
