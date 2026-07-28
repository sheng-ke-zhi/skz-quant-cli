#!/usr/bin/env python3
"""在 WSL 构建 macOS、Linux 和 Windows 五个平台发布产物。"""

import argparse
import platform
from pathlib import Path

from build_target import build_target
from common import DEFAULT_OUTPUT, write_manifest

TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", action="append", choices=TARGETS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    if "microsoft" not in platform.release().lower():
        raise SystemExit("build_wsl.py 只能在 WSL 中运行")
    targets = args.target or TARGETS
    records = [build_target(target, args.output.resolve()) for target in targets]
    manifest = write_manifest("wsl", records, args.output.resolve())
    print(f"WSL 构建完成：{manifest}")


if __name__ == "__main__":
    main()
