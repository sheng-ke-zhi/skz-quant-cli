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
BOOKS = ("factor", "candidate", "strategy", "guide", "portfolio")
CONTRACT = "3.5"
AUTHORING = ROOT / "skill-src"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sync_sources(destination: Path | None = None) -> None:
    """Render the canonical skill sources into every harness-specific tree."""
    destination = destination or ROOT / "skills"
    common = AUTHORING / "common"
    for book in BOOKS:
        source_book = AUTHORING / "books" / f"skz-{book}"
        if not (source_book / "SKILL.md").is_file():
            raise SystemExit(f"missing authored skill: {source_book / 'SKILL.md'}")
        for target in TARGETS:
            output = destination / target / source_book.name
            if output.exists():
                shutil.rmtree(output)
            shutil.copytree(source_book, output)
            shutil.copytree(common / "references", output / "references")
            shutil.copytree(
                common / "scripts",
                output / "scripts",
                ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
            )


def assert_sources_synced() -> None:
    rendered = ROOT / "skills"
    scratch = ROOT / "target" / "skill-sync-check"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    sync_sources(scratch)
    expected = {
        path.relative_to(scratch): (_digest(path), path.stat().st_mode & 0o777)
        for path in scratch.rglob("*")
        if path.is_file()
    }
    actual = {
        path.relative_to(rendered): (_digest(path), path.stat().st_mode & 0o777)
        for target in TARGETS
        for path in (rendered / target).rglob("*")
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc"
    }
    shutil.rmtree(scratch)
    if actual != expected:
        raise SystemExit("generated skills are stale; run scripts/release/build_skills.py --sync")


def write_manifest(root: Path, *, development: bool = False) -> Path:
    files: list[dict[str, object]] = []
    for target in TARGETS:
        for book in BOOKS:
            src = root / target / f"skz-{book}"
            if not (src / "SKILL.md").is_file():
                raise SystemExit(f"missing skill: {src / 'SKILL.md'}")
            for path in sorted(p for p in src.rglob("*") if p.is_file()):
                rel = path.relative_to(root).as_posix()
                mode = path.stat().st_mode & 0o777
                files.append({"path": rel, "sha256": _digest(path), "mode": mode})
    manifest = {
        "cli": "development" if development else cargo_field("version"),
        "contract": CONTRACT,
        "targets": list(TARGETS),
        "books": list(BOOKS),
        "files": files,
    }
    path = root / "manifest.json"
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return path


def build_bundle(output: Path, *, development: bool = False) -> Path:
    assert_sources_synced()
    source = ROOT / "skills"
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    for target in TARGETS:
        shutil.copytree(source / target, output / target, symlinks=False)
    return write_manifest(output, development=development)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-dist" / "skills")
    parser.add_argument("--development", action="store_true")
    parser.add_argument("--sync", action="store_true", help="Render canonical sources into skills/ first.")
    parser.add_argument("--sync-only", action="store_true", help="Sync skills/ and its development manifest, then stop.")
    args = parser.parse_args()
    if args.sync or args.sync_only:
        sync_sources()
        write_manifest(ROOT / "skills", development=True)
    if args.sync_only:
        raise SystemExit(0)
    build_bundle(args.output, development=args.development)
