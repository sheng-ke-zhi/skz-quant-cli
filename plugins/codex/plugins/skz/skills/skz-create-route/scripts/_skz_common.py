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


def run_inventory(command: str, size: int = 100) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    seen: set[str] = set()
    total = None
    page = 1
    while True:
        observed = run_skz(command, "runs", "--page", str(page), "--size", str(size))
        data = observed.get("data")
        reason = None
        if not observed["ok"]:
            reason = "page_failed"
        elif not isinstance(data, dict) or type(data.get("total")) is not int or data["total"] < 0:
            reason = "invalid_total"
        elif total is not None and data["total"] != total:
            reason = "total_changed"
        elif (type(data.get("page")) is not int or data["page"] != page
              or type(data.get("size")) is not int or data["size"] != size):
            reason = "invalid_page"
        elif not isinstance(data.get("items"), list) or len(data["items"]) > size:
            reason = "invalid_items"
        else:
            total = data["total"]
            for row in data["items"]:
                run_id = row.get("fcRunId") if isinstance(row, dict) else None
                if not isinstance(run_id, str) or not run_id.strip() or run_id in seen:
                    reason = "invalid_or_duplicate_id"
                    break
                seen.add(run_id)
                rows.append(row)
            if len(rows) > total:
                reason = "total_exceeded"
            elif len(rows) < total and len(data["items"]) != size:
                reason = "short_page"
        if reason:
            return {"ok": False, "data": {"items": rows, "total": total},
                    "reason": reason, "observed": observed}
        if len(rows) == total:
            return {"ok": True, "data": {"items": rows, "total": total}}
        page += 1


def known_run_state(row: dict[str, Any]) -> bool:
    status = row.get("status")
    if not isinstance(status, str):
        return False
    if status in {"queued", "dispatching", "running"}:
        return row.get("done") is False and row.get("ok") is False
    if status in {"succeeded", "failed"}:
        return row.get("done") is True and row.get("ok") is (status == "succeeded")
    return False


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
