#!/usr/bin/env python3
"""Check CLI contract, active identity, write policy, and remote identity."""

from __future__ import annotations

import argparse
import sys

sys.dont_write_bytecode = True

from _skz_common import emit, run_skz


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--operation",
        choices=("read", "write", "paid"),
        default="read",
        help="Required capability; write and paid reject local read-only mode.",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="Skip whoami; only inspect local CLI and credential policy.",
    )
    args = parser.parse_args()

    version = run_skz("--version")
    auth = run_skz("auth", "status")
    auth_data = auth.get("data") if isinstance(auth.get("data"), dict) else {}
    checks = {
        "cli": version["ok"],
        "identity": auth["ok"] and bool(auth_data.get("present")),
        "writable": args.operation == "read" or not bool(auth_data.get("readOnly", True)),
    }
    whoami = None
    if not args.offline and checks["identity"]:
        whoami = run_skz("whoami")
        checks["remote_identity"] = whoami["ok"]
    ready = all(checks.values())
    emit(
        {
            "ready": ready,
            "operation": args.operation,
            "checks": checks,
            "version": version,
            "auth": auth,
            "whoami": whoami,
            "next_action": None if ready else "fix_auth_or_policy",
        }
    )
    return 0 if ready else 2


if __name__ == "__main__":
    raise SystemExit(main())
