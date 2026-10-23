#!/usr/bin/env python3
"""Validate a paid skz execution plan without performing any write."""

from __future__ import annotations

import argparse
from datetime import date
import sys
from typing import Any

sys.dont_write_bytecode = True

from _skz_common import emit, find_value, load_json, run_skz


REQUIRED = {
    "mine.start": ("route",),
    "explore.start": ("route", "problem"),
    "promote.start": ("experiment_id", "strategy_code"),
    "portfolio.create": ("portfolio_code", "candidate_strategies", "base_market", "rebalance_dates"),
}


def nonempty(plan: dict[str, Any], key: str) -> bool:
    value = plan.get(key)
    if key == "candidate_strategies":
        return (isinstance(value, list) and bool(value)
                and all(isinstance(code, str) and bool(code.strip()) and code == code.strip() for code in value))
    if key == "rebalance_dates":
        if not isinstance(value, list) or not value:
            return False
        try:
            return all(isinstance(day, str) and date.fromisoformat(day).isoformat() == day for day in value)
        except ValueError:
            return False
    if key == "portfolio_code":
        return isinstance(value, str) and bool(value.strip()) and value == value.strip()
    return isinstance(value, str) and bool(value.strip())


def live_strategy_inventory() -> dict[str, Any]:
    codes: set[str] = set()
    total = None
    page = 1
    size = 200
    while True:
        observed = run_skz("strategy", "list", "--status", "实盘", "--page", str(page), "--page-size", str(size))
        data = observed.get("data")
        reason = None
        if not observed["ok"]:
            reason = "page_failed"
        elif not isinstance(data, dict) or type(data.get("total")) is not int or data["total"] < 0:
            reason = "invalid_total"
        elif total is not None and data["total"] != total:
            reason = "total_changed"
        elif (type(data.get("page")) is not int or data["page"] != page
              or type(data.get("page_size")) is not int or data["page_size"] != size):
            reason = "invalid_page"
        elif not isinstance(data.get("items"), list) or len(data["items"]) > size:
            reason = "invalid_items"
        else:
            total = data["total"]
            for row in data["items"]:
                code = row.get("code") if isinstance(row, dict) else None
                if (not isinstance(code, str) or not code.strip() or code != code.strip()
                        or code in codes or row.get("status") != "实盘"):
                    reason = "invalid_or_duplicate_strategy"
                    break
                codes.add(code)
            if reason is None:
                if len(codes) > total:
                    reason = "total_exceeded"
                elif len(codes) < total and len(data["items"]) != size:
                    reason = "short_page"
        if reason:
            return {"ok": False, "reason": reason, "observed": observed}
        if len(codes) == total:
            return {"ok": True, "data": {"items": sorted(codes), "total": total}}
        page += 1


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
    if not isinstance(operation, str) or operation not in REQUIRED:
        errors.append(f"operation must be one of: {', '.join(REQUIRED)}")
    else:
        errors.extend(f"missing or invalid {key}" for key in REQUIRED[operation] if not nonempty(plan, key))
    if operation == "portfolio.create":
        for key in ("description", "base_freq", "price_field", "rebalance_method"):
            if key in plan and not isinstance(plan[key], str):
                errors.append(f"invalid {key}: expected string")
    for key in ("assumption", "failure_signal"):
        if not nonempty(plan, key):
            errors.append(f"missing or invalid {key}")
    if errors or args.offline:
        emit(
            {
                "valid": not errors,
                "validation_scope": "schema_only",
                "ready_to_submit": False,
                "billing_verified": False,
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
    live_checks.append({"name": "writable_identity", "ok": auth["ok"] and auth_data.get("present") is True and auth_data.get("readOnly") is False, "result": auth})
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
        strategies = live_strategy_inventory()
        wanted = set(plan["candidate_strategies"])
        candidates_live = wanted <= set(strategies["data"]["items"]) if strategies["ok"] else None
        candidate_state = "unknown"
        if candidates_live is not None:
            candidate_state = "passed" if candidates_live else "failed"
        live_checks.append({"name": "candidates_live", "ok": candidates_live,
                            "state": candidate_state,
                            "result": strategies})
        portfolios = run_skz("portfolio", "list")
        portfolio_data = portfolios.get("data")
        rows = portfolio_data.get("items") if isinstance(portfolio_data, dict) else None
        complete = (portfolios["ok"] and isinstance(rows, list)
                    and all(isinstance(row, dict) and isinstance(row.get("code"), str)
                            and bool(row["code"].strip()) and row["code"] == row["code"].strip()
                            for row in rows))
        available = all(row["code"] != plan["portfolio_code"] for row in rows) if complete else None
        portfolio_state = "unknown"
        if available is not None:
            portfolio_state = "passed" if available else "failed"
        live_checks.append({"name": "portfolio_code_available", "ok": available,
                            "state": portfolio_state,
                            "result": portfolios})
    valid = all(check["ok"] for check in live_checks)
    emit(
        {
            "valid": valid,
            "validation_scope": "read_only_preflight",
            "ready_to_submit": False,
            "billing_verified": False,
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
