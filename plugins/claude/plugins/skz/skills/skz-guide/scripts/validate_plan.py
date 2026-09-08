#!/usr/bin/env python3
"""Validate a paid skz execution plan without performing any write."""

from __future__ import annotations

import argparse
import sys
from typing import Any

sys.dont_write_bytecode = True

from _skz_common import emit, find_value, load_json, run_skz


REQUIRED = {
    "mine.start": ("route",),
    "explore.start": ("route", "problem"),
    "promote.start": ("experiment_id", "strategy_code"),
    "portfolio.create": ("portfolio_code", "candidate_strategies"),
}


def nonempty(plan: dict[str, Any], key: str) -> bool:
    value = plan.get(key)
    if key == "candidate_strategies":
        return isinstance(value, list) and bool(value) and all(isinstance(v, str) and v for v in value)
    return isinstance(value, str) and bool(value.strip())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", help="Read plan JSON from this file instead of stdin.")
    parser.add_argument("--offline", action="store_true", help="Only validate schema; skip live reads.")
    args = parser.parse_args()
    try:
        plan = load_json(args.file)
    except (OSError, ValueError) as error:
        emit({"valid": False, "errors": [f"invalid JSON: {error}"]})
        return 2
    if not isinstance(plan, dict):
        emit({"valid": False, "errors": ["plan must be a JSON object"]})
        return 2
    operation = plan.get("operation")
    errors: list[str] = []
    if operation not in REQUIRED:
        errors.append(f"operation must be one of: {', '.join(REQUIRED)}")
    else:
        errors.extend(f"missing or invalid {key}" for key in REQUIRED[operation] if not nonempty(plan, key))
    for key in ("assumption", "failure_signal"):
        if not nonempty(plan, key):
            errors.append(f"missing or invalid {key}")
    if errors or args.offline:
        emit(
            {
                "valid": not errors,
                "approved": False,
                "approval_required": True,
                "operation": operation,
                "errors": errors,
                "live_checks": [],
            }
        )
        return 0 if not errors else 2

    live_checks: list[dict[str, Any]] = []
    auth = run_skz("auth", "status")
    auth_data = auth.get("data") if isinstance(auth.get("data"), dict) else {}
    live_checks.append({"name": "writable_identity", "ok": auth["ok"] and auth_data.get("present") and not auth_data.get("readOnly"), "result": auth})
    if operation in {"mine.start", "explore.start"}:
        routes = run_skz("factor-routes", "list")
        live_checks.append({"name": "route_exists", "ok": routes["ok"] and find_value(routes.get("data"), {plan["route"]}), "result": routes})
    if operation == "explore.start":
        problem = run_skz("problem", "get", plan["problem"])
        live_checks.append({"name": "problem_exists", "ok": problem["ok"], "result": problem})
    elif operation == "promote.start":
        candidates = run_skz("experiment", "strategies", plan["experiment_id"])
        live_checks.append({"name": "candidate_exists", "ok": candidates["ok"] and find_value(candidates.get("data"), {plan["strategy_code"]}), "result": candidates})
    elif operation == "portfolio.create":
        strategies = run_skz("strategy", "list", "--status", "实盘", "--page-size", "200")
        wanted = set(plan["candidate_strategies"])
        live_checks.append({"name": "candidates_live", "ok": strategies["ok"] and all(find_value(strategies.get("data"), {code}) for code in wanted), "result": strategies})
        portfolios = run_skz("portfolio", "list")
        live_checks.append({"name": "portfolio_code_available", "ok": portfolios["ok"] and not find_value(portfolios.get("data"), {plan["portfolio_code"]}), "result": portfolios})
    valid = all(check["ok"] for check in live_checks)
    emit(
        {
            "valid": valid,
            "approved": False,
            "approval_required": True,
            "operation": operation,
            "errors": [] if valid else [check["name"] for check in live_checks if not check["ok"]],
            "live_checks": live_checks,
            "plan": plan,
        }
    )
    return 0 if valid else 2


if __name__ == "__main__":
    raise SystemExit(main())
