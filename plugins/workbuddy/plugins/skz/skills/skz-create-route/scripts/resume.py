#!/usr/bin/env python3
"""Rebuild cross-session skz research state using read-only commands."""

from __future__ import annotations

import argparse
import sys

sys.dont_write_bytecode = True

from _skz_common import emit, items, known_run_state, run_inventory, run_skz


def in_flight(result: dict) -> tuple[list[dict], bool]:
    active = []
    valid = result["ok"]
    for row in items(result):
        status = row.get("status")
        known = known_run_state(row)
        valid = valid and known
        if not known or status in {"queued", "dispatching", "running"}:
            active.append(row)
    return active, valid


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=100, help="Runs per page (all pages are read).")
    args = parser.parse_args()
    if not 1 <= args.size <= 100:
        parser.error("--size must be between 1 and 100")

    auth = run_skz("auth", "status")
    auth_data = auth.get("data") if isinstance(auth.get("data"), dict) else {}
    if not auth["ok"] or not auth_data.get("present"):
        emit({"ready": False, "safe_to_retry": False, "auth": auth, "next_action": "fix_auth"})
        return 2

    whoami = run_skz("whoami")
    mining = run_inventory("mine", args.size)
    exploration = run_inventory("explore", args.size)
    portfolios = run_skz("portfolio", "list")
    portfolios_missing_performance = [
        item
        for item in items(portfolios)
        if item.get("has_performance") is False
    ]
    active_mining, mining_complete = in_flight(mining)
    active_exploration, exploration_complete = in_flight(exploration)
    if active_exploration:
        suggested_skill, next_action = "skz-guide", "poll_exploration"
    elif active_mining:
        suggested_skill, next_action = "skz-guide", "poll_mining"
    elif portfolios_missing_performance:
        suggested_skill, next_action = "skz-portfolio", "refresh_portfolio_performance"
    else:
        suggested_skill, next_action = "skz-guide", "inspect_existing_assets"
    ready = (whoami["ok"] and portfolios["ok"] and mining_complete and exploration_complete)
    if not ready:
        next_action = "review_incomplete_recovery"
    emit(
        {
            "ready": ready,
            "complete": {"mine": mining_complete, "explore": exploration_complete},
            "safe_to_retry": False,
            "identity": whoami.get("data"),
            "active_mining": active_mining,
            "active_exploration": active_exploration,
            "portfolios_missing_performance": portfolios_missing_performance,
            "suggested_skill": suggested_skill,
            "next_action": next_action,
            "diagnostics": {
                "mine": None if mining_complete else mining,
                "explore": None if exploration_complete else exploration,
                "portfolio": None if portfolios["ok"] else portfolios,
            },
        }
    )
    return 0 if ready else 2


if __name__ == "__main__":
    raise SystemExit(main())
