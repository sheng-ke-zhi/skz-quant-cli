#!/usr/bin/env python3
"""Build and validate the external, harness-specific skill bundle."""

from __future__ import annotations

import hashlib
import json
import shutil
import argparse
from pathlib import Path

from common import ROOT, cargo_field

TARGETS = ("claude", "codex", "openclaw", "hermes")
BOOKS = ("factor", "strategy", "guide", "portfolio")
CONTRACT = "3.0"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_bundle(output: Path, *, development: bool = False) -> Path:
    source = ROOT / "skills"
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    files: list[dict[str, object]] = []
    for target in TARGETS:
        for book in BOOKS:
            src = source / target / f"skz-{book}"
            if not (src / "SKILL.md").is_file():
                raise SystemExit(f"missing skill: {src / 'SKILL.md'}")
            dst = output / target / src.name
            shutil.copytree(src, dst, symlinks=False)
            for path in sorted(p for p in dst.rglob("*") if p.is_file()):
                rel = path.relative_to(output).as_posix()
                mode = path.stat().st_mode & 0o777
                files.append({"path": rel, "sha256": _digest(path), "mode": mode})
    manifest = {
        "cli": "development" if development else cargo_field("version"),
        "contract": CONTRACT,
        "targets": list(TARGETS),
        "books": list(BOOKS),
        "files": files,
    }
    path = output / "manifest.json"
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return path


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-dist" / "skills")
    parser.add_argument("--development", action="store_true")
    args = parser.parse_args()
    build_bundle(args.output, development=args.development)
