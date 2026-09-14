#!/usr/bin/env python3
"""Shared helpers for read-only skz skill scripts."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


def emit(value: Any, *, stream: Any = sys.stdout) -> None:
    json.dump(value, stream, ensure_ascii=False, separators=(",", ":"))
    stream.write("\n")


def run_skz(*args: str, stdin: Any | None = None) -> dict[str, Any]:
    command = [os.environ.get("SKZ_BIN", "skz"), *args]
    raw_stdin = None if stdin is None else json.dumps(stdin, ensure_ascii=False)
    try:
        completed = subprocess.run(
            command,
            input=raw_stdin,
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError as error:
        return {
            "ok": False,
            "command": command,
            "exit_code": None,
            "error": {"kind": "spawn_error", "message": str(error)},
        }
    stdout = completed.stdout.strip()
    stderr = completed.stderr.strip()
    result: dict[str, Any] = {
        "ok": completed.returncode == 0,
        "command": command,
        "exit_code": completed.returncode,
    }
    if stdout:
        try:
            result["data"] = json.loads(stdout)
        except json.JSONDecodeError:
            result["stdout"] = stdout
    if stderr:
        try:
            result["error"] = json.loads(stderr)
        except json.JSONDecodeError:
            result["stderr"] = stderr
    return result


def load_json(path: str | None) -> Any:
    if path:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    return json.load(sys.stdin)


def items(result: dict[str, Any]) -> list[dict[str, Any]]:
    data = result.get("data")
    if isinstance(data, list):
        return [item for item in data if isinstance(item, dict)]
    if isinstance(data, dict):
        value = data.get("items", [])
        if isinstance(value, list):
            return [item for item in value if isinstance(item, dict)]
    return []


def find_value(value: Any, names: set[str]) -> bool:
    if isinstance(value, dict):
        return any(
            (str(item) in names if key in {"code", "name", "strategy_code", "strategyCode", "portfolio_code", "portfolioCode"} else False)
            or find_value(item, names)
            for key, item in value.items()
        )
    if isinstance(value, list):
        return any(find_value(item, names) for item in value)
    return False
