"""构建一个 target，并生成统一目录下的二进制和校验记录。"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

from common import ROOT, require_tool, run, sha256


def _cargo_command(target: str) -> list[str]:
    base = ["--locked", "--profile", "dist", "--target", target, "--bin", "skz"]
    zig_cross = target == "aarch64-unknown-linux-musl" or (
        target.endswith("-apple-darwin") and platform.system() != "Darwin"
    )
    if zig_cross:
        require_tool("cargo-zigbuild", "cargo install cargo-zigbuild --locked")
        return ["cargo", "zigbuild", *base]
    return ["cargo", "build", *base]


def _ensure_rust_target(target: str) -> None:
    require_tool("rustup", "通过 rustup 安装 Rust")
    installed = subprocess.run(
        ["rustup", "target", "list", "--installed"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.splitlines()
    if target not in installed:
        run(["rustup", "target", "add", target])


def build_target(target: str, output: Path) -> dict[str, str]:
    require_tool("cargo", "通过 rustup 安装 Rust")
    _ensure_rust_target(target)
    env = os.environ.copy()
    zig_cross = target == "aarch64-unknown-linux-musl" or (
        target.endswith("-apple-darwin") and platform.system() != "Darwin"
    )
    if zig_cross:
        zig = shutil.which("zig") or shutil.which("python-zig")
        if not zig:
            raise SystemExit("缺少 Zig。安装方式：uv tool install ziglang")
        env["CARGO_ZIGBUILD_ZIG_PATH"] = zig
    elif target == "x86_64-unknown-linux-musl":
        env["CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"] = require_tool(
            "musl-gcc", "sudo apt-get install musl-tools"
        )
    elif target == "x86_64-pc-windows-gnu":
        env["CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER"] = require_tool(
            "x86_64-w64-mingw32-gcc", "sudo apt-get install mingw-w64"
        )

    run(_cargo_command(target), env=env)
    filename = "skz.exe" if "windows" in target else "skz"
    source = ROOT / "target" / target / "dist" / filename
    if not source.is_file():
        raise SystemExit(f"构建成功但找不到产物：{source}")

    binary_dir = output / "binaries" / target
    binary_dir.mkdir(parents=True, exist_ok=True)
    binary = binary_dir / filename
    shutil.copy2(source, binary)
    return {
        "target": target,
        "binary": str(binary.relative_to(output)),
        "binary_sha256": sha256(binary),
    }
