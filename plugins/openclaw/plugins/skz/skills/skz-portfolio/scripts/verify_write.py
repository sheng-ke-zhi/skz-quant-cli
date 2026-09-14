#!/usr/bin/env python3
"""Read back state after an uncertain skz write; never replay the write."""

from __future__ import annotations

import argparse
import sys

sys.dont_write_bytecode = True

from _skz_common import emit, find_value, items, run_skz


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "operation",
        choices=("route.create", "problem.create", "problem.delete", "mine.start", "mining.delete-run", "explore.start", "promote.start", "strategy.write", "portfolio.create"),
    )
    parser.add_argument("--code", help="Resource, route, problem, strategy, or portfolio code.")
    parser.add_argument("--name", help="Created route name.")
    parser.add_argument("--promotion-id")
    args = parser.parse_args()
    observed = None
    confirmed: bool | None = None
    meaning = "inconclusive"
    not_found = False

    if args.operation == "route.create":
        if not args.name:
            parser.error("route.create requires --name")
        observed = run_skz("factor-routes", "list")
        confirmed = observed["ok"] and find_value(observed.get("data"), {args.name})
        meaning = "present" if confirmed else "absent" if observed["ok"] else "inconclusive"
    elif args.operation in {"problem.create", "problem.delete"}:
        if not args.code:
            parser.error(f"{args.operation} requires --code")
        observed = run_skz("problem", "get", args.code)
        envelope = observed.get("error", {})
        error = envelope.get("error", {}) if isinstance(envelope, dict) else {}
        not_found = (observed.get("exit_code") == 2 and isinstance(error, dict)
                     and error.get("status") == 404 and str(error.get("code")) == "40400")
        present = observed["ok"]
        confirmed = present if args.operation.endswith("create") else not_found
        meaning = "present" if present else "absent" if not_found else "inconclusive"
    elif args.operation in {"mine.start", "explore.start"}:
        command = "mine" if args.operation.startswith("mine") else "explore"
        observed = run_skz(command, "runs", "--status", "active", "--size", "100")
        if args.code:
            confirmed = observed["ok"] and find_value(observed.get("data"), {args.code})
        else:
            confirmed = observed["ok"] and bool(items(observed))
        meaning = "active_match" if confirmed else "inconclusive"
    elif args.operation == "mining.delete-run":
        if not args.code:
            parser.error("mining.delete-run requires --code")
        observed = run_skz("mining", "runs")
        # Only the run_id column identifies a run. Absence is evidence only
        # when the response is a complete, well-formed inventory.
        data = observed.get("data")
        rows = data.get("items") if isinstance(data, dict) else None
        if observed["ok"] and isinstance(rows, list) and all(
            isinstance(row, dict) and isinstance(row.get("run_id"), str)
            for row in rows
        ):
            present = any(row["run_id"] == args.code for row in rows)
            if present:
                confirmed, meaning = False, "run_present"
            elif type(data.get("total")) is int and data["total"] == len(rows):
                confirmed, meaning = True, "run_absent"
    elif args.operation == "promote.start" and args.promotion_id:
        observed = run_skz("promote", "get", args.promotion_id)
        confirmed = observed["ok"]
        meaning = "promotion_found" if confirmed else "inconclusive"
    elif args.operation in {"promote.start", "strategy.write"}:
        if not args.code:
            parser.error(f"{args.operation} requires --code or --promotion-id")
        observed = run_skz("strategy", "get", args.code)
        confirmed = observed["ok"]
        meaning = "strategy_found" if confirmed else "strategy_not_found" if observed.get("exit_code") == 2 else "inconclusive"
    elif args.operation == "portfolio.create":
        if not args.code:
            parser.error("portfolio.create requires --code")
        observed = run_skz("portfolio", "list")
        confirmed = observed["ok"] and find_value(observed.get("data"), {args.code})
        meaning = "portfolio_found" if confirmed else "portfolio_not_found" if observed["ok"] else "inconclusive"

    if not observed["ok"] and not not_found:
        confirmed, meaning = None, "inconclusive"
    # A read cannot establish that a timed-out write will never commit later.
    # Never authorize replay, even when the resource is currently absent.
    safe_to_retry = False
    emit(
        {
            "operation": args.operation,
            "confirmed": confirmed,
            "meaning": meaning,
            "safe_to_retry": safe_to_retry,
            "observed": observed,
            "note": "Never replay an uncertain write automatically; unresolved results require user review.",
        }
    )
    return 0 if confirmed is not None else 2


if __name__ == "__main__":
    raise SystemExit(main())
