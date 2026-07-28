#!/usr/bin/env python3
"""在 macOS 构建 arm64/x64 发布产物。"""

import argparse
import platform
from pathlib import Path

from build_target import build_target
from common import DEFAULT_OUTPUT, write_manifest

TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", action="append", choices=TARGETS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        raise SystemExit("build_macos.py 只能在 macOS 中运行")
    targets = args.target or TARGETS
    records = [build_target(target, args.output.resolve()) for target in targets]
    manifest = write_manifest("macos", records, args.output.resolve())
    print(f"macOS 构建完成：{manifest}")


if __name__ == "__main__":
    main()
