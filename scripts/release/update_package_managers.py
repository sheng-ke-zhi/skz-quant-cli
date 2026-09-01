#!/usr/bin/env python3
"""根据主仓库 Release 的 SHA256SUMS 更新 Homebrew 与 Scoop metadata。"""

from __future__ import annotations

import argparse
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


def push_repo(
    repo: str, relative_path: str, content: str, message: str, *, dry_run: bool
) -> None:
    if dry_run:
        print(f"# --dry-run，跳过推送 {repo}:{relative_path}\n{content}")
        return

    require_tool("gh", "安装 GitHub CLI 并执行 gh auth login")
    require_tool("git", "安装 Git")

    with tempfile.TemporaryDirectory(prefix="skz-tap-") as tmp:
        clone_dir = Path(tmp) / "repo"
        clone = subprocess.run(
            ["gh", "repo", "clone", repo, str(clone_dir), "--", "--depth", "1"],
            capture_output=True,
            text=True,
        )
        if clone.returncode != 0:
            raise SystemExit(
                f"clone {repo} 失败（仓库不存在或没权限？）：\n{clone.stderr.strip()}"
            )

        target_file = clone_dir / relative_path
        target_file.parent.mkdir(parents=True, exist_ok=True)
        if target_file.is_file() and target_file.read_text() == content:
            print(f"{repo}:{relative_path} 内容未变化，跳过")
            return
        target_file.write_text(content)

        subprocess.run(["git", "add", relative_path], cwd=clone_dir, check=True)
        commit = subprocess.run(
            ["git", "commit", "-m", message],
            cwd=clone_dir,
            capture_output=True,
            text=True,
        )
        if commit.returncode != 0:
            raise SystemExit(
                f"commit {repo}:{relative_path} 失败：\n{commit.stderr.strip()}"
            )
        subprocess.run(["git", "push"], cwd=clone_dir, check=True)
        print(f"已推送 {repo}:{relative_path}")


def sync_package_managers(
    sha256sums_path: Path,
    *,
    version: str,
    download_repo: str = HOMEPAGE_REPO,
    homebrew_tap: str = "sheng-ke-zhi/homebrew-tap",
    scoop_bucket: str = "sheng-ke-zhi/scoop-bucket",
    dry_run: bool,
) -> None:
    sha256sums_path = sha256sums_path.resolve()
    if not sha256sums_path.is_file():
        raise SystemExit(f"找不到 {sha256sums_path}")
    hashes = required_asset_hashes(parse_sha256sums(sha256sums_path))
    message = f"skz v{version}"
    push_repo(
        homebrew_tap,
        "Formula/skz.rb",
        render_formula(version, hashes, download_repo),
        message,
        dry_run=dry_run,
    )
    push_repo(
        scoop_bucket,
        "bucket/skz.json",
        render_scoop_manifest(version, hashes, download_repo),
        message,
        dry_run=dry_run,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--sha256sums",
        type=Path,
        required=True,
        help="release_wsl.py 产出的 SHA256SUMS",
    )
    parser.add_argument("--download-repo", default=HOMEPAGE_REPO)
    parser.add_argument("--homebrew-tap", default="sheng-ke-zhi/homebrew-tap")
    parser.add_argument("--scoop-bucket", default="sheng-ke-zhi/scoop-bucket")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    sync_package_managers(
        args.sha256sums,
        version=cargo_field("version"),
        download_repo=args.download_repo,
        homebrew_tap=args.homebrew_tap,
        scoop_bucket=args.scoop_bucket,
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    main()
