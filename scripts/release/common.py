"""WSL、macOS 和 CI 共用的 release 构建基础设施。"""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / "release-dist"


def cargo_field(field: str) -> str:
    package = (ROOT / "Cargo.toml").read_text().split("\n[", 1)[0]
    match = re.search(rf'^{field}\s*=\s*"([^"]*)"', package, re.MULTILINE)
    if not match:
        raise SystemExit(f"Cargo.toml [package].{field} 不存在")
    return match.group(1)


def require_tool(name: str, install_hint: str) -> str:
    executable = shutil.which(name)
    if not executable:
        raise SystemExit(f"缺少 {name}。安装方式：{install_hint}")
    return executable


def run(command: list[str], *, env: dict[str, str] | None = None) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_commit() -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()


def git_dirty() -> bool:
    return bool(
        subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
    )


def write_manifest(host: str, records: list[dict[str, str]], output: Path) -> Path:
    manifest_dir = output / "manifests"
    manifest_dir.mkdir(parents=True, exist_ok=True)
    path = manifest_dir / f"{host}.json"
    version = cargo_field("version")
    commit = git_commit()
    merged = {record["target"]: record for record in records}
    if path.exists():
        previous = json.loads(path.read_text())
        if previous.get("version") == version and previous.get("commit") == commit:
            for record in previous.get("artifacts", []):
                merged.setdefault(record["target"], record)
    path.write_text(
        json.dumps(
            {
                "version": version,
                "commit": commit,
                "dirty": git_dirty(),
                "host": host,
                "artifacts": [merged[target] for target in sorted(merged)],
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n"
    )
    return path
