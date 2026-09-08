#!/usr/bin/env python3
"""Native harness fixture: resolve marketplaces and cache only the plugin root."""

import json
import os
from pathlib import Path
import runpy
import shutil
import sys


target, *args = sys.argv[1:]
home = Path(os.environ["HOME"])
state = home / "fake-harness" / target
state.mkdir(parents=True, exist_ok=True)
registration = state / "source"
cache = state / "installed"
with (state / "calls.jsonl").open("a") as log:
    log.write(json.dumps(args) + "\n")


def install(source: Path) -> None:
    marketplace = source / (
        ".agents/plugins/marketplace.json" if target == "codex"
        else ".claude-plugin/marketplace.json"
    )
    entry = json.loads(marketplace.read_text())["plugins"][0]
    plugin_source = entry["source"]
    relative = plugin_source["path"] if isinstance(plugin_source, dict) else plugin_source
    plugin = source / relative
    manifest_dir = ".codex-plugin" if target == "codex" else ".claude-plugin"
    manifest = json.loads((plugin / manifest_dir / "plugin.json").read_text())
    assert manifest["name"] == "skz"
    if cache.exists():
        shutil.rmtree(cache)
    shutil.copytree(plugin, cache)
    skills = cache / manifest.get("skills", "skills")
    assert (skills / "skz-openapi/SKILL.md").is_file()
    assert (skills / "skz-guide/references/operating-contract.md").is_file()
    assert (skills / "skz-guide/scripts/preflight.py").is_file()


if args[:3] == ["plugin", "marketplace", "add"]:
    registration.write_text(args[3])
elif args[:3] == ["plugin", "marketplace", "update"]:
    assert Path(registration.read_text()).is_dir()
elif args[:2] in (["plugin", "install"], ["plugin", "add"], ["plugin", "update"]):
    install(Path(registration.read_text()))
elif args[:2] == ["plugins", "install"]:
    install(Path(args[args.index("--marketplace") + 1]))
elif args[:2] == ["plugins", "enable"]:
    plugin = home / ".hermes/plugins/skz"

    class Context:
        def register_skill(self, name, path, description):
            assert path.is_file(), name
            assert (path.parent / "references/operating-contract.md").is_file(), name

    runpy.run_path(str(plugin / "__init__.py"))["register"](Context())
elif args[:2] in (["plugin", "list"], ["plugins", "list"]):
    print(json.dumps({"plugins": [{"name": "skz"}] if cache.is_dir() else []}))
elif args[:2] in (["plugin", "uninstall"], ["plugin", "remove"], ["plugins", "uninstall"], ["plugins", "remove"]):
    shutil.rmtree(home / ".hermes/plugins/skz" if target == "hermes" else cache)
else:
    raise SystemExit(f"unexpected {target} command: {args}")
