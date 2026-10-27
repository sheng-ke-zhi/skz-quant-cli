#!/usr/bin/env python3
"""维护者准备版本并推送 tag；五目标构建与发布由 GitHub Actions 完成。"""

from __future__ import annotations

import argparse
import base64
import json
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
import zipfile
from datetime import datetime, timezone
from pathlib import Path

from common import ROOT, cargo_field, run, sha256
from build_windows_installer import build_installer
from update_package_managers import (
    HOMEPAGE_REPO, sync_package_managers, parse_sha256sums,
    required_asset_hashes, render_formula, render_scoop_manifest,
)

RELEASE_BRANCH = "main"
TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
)
ARCHIVE_TARGETS = TARGETS[:-1]
WINDOWS_TARGET = TARGETS[-1]
NPM_PACKAGE = "@shengkezhi-com/skz-quant-cli"
RELEASE_STATE = "release-state.json"
NPM_VISIBILITY_ATTEMPTS = 30
NPM_VISIBILITY_DELAY_SECONDS = 10


def capture(command: list[str], *, check: bool = True) -> str:
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise SystemExit(f"命令失败：{' '.join(command)}\n{detail}")
    return result.stdout.strip()


def command_available(name: str) -> bool:
    return shutil.which(name) is not None


def preflight(remote: str, *, resume: bool) -> None:
    """一次报告所有可预见问题，任何写操作都在它之后。"""
    failures: list[str] = []
    required = {
        "git": "安装 Git",
        "uvx": "安装 uv",
        "cargo": "通过 rustup 安装 Rust",
    }
    for tool, hint in required.items():
        if not command_available(tool):
            failures.append(f"缺 {tool}；{hint}")

    branch = capture(["git", "branch", "--show-current"], check=False)
    if branch != RELEASE_BRANCH:
        failures.append(
            f"当前分支是 {branch or 'detached HEAD'}，发布必须从 {RELEASE_BRANCH} 进行"
        )
    if capture(["git", "status", "--porcelain"], check=False):
        failures.append("工作树不干净，请先提交或移走改动")

    local_head = capture(["git", "rev-parse", "HEAD"], check=False)
    remote_head = ""
    if command_available("git"):
        fetched = subprocess.run(
            ["git", "fetch", remote, RELEASE_BRANCH, "--quiet"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if fetched.returncode != 0:
            failures.append(
                f"无法获取 {remote}/{RELEASE_BRANCH}：{fetched.stderr.strip()}"
            )
        else:
            remote_head = capture(
                ["git", "rev-parse", f"refs/remotes/{remote}/{RELEASE_BRANCH}"],
                check=False,
            )
            if resume:
                ancestor = subprocess.run(
                    ["git", "merge-base", "--is-ancestor", remote_head, local_head],
                    cwd=ROOT,
                )
                if ancestor.returncode != 0:
                    failures.append(
                        f"本地 {RELEASE_BRANCH} 不是 {remote}/{RELEASE_BRANCH} 的快进后继"
                    )
            elif local_head != remote_head:
                failures.append(
                    f"本地 {RELEASE_BRANCH}({local_head}) 与 {remote}/{RELEASE_BRANCH}({remote_head}) 不一致；"
                    "若是上次 bump 后失败，请改用 --resume"
                )

    version = cargo_field("version")
    tag = f"v{version}"
    if resume:
        tag_commit = capture(["git", "rev-list", "-n", "1", tag], check=False)
        if tag_commit != local_head:
            failures.append(f"--resume 要求本地 {tag} 指向当前 HEAD")

    if failures:
        detail = "\n".join(f"  - {failure}" for failure in failures)
        raise SystemExit(f"发版预检失败，共 {len(failures)} 项：\n{detail}")
    print(
        f"预检通过：{RELEASE_BRANCH}、干净工作树、{remote}/{RELEASE_BRANCH}、"
        "版本准备工具就绪"
    )


def prepare_release(*, resume: bool) -> tuple[str, str]:
    if resume:
        version = cargo_field("version")
        tag = f"v{version}"
        print(f"继续发布已准备的 {tag}")
        return version, tag

    run(
        [
            "uvx",
            "--from",
            "commitizen",
            "cz",
            "bump",
            "--increment",
            "PATCH",
            "--dry-run",
            "--yes",
        ]
    )
    run(["uvx", "--from", "commitizen", "cz", "bump", "--increment", "PATCH", "--yes"])
    version = cargo_field("version")
    return version, f"v{version}"


def validate_prepared_release(version: str, tag: str) -> None:
    head = capture(["git", "rev-parse", "HEAD"])
    tag_commit = capture(["git", "rev-list", "-n", "1", tag], check=False)
    failures = []
    if tag != f"v{version}":
        failures.append(f"tag {tag} 与 Cargo.toml version {version} 不一致")
    if tag_commit != head:
        failures.append(f"本地 annotated tag {tag} 未指向 HEAD")
    if capture(["git", "cat-file", "-t", f"refs/tags/{tag}"], check=False) != "tag":
        failures.append(f"{tag} 必须是 annotated tag（git tag -a）")
    if capture(["git", "status", "--porcelain"], check=False):
        failures.append("bump 后工作树不干净")
    if f"## {tag} " not in (ROOT / "CHANGELOG.md").read_text():
        failures.append(f"CHANGELOG.md 没有 {tag} 条目")
    metadata = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env={**os.environ, "RUSTC_WRAPPER": ""},
    )
    if metadata.returncode != 0:
        failures.append(f"Cargo.lock 与 Cargo.toml 不一致：{metadata.stderr.strip()}")
    if failures:
        raise SystemExit(
            "版本准备校验失败：\n" + "\n".join(f"  - {item}" for item in failures)
        )


def validate_artifacts(output: Path, version: str) -> None:
    missing = []
    for target in TARGETS:
        filename = "skz.exe" if target == WINDOWS_TARGET else "skz"
        path = output / "binaries" / target / filename
        if not path.is_file():
            missing.append(str(path))
    if not (output / "plugins" / "manifest.json").is_file():
        missing.append(str(output / "plugins" / "manifest.json"))
    if missing:
        raise SystemExit("发布产物不完整：\n  - " + "\n  - ".join(missing))


def prepare_release_assets(output: Path, version: str) -> list[Path]:
    assets = output / "github-release"
    shutil.rmtree(assets, ignore_errors=True)
    assets.mkdir(parents=True)
    commit_time = int(capture(["git", "show", "-s", "--format=%ct", "HEAD"]))

    def normalized_tar_info(info: tarfile.TarInfo) -> tarfile.TarInfo:
        info.uid = 0
        info.gid = 0
        info.uname = ""
        info.gname = ""
        info.mtime = commit_time
        return info

    for target in ARCHIVE_TARGETS:
        binary = output / "binaries" / target / "skz"
        binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        archive = assets / f"skz-{target}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(binary, arcname="skz", filter=normalized_tar_info)
            bundle.add(output / "plugins", arcname="plugins", filter=normalized_tar_info)
            bundle.add(ROOT / "LICENSE", arcname="LICENSE", filter=normalized_tar_info)

    windows_binary = output / "binaries" / WINDOWS_TARGET / "skz.exe"
    zip_time = datetime.fromtimestamp(commit_time, timezone.utc).timetuple()[:6]
    zip_info = zipfile.ZipInfo("skz.exe", zip_time)
    zip_info.external_attr = (stat.S_IFREG | 0o755) << 16
    zip_info.compress_type = zipfile.ZIP_DEFLATED
    with zipfile.ZipFile(
        assets / f"skz-{WINDOWS_TARGET}.zip",
        "w",
        compression=zipfile.ZIP_DEFLATED,
    ) as bundle:
        bundle.writestr(zip_info, windows_binary.read_bytes())
        license_info = zipfile.ZipInfo("LICENSE", zip_time)
        license_info.external_attr = (stat.S_IFREG | 0o644) << 16
        license_info.compress_type = zipfile.ZIP_DEFLATED
        bundle.writestr(license_info, (ROOT / "LICENSE").read_bytes())
        for path in sorted((output / "plugins").rglob("*")):
            if path.is_file():
                info = zipfile.ZipInfo((Path("plugins") / path.relative_to(output / "plugins")).as_posix(), zip_time)
                info.external_attr = (stat.S_IFREG | (path.stat().st_mode & 0o777)) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                bundle.writestr(info, path.read_bytes())

    installer = build_installer(output, version)
    shutil.copy2(installer, assets / installer.name)
    files = sorted(path for path in assets.iterdir() if path.is_file())
    checksum = assets / "SHA256SUMS"
    checksum.write_text("".join(f"{sha256(path)}  {path.name}\n" for path in files))
    verify_sha256sums(checksum)

    linux_binary = output / "binaries" / "x86_64-unknown-linux-musl" / "skz"
    version_json = json.loads(capture([str(linux_binary), "--version"]))
    if version_json.get("cli") != version:
        raise SystemExit(
            f"Linux 发布二进制版本是 {version_json.get('cli')}，预期 {version}"
        )
    return sorted(path for path in assets.iterdir() if path.is_file())


def prepare_npm_packages(output: Path) -> Path:
    packages = output / "npm"
    run(
        [
            "node",
            "npm/prepare-packages.mjs",
            str(output / "binaries"),
            str(output / "plugins"),
            str(packages),
        ]
    )
    return packages


def npm_versions(version: str) -> list[tuple[str, str]]:
    platforms = ("darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64")
    platform_versions = [
        (f"@shengkezhi-com/skz-quant-cli-{platform}", f"{version}-{platform}")
        for platform in platforms
    ]
    return [*platform_versions, (NPM_PACKAGE, version)]


def missing_npm_versions(version: str) -> list[str]:
    missing = []
    for package_name, expected in npm_versions(version):
        actual = capture(
            ["npm", "view", f"{package_name}@{expected}", "version", "--json"],
            check=False,
        )
        try:
            found = json.loads(actual)
        except json.JSONDecodeError:
            found = None
        if found != expected:
            missing.append(f"{package_name}@{expected}")
    return missing


def publish_npm(packages: Path) -> None:
    platform_dirs = sorted(
        path
        for path in packages.iterdir()
        if path.is_dir() and json.loads((path / "package.json").read_text())["name"] != NPM_PACKAGE
    )
    main_package = next(
        path
        for path in packages.iterdir()
        if path.is_dir() and json.loads((path / "package.json").read_text())["name"] == NPM_PACKAGE
    )
    for package in [*platform_dirs, main_package]:
        version = json.loads((package / "package.json").read_text())["version"]
        package_name = json.loads((package / "package.json").read_text())["name"]
        existing = capture(
            ["npm", "view", f"{package_name}@{version}", "version", "--json"],
            check=False,
        )
        if existing and json.loads(existing) == version:
            print(f"npm 已存在 {package_name}@{version}，跳过")
            continue
        command = ["npm", "publish", str(package), "--access", "public"]
        if "-" in version:
            command.extend(["--tag", "native"])
        run(command)


def verify_npm(version: str) -> None:
    missing = []
    for attempt in range(NPM_VISIBILITY_ATTEMPTS):
        missing = missing_npm_versions(version)
        if not missing:
            break
        if attempt < NPM_VISIBILITY_ATTEMPTS - 1:
            print(
                f"npm 版本尚未全部可见，{NPM_VISIBILITY_DELAY_SECONDS} 秒后重试：{missing}"
            )
            time.sleep(NPM_VISIBILITY_DELAY_SECONDS)
    if missing:
        raise SystemExit(f"npm 版本尚未全部可见：{missing}")

    with tempfile.TemporaryDirectory(prefix="skz-npm-smoke-") as temp:
        install = [
            "npm",
            "install",
            "--prefix",
            temp,
            f"{NPM_PACKAGE}@{version}",
        ]
        result = None
        for attempt in range(6):
            result = subprocess.run(install, cwd=ROOT, capture_output=True, text=True)
            if result.returncode == 0:
                break
            if attempt < 5:
                time.sleep(10)
        if result is None or result.returncode != 0:
            detail = result.stderr.strip() if result else "npm install 未执行"
            raise SystemExit(f"npm 公共安装冒烟失败：{detail}")

        executable = Path(temp) / "node_modules" / ".bin" / "skz"
        installed = json.loads(capture([str(executable), "--version"]))
        if installed.get("cli") != version:
            raise SystemExit(f"npm 安装后的 CLI 版本不是 {version}：{installed}")
        manifest = (
            Path(temp)
            / "node_modules"
            / "@shengkezhi-com"
            / "skz-quant-cli"
            / "bin"
            / "plugins"
            / "manifest.json"
        )
        if json.loads(manifest.read_text()).get("cli") != version:
            raise SystemExit("npm 安装后的插件 manifest 版本不匹配")


def verify_sha256sums(path: Path) -> None:
    for number, line in enumerate(path.read_text().splitlines(), 1):
        digest, separator, filename = line.partition("  ")
        target = path.parent / filename
        if not separator or not target.is_file() or sha256(target) != digest:
            raise SystemExit(f"{path}:{number} 校验失败")


def publishable_files(assets: list[Path], packages: Path) -> list[Path]:
    return sorted([*assets, *(path for path in packages.rglob("*") if path.is_file())])


def write_release_state(
    output: Path, version: str, assets: list[Path], packages: Path
) -> Path:
    files = publishable_files(assets, packages)
    state = {
        "schema": 1,
        "version": version,
        "commit": capture(["git", "rev-parse", "HEAD"]),
        "files": {
            path.relative_to(output).as_posix(): sha256(path) for path in files
        },
    }
    path = output / RELEASE_STATE
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(path)
    return path


def load_release_state(output: Path, version: str) -> tuple[list[Path], Path]:
    path = output / RELEASE_STATE
    if not path.is_file():
        raise SystemExit(
            f"缺少可恢复发布状态 {path}；请恢复 Actions 的 release-payload artifact"
        )
    try:
        state = json.loads(path.read_text())
    except (json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"无法读取发布状态 {path}：{error}") from error
    expected_commit = capture(["git", "rev-parse", "HEAD"])
    failures = []
    if state.get("schema") != 1:
        failures.append(f"schema 是 {state.get('schema')}，预期 1")
    if state.get("version") != version:
        failures.append(f"版本是 {state.get('version')}，预期 {version}")
    if state.get("commit") != expected_commit:
        failures.append(f"commit 是 {state.get('commit')}，预期 {expected_commit}")
    files = state.get("files")
    if not isinstance(files, dict) or not files:
        failures.append("files 为空或格式错误")
        files = {}
    for relative, expected_digest in files.items():
        relative_path = Path(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts:
            failures.append(f"非法路径 {relative}")
            continue
        target = output / relative_path
        if not target.is_file():
            failures.append(f"缺文件 {relative}")
        elif sha256(target) != expected_digest:
            failures.append(f"SHA256 不匹配 {relative}")
    actual_files = {
        path.relative_to(output).as_posix()
        for directory in (output / "github-release", output / "npm")
        if directory.is_dir()
        for path in directory.rglob("*")
        if path.is_file()
    }
    recorded_files = set(files)
    for relative in sorted(actual_files - recorded_files):
        failures.append(f"存在未锁定文件 {relative}")
    for relative in sorted(recorded_files - actual_files):
        if not any(item == f"缺文件 {relative}" for item in failures):
            failures.append(f"缺文件 {relative}")
    if failures:
        raise SystemExit(
            "发布状态不可复用：\n  - "
            + "\n  - ".join(failures)
            + "\n请恢复 Actions 的原始 release-payload artifact，不要覆盖已发布产物"
        )

    assets = sorted(
        output / relative
        for relative in files
        if relative.startswith("github-release/")
    )
    packages = output / "npm"
    if not assets or not packages.is_dir():
        raise SystemExit("发布状态缺少 GitHub assets 或 npm 包目录")
    verify_sha256sums(output / "github-release" / "SHA256SUMS")
    print(f"复用已锁定发布产物：{path}")
    return assets, packages


def remote_release_assets(tag: str) -> dict[str, str | None]:
    release = json.loads(
        capture(
            [
                "gh",
                "release",
                "view",
                tag,
                "-R",
                HOMEPAGE_REPO,
                "--json",
                "assets",
            ]
        )
    )
    return {item["name"]: item.get("digest") for item in release["assets"]}


def verify_remote_release_assets(tag: str, assets: list[Path]) -> None:
    remote = remote_release_assets(tag)
    local = {path.name: f"sha256:{sha256(path)}" for path in assets}
    if remote != local:
        raise SystemExit(
            "已有 GitHub Release assets 与锁定产物不一致；"
            "拒绝覆盖已发布产物，请检查原始 release-payload artifact"
        )


def publish_github_release(tag: str, assets: list[Path]) -> None:
    exists = (
        subprocess.run(
            ["gh", "release", "view", tag, "-R", HOMEPAGE_REPO],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    )
    # GitHub 不接受中文资产文件名，用 label 显示中文，保留升级协议中的文件名。
    asset_args = [
        f"{path}#{path.name.replace('SKZ-', '胜可知-', 1)}"
        if path.name.startswith("SKZ-WorkBuddy-Setup-") else str(path)
        for path in assets
    ]
    if exists:
        remote = remote_release_assets(tag)
        expected = {path.name: f"sha256:{sha256(path)}" for path in assets}
        if any(expected.get(name) != digest for name, digest in remote.items()):
            raise SystemExit("已有 GitHub Release assets 与锁定产物不一致，拒绝覆盖")
        missing = [arg for path, arg in zip(assets, asset_args) if path.name not in remote]
        if missing:
            run(["gh", "release", "upload", tag, *missing, "-R", HOMEPAGE_REPO])
        verify_remote_release_assets(tag, assets)
        print(f"GitHub Release {tag} assets 与锁定产物一致")
    else:
        run(
            [
                "gh",
                "release",
                "create",
                tag,
                *asset_args,
                "-R",
                HOMEPAGE_REPO,
                "--verify-tag",
                "--generate-notes",
                "--title",
                tag,
            ]
        )
        verify_remote_release_assets(tag, assets)


def github_file(repo: str, path: str) -> str:
    encoded = capture(
        ["gh", "api", f"repos/{repo}/contents/{path}", "--jq", ".content"]
    )
    return base64.b64decode(encoded.replace("\n", "")).decode()


def verify_publication(version: str, tag: str, assets: list[Path], remote: str) -> None:
    verify_npm(version)
    verify_remote_release_assets(tag, assets)
    remote_tag = capture(
        ["git", "ls-remote", "--tags", remote, f"refs/tags/{tag}^{{}}"], check=False
    )
    if not remote_tag.startswith(capture(["git", "rev-parse", "HEAD"])):
        raise SystemExit(f"远端 {tag} 未指向当前 HEAD")

    release = json.loads(
        capture(
            [
                "gh",
                "release",
                "view",
                tag,
                "-R",
                HOMEPAGE_REPO,
                "--json",
                "tagName,assets,url",
            ]
        )
    )
    actual_assets = {item["name"] for item in release["assets"]}
    expected_assets = {path.name for path in assets}
    if actual_assets != expected_assets:
        raise SystemExit(
            f"GitHub Release assets 不一致：缺 {sorted(expected_assets - actual_assets)}，"
            f"多 {sorted(actual_assets - expected_assets)}"
        )

    formula = github_file(HOMEPAGE_REPO, "Formula/skz.rb")
    manifest = github_file(HOMEPAGE_REPO, "bucket/skz.json")
    sums = next(path for path in assets if path.name == "SHA256SUMS")
    hashes = required_asset_hashes(parse_sha256sums(sums))
    if (
        formula != render_formula(version, hashes, HOMEPAGE_REPO)
        or manifest != render_scoop_manifest(version, hashes, HOMEPAGE_REPO)
    ):
        raise SystemExit("Homebrew Formula 或 Scoop manifest 版本、URL 或 SHA256 未同步")

    probe_name = "skz-x86_64-unknown-linux-musl.tar.gz"
    request = urllib.request.Request(
        f"https://github.com/{HOMEPAGE_REPO}/releases/download/{tag}/{probe_name}",
        method="HEAD",
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        if response.status != 200:
            raise SystemExit(f"匿名下载 {probe_name} 返回 HTTP {response.status}")
    print(
        f"发布核验通过：{release['url']}，{len(actual_assets)} 个 assets，npm/Homebrew/Scoop 均为 {version}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--remote", default="origin")
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="只做预检和 bump dry-run，不修改工作树、不 bump、不 push、不发布",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="继续推送当前 HEAD 已准备好的版本 tag，不再次 bump",
    )
    args = parser.parse_args()
    preflight(args.remote, resume=args.resume)
    if args.check_only:
        if args.resume:
            version = cargo_field("version")
            validate_prepared_release(version, f"v{version}")
        else:
            run(
                [
                    "uvx",
                    "--from",
                    "commitizen",
                    "cz",
                    "bump",
                    "--increment",
                    "PATCH",
                    "--dry-run",
                    "--yes",
                ]
            )
        return

    os.environ["RUSTC_WRAPPER"] = ""
    version, tag = prepare_release(resume=args.resume)
    if not args.resume:
        run([sys.executable, "scripts/release/build_plugins.py", "--sync-only"])
        run(["git", "add", "plugins"])
        run(["git", "commit", "--amend", "--no-edit"])
        run(["git", "tag", "-f", "-a", tag, "-m", tag])
    validate_prepared_release(version, tag)
    run([sys.executable, "scripts/release/build_plugins.py", "--check"])

    run(
        [
            "git",
            "push",
            "--atomic",
            args.remote,
            f"HEAD:refs/heads/{RELEASE_BRANCH}",
            f"refs/tags/{tag}",
        ]
    )
    print(f"已推送 {tag}；构建和发布进度：https://github.com/{HOMEPAGE_REPO}/actions/workflows/release.yml")


if __name__ == "__main__":
    main()
