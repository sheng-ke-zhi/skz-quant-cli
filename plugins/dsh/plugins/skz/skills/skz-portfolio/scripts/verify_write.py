#!/usr/bin/env python3
"""Read back state after an uncertain skz write; never replay the write."""

from __future__ import annotations

import argparse
import json
import sys

sys.dont_write_bytecode = True

from _skz_common import emit, find_value, items, known_run_state, run_inventory, run_skz


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "operation",
        choices=("route.create", "problem.create", "problem.delete", "mine.start", "mining.delete-run", "explore.start", "promote.start", "strategy.write", "portfolio.create"),
    )
    parser.add_argument("--code", help="Resource, route, problem, strategy, or portfolio code.")
    parser.add_argument("--name", help="Created route name.")
    parser.add_argument("--promotion-id")
    parser.add_argument("--run-id", help="Trusted fcRunId from the original receipt.")
    parser.add_argument("--expected", type=json.loads, help="Expected strategy fields as a JSON object.")
    args = parser.parse_args()
    if args.expected is not None and (not isinstance(args.expected, dict) or not args.expected):
        parser.error("--expected must be a nonempty JSON object")
    if args.expected and not args.expected.keys() <= {"status", "memo", "tags"}:
        parser.error("--expected supports only writable fields: status, memo, tags")
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
        observed = run_inventory(command)
        matches = [row for row in items(observed)
                   if (not args.run_id or row.get("fcRunId") == args.run_id)
                   and (not args.code or args.code in (row.get("routeCode"), row.get("problemCode")))]
        if observed["ok"] and args.run_id and len(matches) == 1 and known_run_state(matches[0]):
            confirmed, meaning = True, "run_match"
        elif observed["ok"] and args.code and matches:
            meaning = "target_match_only"
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
    elif args.operation == "strategy.write":
        if not args.code:
            parser.error("strategy.write requires --code")
        flags = ("--require-memo",) if args.expected and "memo" in args.expected else ()
        observed = run_skz("strategy", "get", args.code, *flags)
        data = observed.get("data")
        if observed["ok"] and args.expected and isinstance(data, dict) and data.get("code") == args.code:
            confirmed = all(key in data and type(data[key]) is type(value) and data[key] == value
                            for key, value in args.expected.items())
            meaning = "expected_fields_match" if confirmed else "expected_fields_mismatch"
    elif args.operation == "promote.start":
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
