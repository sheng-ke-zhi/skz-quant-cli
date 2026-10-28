"""构建一个 target，并生成统一目录下的二进制和校验记录。"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

from common import ROOT, require_tool, run, sha256


def _uses_zig(target: str) -> bool:
    native_linux_arm = (
        platform.system() == "Linux"
        and platform.machine().lower() in {"aarch64", "arm64"}
    )
    return (target == "aarch64-unknown-linux-musl" and not native_linux_arm) or (
        target.endswith("-apple-darwin") and platform.system() != "Darwin"
    )


def _cargo_command(target: str) -> list[str]:
    base = ["--locked", "--profile", "dist", "--target", target, "--bin", "skz"]
    if _uses_zig(target):
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


def macos_sdk_root(target: str) -> str | None:
    if not target.endswith("-apple-darwin") or platform.system() == "Darwin":
        return None
    value = os.environ.get("SDKROOT")
    if value:
        sdk = Path(value).expanduser().resolve()
        frameworks = sdk / "System/Library/Frameworks"
        required = [
            frameworks / f"{name}.framework" / f"{name}.tbd"
            for name in ("Security", "CoreFoundation")
        ]
        required.append(sdk / "usr/lib/libSystem.tbd")
        if all(path.is_file() for path in required):
            return str(sdk)
    raise SystemExit(
        f"{target} 交叉构建需要完整的 macOS SDK。请将 SDKROOT 设为 MacOSX*.sdk 目录，"
        "其中须包含 Security、CoreFoundation 和 libSystem 链接库；"
        "配置步骤见 docs/releasing.md 的‘WSL 本地构建’。"
        "仅构建 Linux/Windows 时用 --target 显式选择目标。"
    )


def build_target(target: str, output: Path) -> dict[str, str]:
    sdk = macos_sdk_root(target)
    require_tool("cargo", "通过 rustup 安装 Rust")
    _ensure_rust_target(target)
    env = os.environ.copy()
    if sdk:
        env["SDKROOT"] = sdk
    if _uses_zig(target):
        zig = shutil.which("zig") or shutil.which("python-zig")
        if not zig:
            raise SystemExit("缺少 Zig。安装方式：uv tool install ziglang")
        env["CARGO_ZIGBUILD_ZIG_PATH"] = zig
    elif target.endswith("-unknown-linux-musl"):
        env[f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER"] = require_tool(
            "musl-gcc", "sudo apt-get install musl-tools"
        )
    elif target == "x86_64-pc-windows-gnu":
        compiler = "gcc" if platform.system() == "Windows" else "x86_64-w64-mingw32-gcc"
        env["CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER"] = require_tool(
            compiler, "安装 MinGW-w64 并加入 PATH"
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
        "binary": binary.relative_to(output).as_posix(),
        "binary_sha256": sha256(binary),
    }
