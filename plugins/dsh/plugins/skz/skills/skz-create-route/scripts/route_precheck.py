#!/usr/bin/env python3
"""Check route fields offline without creating a route or granting approval."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
from typing import Any

sys.dont_write_bytecode = True

# 原稿尚未与共享脚本合并，生成物直接使用同目录 helper。
if not (Path(__file__).parent / "_skz_common.py").is_file():
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "common/scripts"))

from _skz_common import emit, load_json


MECHANISMS = (
    "错误定价", "风险补偿", "行为偏差", "流动性溢价", "制度性套利",
    "自我实现预言", "微观结构", "信息扩散", "趋势跟踪", "套保压力",
)
TEXT_FIELDS = ("name", "key_inspect", "economic_logic", "why_effective")
SUGGESTIONS = (
    (("反应不足", "反应过度", "散户追涨"), "行为偏差"),
    (("机构调仓", "被迫接盘"), "流动性溢价"),
    (("套保",), "套保压力"),
)


def check_route(route: Any) -> dict[str, Any]:
    result: dict[str, Any] = {
        "valid": False,
        "errors": [],
        "warnings": [],
        "approved": False,
        "approval_required": True,
        "note": "offline check only; it never replaces showing the route to the user and getting approval",
    }
    errors = result["errors"]
    warnings = result["warnings"]
    if not isinstance(route, dict):
        errors.append("route must be a JSON object")
        return result

    for field in TEXT_FIELDS:
        value = route.get(field)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"missing or invalid {field}: expected nonempty string")
    mechanism = route.get("market_mechanism")
    if not isinstance(mechanism, str) or mechanism not in MECHANISMS:
        errors.append(f"market_mechanism must be one of: {', '.join(MECHANISMS)}")
        if isinstance(mechanism, str):
            for keywords, suggestion in SUGGESTIONS:
                if any(keyword in mechanism for keyword in keywords):
                    warnings.append(f"market_mechanism: 你是不是想写 `{suggestion}`？仅为归并建议，最终由人判定。")
    failures = route.get("failure_scenarios")
    if (not isinstance(failures, list) or not failures
            or not all(isinstance(item, str) and item.strip() for item in failures)):
        errors.append("missing or invalid failure_scenarios: expected nonempty array of nonempty strings")
    tags = route.get("tags")
    if not isinstance(tags, list) or not all(isinstance(tag, str) for tag in tags):
        errors.append("missing or invalid tags: expected string array (may be empty)")
    if "compute_engine" in route:
        engine = route["compute_engine"]
        if not isinstance(engine, str) or not engine.strip():
            errors.append("invalid compute_engine: expected nonempty string")
    known_fields = {*TEXT_FIELDS, "market_mechanism", "failure_scenarios", "tags", "compute_engine"}
    for field in route:
        if field not in known_fields:
            warnings.append(f"unknown field: {field}")

    for field in ("economic_logic", "why_effective"):
        value = route.get(field)
        if isinstance(value, str) and value.strip() and len(value.strip()) < 15:
            warnings.append(f"{field}: 少于 15 字，一行通常只答了半问。")
    name = route.get("name")
    if isinstance(name, str) and len(name.strip()) > 30:
        warnings.append("name: 超过 30 字，优先用市场现成的叫法。")
    if isinstance(failures, list):
        for index, item in enumerate(failures):
            if isinstance(item, str) and item.strip() and len(item.strip()) < 6:
                warnings.append(f"failure_scenarios[{index}]: 少于 6 字，失效条件过泛，请说明可观察的失败信号。")

    result["valid"] = not errors
    if not errors:
        # 哈希字段原样保留，预检不能悄悄改写用户将要批准的路线。
        result["normalized"] = route
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", help="Read route JSON from this file instead of stdin.")
    args = parser.parse_args()
    try:
        result = check_route(load_json(args.file))
    except (OSError, ValueError) as error:
        result = check_route(None)
        result["errors"] = [f"invalid JSON: {error}"]
    emit(result)
    return 0 if result["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
