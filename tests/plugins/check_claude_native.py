#!/usr/bin/env python3
"""Opt-in smoke check using a real Claude CLI and an isolated HOME/config."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skz", type=Path, required=True)
    parser.add_argument("--claude", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    skz = args.skz.resolve()
    claude = args.claude.absolute()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parents[2]
    results = {}
    manifest = json.loads((repo / "plugins/manifest.json").read_text())

    def check_payload(listing) -> int:
        plugin = next(item for item in listing if item["id"] == "skz@skz" and item["scope"] == "user")
        installed_root = Path(plugin["installPath"])
        checked = 0
        for entry in manifest["files"]:
            if entry["path"].startswith("shared/skills/"):
                path = installed_root / entry["path"].removeprefix("shared/")
                assert hashlib.sha256(path.read_bytes()).hexdigest() == entry["sha256"], entry["path"]
                checked += 1
        return checked

    with tempfile.TemporaryDirectory(prefix="claude-smoke-", dir=output) as temp:
        home = Path(temp)
        env = {
            **os.environ,
            "HOME": temp,
            "XDG_CONFIG_HOME": str(home / ".config"),
            "CLAUDE_CONFIG_DIR": str(home / ".claude"),
            "SKZ_PLUGINS_DIR": str(repo / "plugins"),
            "PATH": str(claude.parent) + os.pathsep + os.environ.get("PATH", ""),
        }

        def run(name: str, command: list[str]):
            result = subprocess.run(command, cwd=home, env=env, text=True, capture_output=True, timeout=120)
            # Preserve the actual JSON shape without publishing machine-specific paths.
            text = result.stdout.replace(temp, "<HOME>")
            (output / f"{name}.txt").write_text(text + result.stderr.replace(temp, "<HOME>"), encoding="utf-8")
            if result.returncode:
                raise RuntimeError(f"{name} failed ({result.returncode}): {result.stderr}")
            return json.loads(result.stdout) if name != "version" else result.stdout.strip()

        results["claude_version"] = run("version", [str(claude), "--version"])
        run("install", [str(skz), "plugin", "install", "claude"])
        listing = run("list-installed", [str(claude), "plugin", "list", "--json"])
        results["installed"] = run("status-installed", [str(skz), "plugin", "status", "claude"])
        results["installed_shared_files"] = check_payload(listing)

        receipt_path = home / ".skz/plugins/claude/.skz-plugin-install.json"
        receipt = json.loads(receipt_path.read_text())
        receipt["contract"] = "4.3"
        receipt.pop("skills", None)
        receipt_path.write_text(json.dumps(receipt))
        results["legacy"] = run("status-legacy", [str(skz), "plugin", "status", "claude"])
        run("upgrade", [str(skz), "plugin", "upgrade", "claude"])
        listing = run("list-upgraded", [str(claude), "plugin", "list", "--json"])
        results["upgraded"] = run("status-upgraded", [str(skz), "plugin", "status", "claude"])
        results["upgraded_shared_files"] = check_payload(listing)
        results["receipt"] = json.loads(receipt_path.read_text())
        run("uninstall", [str(skz), "plugin", "uninstall", "claude"])
        results["uninstalled"] = run("status-uninstalled", [str(skz), "plugin", "status", "claude"])
        (output / "results.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
        for phase in ("installed", "upgraded"):
            status = results[phase]
            assert status["content_ok"] and status["native_ok"] and status["installed"], status
            assert not status["needs_upgrade"], status
        assert results["legacy"]["needs_upgrade"]
        assert not results["uninstalled"]["installed"]
        assert results["receipt"]["contract"] == manifest["contract"]
        assert results["receipt"]["skills"] == manifest["skills"]
        print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
