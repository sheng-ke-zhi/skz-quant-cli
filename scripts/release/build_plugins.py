#!/usr/bin/env python3
"""Build and validate the bundled, harness-native SKZ plugins."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path

from common import ROOT, cargo_field

TARGETS = ("claude", "codex", "openclaw", "hermes")
BOOKS = ("factor", "candidate", "strategy", "guide", "create-problem", "portfolio")
CONTRACT = "4.1"
AUTHORING = ROOT / "plugin-src"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


_COPY_IGNORE = shutil.ignore_patterns("__pycache__", "*.pyc")


def _overlay(source: Path, output: Path) -> None:
    if source.is_dir():
        shutil.copytree(source, output, dirs_exist_ok=True, ignore=_COPY_IGNORE)


def _copy_skills(plugin: Path, target: str) -> None:
    common = AUTHORING / "common"
    overrides = AUTHORING / "targets" / target
    for book in BOOKS:
        source = AUTHORING / "books" / f"skz-{book}"
        if not (source / "SKILL.md").is_file():
            raise SystemExit(f"missing authored skill: {source / 'SKILL.md'}")
        output = plugin / "skills" / source.name
        shutil.copytree(source, output)
        shutil.copytree(common / "references", output / "references")
        shutil.copytree(common / "scripts", output / "scripts", ignore=_COPY_IGNORE)
        _overlay(overrides / "books" / source.name, output)
        _overlay(overrides / "common" / "references", output / "references")
        _overlay(overrides / "common" / "scripts", output / "scripts")


def _render_target(root: Path, target: str, version: str) -> None:
    target_root = root / target
    plugin = target_root / "plugins" / "skz"
    plugin.mkdir(parents=True)
    _copy_skills(plugin, target)

    description = "胜可知量化投研与策略管理能力"
    if target in {"claude", "openclaw"}:
        _write_json(
            plugin / ".claude-plugin" / "plugin.json",
            {
                "name": "skz",
                "version": version,
                "description": description,
                "author": {"name": "胜可知"},
            },
        )
        _write_json(
            target_root / ".claude-plugin" / "marketplace.json",
            {
                "name": "skz",
                "description": description,
                "owner": {"name": "胜可知"},
                "plugins": [
                    {
                        "name": "skz",
                        "description": description,
                        "source": "./plugins/skz",
                        "category": "productivity",
                    }
                ],
            },
        )
    elif target == "codex":
        _write_json(
            plugin / ".codex-plugin" / "plugin.json",
            {
                "name": "skz",
                "version": version,
                "description": description,
                "author": {"name": "胜可知"},
                "skills": "./skills/",
                "interface": {
                    "displayName": "胜可知",
                    "shortDescription": description,
                    "longDescription": "使用胜可知 CLI 完成量化研究、因子管理、候选评审、策略运营与组合管理。",
                    "developerName": "胜可知",
                    "category": "Productivity",
                    "capabilities": ["Instructions"],
                    "defaultPrompt": ["使用 SKZ 完成这项量化投研任务。"],
                },
            },
        )
        _write_json(
            target_root / ".agents" / "plugins" / "marketplace.json",
            {
                "name": "skz",
                "interface": {"displayName": "胜可知"},
                "plugins": [
                    {
                        "name": "skz",
                        "source": {"source": "local", "path": "./plugins/skz"},
                        "policy": {
                            "installation": "AVAILABLE",
                            "authentication": "ON_INSTALL",
                        },
                        "category": "Productivity",
                    }
                ],
            },
        )
    else:
        (plugin / "plugin.yaml").write_text(
            "\n".join(
                [
                    "name: skz",
                    f'version: "{version}"',
                    f'description: "{description}"',
                    'author: "胜可知"',
                    "manifest_version: 1",
                    "",
                ]
            )
        )
        registrations = "\n".join(
            f'    ctx.register_skill("skz-{book}", root / "skills" / "skz-{book}" / "SKILL.md", "SKZ {book}")'
            for book in BOOKS
        )
        (plugin / "__init__.py").write_text(
            "from pathlib import Path\n\n\ndef register(ctx) -> None:\n"
            "    root = Path(__file__).parent\n"
            f"{registrations}\n"
        )


def sync_sources(destination: Path | None = None, *, development: bool = True) -> None:
    destination = destination or ROOT / "plugins"
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    version = cargo_field("version")
    for target in TARGETS:
        _render_target(destination, target, version)
    write_manifest(destination, development=development)


def write_manifest(root: Path, *, development: bool = False) -> Path:
    files = []
    for target in TARGETS:
        for path in sorted(p for p in (root / target).rglob("*") if p.is_file()):
            files.append(
                {
                    "path": path.relative_to(root).as_posix(),
                    "sha256": _digest(path),
                    "mode": path.stat().st_mode & 0o777,
                }
            )
    manifest = {
        "cli": "development" if development else cargo_field("version"),
        "contract": CONTRACT,
        "plugin": "skz",
        "targets": list(TARGETS),
        "skills": [f"skz-{book}" for book in BOOKS],
        "files": files,
    }
    path = root / "manifest.json"
    _write_json(path, manifest)
    return path


def assert_sources_synced() -> None:
    scratch = ROOT / "target" / "plugin-sync-check"
    sync_sources(scratch)
    expected = {
        p.relative_to(scratch): (_digest(p), p.stat().st_mode & 0o777)
        for p in scratch.rglob("*")
        if p.is_file()
    }
    rendered = ROOT / "plugins"
    actual = {
        p.relative_to(rendered): (_digest(p), p.stat().st_mode & 0o777)
        for p in rendered.rglob("*")
        if p.is_file() and "__pycache__" not in p.parts and p.suffix != ".pyc"
    }
    shutil.rmtree(scratch)
    if actual != expected:
        raise SystemExit("generated plugins are stale; run build_plugins.py --sync-only")


def build_bundle(output: Path, *, development: bool = False) -> Path:
    assert_sources_synced()
    if output.exists():
        shutil.rmtree(output)
    shutil.copytree(ROOT / "plugins", output)
    return write_manifest(output, development=development)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-dist" / "plugins")
    parser.add_argument("--development", action="store_true")
    parser.add_argument("--sync", action="store_true")
    parser.add_argument("--sync-only", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify plugins/ matches plugin-src without writing anything",
    )
    args = parser.parse_args()
    if args.check:
        assert_sources_synced()
    elif args.sync or args.sync_only:
        sync_sources()
    if not (args.check or args.sync_only):
        build_bundle(args.output, development=args.development)
