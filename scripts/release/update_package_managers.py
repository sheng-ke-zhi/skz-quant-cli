#!/usr/bin/env python3
"""根据主仓库 Release 的 SHA256SUMS 更新 Homebrew 与 Scoop metadata。"""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

from common import cargo_field, require_tool

HOMEPAGE_REPO = "sheng-ke-zhi/skz-quant-cli"

# target -> release 资产文件名，对应 release_wsl.py 的标准库归档规则。
DARWIN_LINUX_TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
)
WINDOWS_TARGET = "x86_64-pc-windows-gnu"


def parse_sha256sums(path: Path) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        # sha256sum 输出：`<hash>  <filename>`（两个空格，或末尾带 `*` 表示二进制模式）。
        digest, _, filename = line.partition("  ")
        filename = filename.lstrip("*")
        if not digest or not filename:
            raise SystemExit(f"{path} 里有一行解析不出 hash/文件名：{line!r}")
        hashes[filename] = digest
    return hashes


def required_asset_hashes(hashes: dict[str, str]) -> dict[str, str]:
    needed = {f"skz-{target}.tar.gz" for target in DARWIN_LINUX_TARGETS}
    needed.add(f"skz-{WINDOWS_TARGET}.zip")
    missing = sorted(needed - hashes.keys())
    if missing:
        raise SystemExit(
            "SHA256SUMS 里缺以下发布资产，无法渲染 Formula/manifest：\n- "
            + "\n- ".join(missing)
        )
    return {name: hashes[name] for name in needed}


def render_formula(version: str, hashes: dict[str, str], download_repo: str) -> str:
    def asset(target: str) -> tuple[str, str]:
        filename = f"skz-{target}.tar.gz"
        url = f"https://github.com/{download_repo}/releases/download/v{version}/{filename}"
        return url, hashes[filename]

    darwin_arm_url, darwin_arm_sha = asset("aarch64-apple-darwin")
    darwin_intel_url, darwin_intel_sha = asset("x86_64-apple-darwin")
    linux_arm_url, linux_arm_sha = asset("aarch64-unknown-linux-musl")
    linux_intel_url, linux_intel_sha = asset("x86_64-unknown-linux-musl")

    return f'''class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/{HOMEPAGE_REPO}"
  version "{version}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "{darwin_arm_url}"
      sha256 "{darwin_arm_sha}"
    end
    on_intel do
      url "{darwin_intel_url}"
      sha256 "{darwin_intel_sha}"
    end
  end

  on_linux do
    on_arm do
      url "{linux_arm_url}"
      sha256 "{linux_arm_sha}"
    end
    on_intel do
      url "{linux_intel_url}"
      sha256 "{linux_intel_sha}"
    end
  end

  def install
    libexec.install "skz", "plugins"
    bin.install_symlink libexec/"skz"
  end

  test do
    system "#{{bin}}/skz", "--version"
  end
end
'''


def render_scoop_manifest(
    version: str, hashes: dict[str, str], download_repo: str
) -> str:
    filename = f"skz-{WINDOWS_TARGET}.zip"
    url = f"https://github.com/{download_repo}/releases/download/v{version}/{filename}"
    sha = hashes[filename]
    return f'''{{
    "version": "{version}",
    "description": "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具",
    "homepage": "https://github.com/{HOMEPAGE_REPO}",
    "license": "Apache-2.0",
    "url": "{url}",
    "hash": "sha256:{sha}",
    "bin": "skz.exe",
    "checkver": {{
        "url": "https://github.com/{download_repo}/releases/latest",
        "regex": "/releases/tag/v([\\\\d.]+)"
    }},
    "autoupdate": {{
        "url": "https://github.com/{download_repo}/releases/download/v$version/skz-{WINDOWS_TARGET}.zip"
    }}
}}
'''


def sync_package_managers(
    sha256sums_path: Path,
    *,
    version: str,
    download_repo: str = HOMEPAGE_REPO,
    dry_run: bool,
) -> None:
    hashes = required_asset_hashes(parse_sha256sums(sha256sums_path.resolve()))
    files = {
        "Formula/skz.rb": render_formula(version, hashes, download_repo),
        "bucket/skz.json": render_scoop_manifest(version, hashes, download_repo),
    }
    if dry_run:
        for path, content in files.items():
            print(f"# --dry-run，跳过推送 {HOMEPAGE_REPO}:{path}\n{content}")
        return

    require_tool("gh", "安装 GitHub CLI 并执行 gh auth login")
    require_tool("git", "安装 Git")

    with tempfile.TemporaryDirectory(prefix="skz-tap-") as tmp:
        clone_dir = Path(tmp) / "repo"
        clone = subprocess.run(
            ["gh", "repo", "clone", HOMEPAGE_REPO, str(clone_dir), "--", "--depth", "1", "--branch", "main"],
            capture_output=True,
            text=True,
        )
        if clone.returncode != 0:
            raise SystemExit(
                f"clone {HOMEPAGE_REPO} 失败：\n{clone.stderr.strip()}"
            )

        bucket = clone_dir / "bucket/skz.json"
        if bucket.is_file():
            previous = json.loads(bucket.read_text())["version"]
            if tuple(map(int, previous.split("."))) > tuple(map(int, version.split("."))):
                raise SystemExit(f"main 已发布 {previous}，拒绝回退到 {version}")
        changed = []
        for relative_path, content in files.items():
            target_file = clone_dir / relative_path
            if target_file.is_file() and target_file.read_text() == content:
                continue
            target_file.parent.mkdir(parents=True, exist_ok=True)
            target_file.write_text(content)
            changed.append(relative_path)
        if not changed:
            print("Homebrew/Scoop 内容未变化，跳过")
            return

        subprocess.run(["git", "add", *changed], cwd=clone_dir, check=True)
        commit = subprocess.run(
            ["git", "commit", "-m", f"chore(packages): skz v{version}"],
            cwd=clone_dir,
            capture_output=True,
            text=True,
        )
        if commit.returncode != 0:
            raise SystemExit(
                f"commit Homebrew/Scoop 失败：\n{commit.stderr.strip()}"
            )
        subprocess.run(["git", "push"], cwd=clone_dir, check=True)
        print(f"已一次提交更新 {HOMEPAGE_REPO} 的 Homebrew/Scoop")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--sha256sums",
        type=Path,
        required=True,
        help="release_wsl.py 产出的 SHA256SUMS",
    )
    parser.add_argument("--download-repo", default=HOMEPAGE_REPO)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    sync_package_managers(
        args.sha256sums,
        version=cargo_field("version"),
        download_repo=args.download_repo,
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    main()
