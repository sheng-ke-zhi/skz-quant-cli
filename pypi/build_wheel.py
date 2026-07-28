#!/usr/bin/env python3
"""手搓 wheel：把 cargo dist 出的 skz 二进制塞进 wheel 的 .data/scripts/。

不用 maturin/setuptools —— pip 装 wheel 时会把 .data/scripts/ 下的文件原样拷到
venv 的 bin/ 并保留可执行位，这是 ruff/uv 这类 Rust CLI 分发到 PyPI 的机制，
不需要任何 Python 胶水代码。只依赖标准库 + 外部的 cargo/uv 命令。

既可以单独按 target 构建，也可以由 WSL 一键发布入口统一调度。脚本本身只构建 wheel，
不持有发布凭据：
    brew install mingw-w64
    brew tap messense/macos-cross-toolchains
    brew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl
    brew install messense/macos-cross-toolchains/aarch64-unknown-linux-musl
    rustup target add x86_64-apple-darwin x86_64-pc-windows-gnu \
        x86_64-unknown-linux-musl aarch64-unknown-linux-musl

交叉链接器路径通过 subprocess 的 env 注入（CARGO_TARGET_*_LINKER），不写进
.cargo/config.toml —— 那是仓库共享文件，不同宿主需要的 linker 名称并不相同；env
只在本脚本的子进程里生效，不会污染其他构建路径。

Linux 两个 target 各自的 wheel 同时打 manylinux + musllinux 两种 tag：musl 静态
二进制（rustup 的 *-unknown-linux-musl 默认就是 +crt-static）零动态 libc 依赖，
两边的承诺都满足，不用分别编译两份。wheel 文件名和 WHEEL 元数据的 Tag 字段都要
把这些 tag 列全，不是只挑一个（参考 ruff 在 PyPI 上发的 wheel 命名）。

PyPI 上传由 `scripts/release/release_wsl.py` 使用临时子进程环境中的 token 完成。
"""

import argparse
import hashlib
import os
import re
import stat
import subprocess
import zipfile
from base64 import urlsafe_b64encode
from pathlib import Path
from typing import NamedTuple, Optional

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
BIN_NAME = "skz"


class Target(NamedTuple):
    triple: str
    wheel_tags: tuple  # 一个二进制可能同时满足多个 wheel 平台 tag，见上面 docstring
    windows: bool = False
    linker: Optional[str] = None  # None = 原生 target，不需要交叉链接器


TARGETS = [
    Target("aarch64-apple-darwin", ("macosx_11_0_arm64",)),
    Target("x86_64-apple-darwin", ("macosx_10_12_x86_64",)),
    Target(
        "x86_64-pc-windows-gnu",
        ("win_amd64",),
        windows=True,
        linker="x86_64-w64-mingw32-gcc",
    ),
    Target(
        "x86_64-unknown-linux-musl",
        ("manylinux_2_17_x86_64", "musllinux_1_1_x86_64"),
        linker="x86_64-unknown-linux-musl-gcc",
    ),
    Target(
        "aarch64-unknown-linux-musl",
        ("manylinux_2_17_aarch64", "musllinux_1_1_aarch64"),
        linker="aarch64-unknown-linux-musl-gcc",
    ),
]
TARGETS_BY_TRIPLE = {t.triple: t for t in TARGETS}


def cargo_toml_field(field: str) -> str:
    text = CARGO_TOML.read_text()
    # 只在 [package] 段内找（截到下一个 section 之前），避免碰到别处的同名字段。
    package_section = text.split("\n[", 1)[0]
    m = re.search(rf'^{field}\s*=\s*"([^"]*)"', package_section, re.MULTILINE)
    if not m:
        raise SystemExit(f"Cargo.toml 里没找到 [package].{field}")
    return m.group(1)


def project_version() -> str:
    # 版本号单一来源 = Cargo.toml；直接复用本文件已有的 [package] 定位逻辑，避免 CI
    # 为读一个字段额外联网安装 commitizen。
    return cargo_toml_field("version")


def normalize(name: str) -> str:
    # PEP 503 归一化：连续的 -_. 变成单个 _。
    return re.sub(r"[-_.]+", "_", name).lower()


def record_line(path: str, data: bytes) -> str:
    digest = urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
    return f"{path},sha256={digest},{len(data)}"


def build_one(
    target: Target,
    *,
    name: str,
    description: str,
    license_expr: str,
    version: str,
    dist_name: str,
    dist_dir: Path,
) -> Path:
    env = os.environ.copy()
    if target.linker:
        env_key = "CARGO_TARGET_" + target.triple.upper().replace("-", "_") + "_LINKER"
        # CI 的原生 Linux runner 使用系统 `musl-gcc`；macOS 本地交叉编译仍使用
        # target-prefixed 工具链。显式环境变量优先，兼顾两条构建路径。
        env.setdefault(env_key, target.linker)

    subprocess.run(
        [
            "cargo",
            "build",
            "--profile",
            "dist",
            "--target",
            target.triple,
            "--bin",
            BIN_NAME,
        ],
        cwd=ROOT,
        check=True,
        env=env,
    )
    bin_filename = f"{BIN_NAME}.exe" if target.windows else BIN_NAME
    # 带 --target 时 cargo 把产物放到 target/<triple>/<profile>/ 下，不是 target/<profile>/。
    binary = ROOT / "target" / target.triple / "dist" / bin_filename
    if not binary.exists():
        raise SystemExit(f"没找到构建产物: {binary}")

    return package_one(
        target,
        binary=binary,
        name=name,
        description=description,
        license_expr=license_expr,
        version=version,
        dist_name=dist_name,
        dist_dir=dist_dir,
    )


def package_one(
    target: Target,
    *,
    binary: Path,
    name: str,
    description: str,
    license_expr: str,
    version: str,
    dist_name: str,
    dist_dir: Path,
) -> Path:
    """把已经构建好的单个平台二进制封装成 wheel，不重复调用 Cargo。"""
    tag = ".".join(target.wheel_tags)
    print(f"[build_wheel] package {name} {version} {target.triple} -> py3-none-{tag}")
    dist_dir.mkdir(parents=True, exist_ok=True)
    bin_filename = f"{BIN_NAME}.exe" if target.windows else BIN_NAME

    wheel_path = dist_dir / f"{dist_name}-{version}-py3-none-{tag}.whl"

    prefix = f"{dist_name}-{version}"
    metadata = (
        "Metadata-Version: 2.1\n"
        f"Name: {name}\n"
        f"Version: {version}\n"
        f"Summary: {description}\n"
        f"License: {license_expr}\n"
    ).encode()
    wheel_meta = (
        "Wheel-Version: 1.0\n"
        "Generator: skz-build-wheel\n"
        "Root-Is-Purelib: false\n"
        + "".join(f"Tag: py3-none-{t}\n" for t in target.wheel_tags)
    ).encode()
    binary_bytes = binary.read_bytes()

    scripts_path = f"{prefix}.data/scripts/{bin_filename}"
    entries = {
        scripts_path: binary_bytes,
        f"{prefix}.dist-info/METADATA": metadata,
        f"{prefix}.dist-info/WHEEL": wheel_meta,
    }

    record_lines = [record_line(path, data) for path, data in entries.items()]
    record_lines.append(f"{prefix}.dist-info/RECORD,,")
    record_bytes = ("\n".join(record_lines) + "\n").encode()

    if wheel_path.exists():
        wheel_path.unlink()

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for path, data in entries.items():
            zi = zipfile.ZipInfo(path)
            # .data/scripts/ 下的二进制要保留可执行位，否则 pip 装出来是打不开的普通文件
            # （Windows 的 zip 权限位没有意义，但写了也无害，判断用 path 而不是猜后缀）。
            # 必须带 stat.S_IFREG（普通文件类型位）——pip 判断该不该 chmod +x 用的是
            # zip_item_is_executable()，里面靠 stat.S_ISREG(mode) 先过滤一遍，只给权限位
            # （比如单独一个 0o755）这一位是空的，会被当成"不是普通文件"直接跳过 chmod。
            mode = stat.S_IFREG | (0o755 if path == scripts_path else 0o644)
            zi.external_attr = mode << 16
            zf.writestr(zi, data)
        zf.writestr(f"{prefix}.dist-info/RECORD", record_bytes)

    print(f"[build_wheel] wrote {wheel_path}")
    return wheel_path


def package_binary(triple: str, binary: Path, dist_dir: Path) -> Path:
    """供 WSL/macOS 构建脚本复用的预构建二进制打包入口。"""
    try:
        target = TARGETS_BY_TRIPLE[triple]
    except KeyError as exc:
        raise SystemExit(f"未知 target: {triple}") from exc
    return package_one(
        target,
        binary=binary,
        name=cargo_toml_field("name"),
        description=cargo_toml_field("description"),
        license_expr=cargo_toml_field("license"),
        version=project_version(),
        dist_name=normalize(cargo_toml_field("name")),
        dist_dir=dist_dir,
    )


def build(triples: Optional[list] = None) -> list:
    targets = TARGETS
    if triples:
        unknown = set(triples) - TARGETS_BY_TRIPLE.keys()
        if unknown:
            raise SystemExit(f"未知 target: {', '.join(sorted(unknown))}")
        targets = [TARGETS_BY_TRIPLE[t] for t in triples]

    name = cargo_toml_field("name")
    description = cargo_toml_field("description")
    license_expr = cargo_toml_field("license")
    version = project_version()  # 只调一次：每个 target 都是同一个版本号
    dist_name = normalize(name)

    dist_dir = ROOT / "dist"
    dist_dir.mkdir(exist_ok=True)

    return [
        build_one(
            t,
            name=name,
            description=description,
            license_expr=license_expr,
            version=version,
            dist_name=dist_name,
            dist_dir=dist_dir,
        )
        for t in targets
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--target",
        action="append",
        dest="triples",
        metavar="TRIPLE",
        help="只构建指定 target（可重复传）；默认构建全部 target。可选："
        + ", ".join(t.triple for t in TARGETS),
    )
    args = parser.parse_args()
    build(args.triples)


if __name__ == "__main__":
    main()
