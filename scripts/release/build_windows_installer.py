"""Build the single-file WorkBuddy launcher from the existing Windows artifact."""
from __future__ import annotations

import subprocess
import shutil
import argparse
import json
from pathlib import Path

from common import DEFAULT_OUTPUT, ROOT, cargo_field, require_tool, sha256
from build_plugins import build_bundle
from build_target import build_target


def build_installer(output_dir: Path, version: str) -> Path:
    output_dir = output_dir.resolve()
    compiler = require_tool("makensis", "install NSIS on the build machine")
    binary = output_dir / "binaries/x86_64-pc-windows-gnu/skz.exe"
    bundle = output_dir / "plugins"
    if json.loads((bundle / "manifest.json").read_text())["cli"] != version:
        raise SystemExit("Windows 安装包版本与技能 bundle 不一致")
    payload = output_dir / "windows-installer/payload"
    shutil.rmtree(payload, ignore_errors=True)
    payload.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, payload / "skz.exe")
    shutil.copy2(ROOT / "LICENSE", payload / "LICENSE")
    shutil.copytree(bundle, payload / "plugins")
    # Windows PowerShell 5.1 needs a BOM for Chinese messages.
    for name in ("setup.ps1", "update.ps1"):
        (payload / name).write_text((ROOT / "scripts/windows" / name).read_text(encoding="utf-8-sig"), encoding="utf-8-sig")
    files = [{"path":path.relative_to(payload).as_posix(), "sha256":sha256(path)}
             for path in sorted(payload.rglob("*")) if path.is_file()]
    (payload / "installer-manifest.json").write_text(json.dumps({"cli":version,"files":files}, indent=2) + "\n")
    output = payload.parent / f"SKZ-WorkBuddy-Setup-{version}.exe"
    subprocess.run([
        compiler, f"-DVERSION={version}", f"-DPAYLOAD={payload}",
        f"-DOUTPUT={output}",
        str(ROOT / "scripts/windows/setup.nsi"),
    ], check=True)
    output.with_suffix(".exe.sha256").write_text(f"{sha256(output)}  {output.name}\n")
    friendly = output.with_name("胜可知WorkBuddy一键启动.exe")
    shutil.copy2(output, friendly)
    friendly.with_suffix(".exe.sha256").write_text(f"{sha256(friendly)}  {friendly.name}\n")
    return output


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    build_target("x86_64-pc-windows-gnu", args.output)
    build_bundle(args.output / "plugins")
    output = build_installer(args.output, cargo_field("version"))
    print(output)


if __name__ == "__main__":
    main()
