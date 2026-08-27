#!/usr/bin/env python3
"""Rebuild cross-session skz research state using read-only commands."""

from __future__ import annotations

import argparse
import sys

sys.dont_write_bytecode = True

from _skz_common import emit, items, run_skz


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=100, help="Maximum active runs to request.")
    args = parser.parse_args()
    if not 1 <= args.size <= 100:
        parser.error("--size must be between 1 and 100")

    auth = run_skz("auth", "status")
    auth_data = auth.get("data") if isinstance(auth.get("data"), dict) else {}
    if not auth["ok"] or not auth_data.get("present"):
        emit({"ready": False, "auth": auth, "next_action": "fix_auth"})
        return 2

    whoami = run_skz("whoami")
    mining = run_skz("mine", "runs", "--status", "active", "--size", str(args.size))
    exploration = run_skz("explore", "runs", "--status", "active", "--size", str(args.size))
    portfolios = run_skz("portfolio", "list")
    portfolios_missing_performance = [
        item
        for item in items(portfolios)
        if item.get("has_performance") is False
    ]
    active_mining = items(mining)
    active_exploration = items(exploration)
    if active_exploration:
        suggested_skill, next_action = "skz-guide", "poll_exploration"
    elif active_mining:
        suggested_skill, next_action = "skz-guide", "poll_mining"
    elif portfolios_missing_performance:
        suggested_skill, next_action = "skz-portfolio", "refresh_portfolio_performance"
    else:
        suggested_skill, next_action = "skz-guide", "inspect_existing_assets"
    ready = all(result["ok"] for result in (whoami, mining, exploration, portfolios))
    emit(
        {
            "ready": ready,
            "identity": whoami.get("data"),
            "active_mining": active_mining,
            "active_exploration": active_exploration,
            "portfolios_missing_performance": portfolios_missing_performance,
            "suggested_skill": suggested_skill,
            "next_action": next_action,
            "diagnostics": {
                "mine": None if mining["ok"] else mining,
                "explore": None if exploration["ok"] else exploration,
                "portfolio": None if portfolios["ok"] else portfolios,
            },
        }
    )
    return 0 if ready else 2


if __name__ == "__main__":
    raise SystemExit(main())
