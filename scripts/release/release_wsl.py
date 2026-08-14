#!/usr/bin/env python3
"""供维护者在 WSL 中发布 skz-quant-cli 到 npm、GitHub Release、Homebrew 和 Scoop。"""

from __future__ import annotations

import argparse
import base64
import json
import os
import platform
import shutil
import stat
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from datetime import datetime, timezone
from pathlib import Path

from common import DEFAULT_OUTPUT, ROOT, cargo_field, run, sha256
from build_plugins import build_bundle
from update_package_managers import HOMEPAGE_REPO, sync_package_managers

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
        "uv": "安装 uv",
        "uvx": "安装 uv",
        "cargo": "通过 rustup 安装 Rust",
        "rustup": "通过 rustup 安装 Rust",
        "cargo-zigbuild": "cargo install cargo-zigbuild --locked",
        "gh": "安装 GitHub CLI",
        "node": "安装 Node.js 18 或更高版本",
        "npm": "安装 npm 并执行 npm login",
        "musl-gcc": "sudo apt-get install musl-tools",
        "x86_64-w64-mingw32-gcc": "sudo apt-get install gcc-mingw-w64-x86-64",
    }
    for tool, hint in required.items():
        if not command_available(tool):
            failures.append(f"缺 {tool}；{hint}")
    if not (command_available("zig") or command_available("python-zig")):
        failures.append("缺 Zig；执行 uv tool install ziglang")
    if "microsoft" not in platform.release().lower():
        failures.append("完整五平台发布只支持 WSL")

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

    if command_available("gh"):
        auth = subprocess.run(
            ["gh", "auth", "status"], cwd=ROOT, capture_output=True, text=True
        )
        if auth.returncode != 0:
            failures.append("gh 尚未登录或凭据失效；执行 gh auth login")
    if command_available("npm"):
        auth = subprocess.run(
            ["npm", "whoami"], cwd=ROOT, capture_output=True, text=True
        )
        if auth.returncode != 0:
            failures.append("npm 尚未登录或凭据失效；执行 npm login")

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
        "WSL 五平台工具链和 GitHub 凭据均就绪"
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


def build(output: Path) -> None:
    if "microsoft" not in platform.release().lower():
        raise SystemExit("完整五平台发布只支持 WSL")
    run([sys.executable, "scripts/release/build_wsl.py", "--output", str(output)])


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
        for path in sorted((output / "plugins").rglob("*")):
            if path.is_file():
                info = zipfile.ZipInfo((Path("plugins") / path.relative_to(output / "plugins")).as_posix(), zip_time)
                info.external_attr = (stat.S_IFREG | (path.stat().st_mode & 0o777)) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                bundle.writestr(info, path.read_bytes())

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


def npm_versions(version: str) -> list[str]:
    platforms = ("darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64")
    return [f"{version}-{platform}" for platform in platforms] + [version]


def publish_npm(packages: Path) -> None:
    platform_dirs = sorted(path for path in packages.iterdir() if path.name != "skz-quant-cli")
    for package in [*platform_dirs, packages / "skz-quant-cli"]:
        run(["npm", "publish", str(package), "--access", "public"])


def verify_npm(version: str) -> None:
    missing = []
    for expected in npm_versions(version):
        actual = capture(
            ["npm", "view", f"skz-quant-cli@{expected}", "version", "--json"],
            check=False,
        )
        try:
            found = json.loads(actual)
        except json.JSONDecodeError:
            found = None
        if found != expected:
            missing.append(expected)
    if missing:
        raise SystemExit(f"npm 版本尚未全部可见：{missing}")


def verify_sha256sums(path: Path) -> None:
    for number, line in enumerate(path.read_text().splitlines(), 1):
        digest, separator, filename = line.partition("  ")
        target = path.parent / filename
        if not separator or not target.is_file() or sha256(target) != digest:
            raise SystemExit(f"{path}:{number} 校验失败")


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
    asset_args = [str(path) for path in assets]
    if exists:
        run(
            [
                "gh",
                "release",
                "upload",
                tag,
                *asset_args,
                "-R",
                HOMEPAGE_REPO,
                "--clobber",
            ]
        )
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


def github_file(repo: str, path: str) -> str:
    encoded = capture(
        ["gh", "api", f"repos/{repo}/contents/{path}", "--jq", ".content"]
    )
    return base64.b64decode(encoded.replace("\n", "")).decode()


def verify_publication(version: str, tag: str, assets: list[Path], remote: str) -> None:
    verify_npm(version)
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

    formula = github_file("sheng-ke-zhi/homebrew-tap", "Formula/skz.rb")
    manifest = github_file("sheng-ke-zhi/scoop-bucket", "bucket/skz.json")
    if (
        f'version "{version}"' not in formula
        or json.loads(manifest).get("version") != version
    ):
        raise SystemExit("Homebrew Formula 或 Scoop manifest 版本未同步")

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
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="只做预检和 bump dry-run，不修改工作树、不 bump、不 push、不发布",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="继续发布当前 HEAD 已存在的版本 tag，不再次 bump",
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
    validate_prepared_release(version, tag)
    run(["cargo", "fmt", "--all", "--", "--check"])
    run([sys.executable, "tests/plugins/test_plugin_bundle.py", "-v"])
    run(["cargo", "test", "--locked"])
    run(["cargo", "clippy", "--all-targets", "--locked", "--", "-D", "warnings"])

    output = args.output.resolve()
    build(output)
    build_bundle(output / "plugins")
    validate_artifacts(output, version)
    npm_packages = prepare_npm_packages(output)
    assets = prepare_release_assets(output, version)
    checksum = next(path for path in assets if path.name == "SHA256SUMS")
    sync_package_managers(
        checksum, version=version, download_repo=HOMEPAGE_REPO, dry_run=True
    )

    run(
        [
            "git",
            "push",
            args.remote,
            f"HEAD:refs/heads/{RELEASE_BRANCH}",
            f"refs/tags/{tag}",
        ]
    )
    publish_github_release(tag, assets)
    publish_npm(npm_packages)
    sync_package_managers(
        checksum, version=version, download_repo=HOMEPAGE_REPO, dry_run=False
    )
    verify_publication(version, tag, assets, args.remote)


if __name__ == "__main__":
    main()
