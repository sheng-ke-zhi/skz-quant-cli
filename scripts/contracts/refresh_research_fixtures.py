#!/usr/bin/env python3
"""从后端生成的 OpenAPI 刷新 CLI 响应结构样例；不访问网络或执行业务请求。"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def sample(schema: dict, schemas: dict, nullable: bool, key: str = "", depth: int = 0):
    if depth > 24:
        raise ValueError("recursive schema requires a curated fixture")
    if "$ref" in schema:
        return sample(schemas[schema["$ref"].rsplit("/", 1)[-1]], schemas, nullable, key, depth + 1)
    choices = schema.get("oneOf", schema.get("anyOf"))
    if choices:
        if nullable and any(item.get("type") == "null" for item in choices):
            return None
        choice = next((item for item in choices if item.get("type") != "null"), choices[0])
        return sample(choice, schemas, nullable, key, depth + 1)
    if "allOf" in schema:
        result = {}
        for item in schema["allOf"]:
            value = sample(item, schemas, nullable, key, depth + 1)
            if not isinstance(value, dict):
                raise ValueError("non-object allOf requires a curated fixture")
            result.update(value)
        return result
    kind = schema.get("type", "object")
    if isinstance(kind, list):
        if nullable and "null" in kind:
            return None
        kind = next(item for item in kind if item != "null")
    if schema.get("nullable") and nullable:
        return None
    if schema.get("enum"):
        return next((value for value in schema["enum"] if value is not None), None)
    if kind == "object":
        if schema.get("properties"):
            return {name: sample(value, schemas, nullable, name, depth + 1)
                    for name, value in schema["properties"].items()}
        if isinstance(schema.get("additionalProperties"), dict):
            return {"contract_key": sample(schema["additionalProperties"], schemas, nullable, key, depth + 1)}
        # live_tracking 的市场分布是 Value；按实现补充其已知行结构。
        if key == "market_distribution":
            return {"market": "future", "实盘": 1, "暂停": 0, "废弃": 0, "total": 1}
        return {}
    if kind == "array":
        return [sample(schema.get("items", {}), schemas, nullable, key, depth + 1)]
    if kind == "integer":
        return max(1, int(schema.get("minimum", 0)))
    if kind == "number":
        return 0.25
    if kind == "boolean":
        return True
    if kind == "null":
        return None
    return "contract_value"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--openapi", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "tests/fixtures/backend-contracts/research.json")
    args = parser.parse_args()
    schemas = json.loads(args.openapi.read_text())["components"]["schemas"]
    tests = (ROOT / "tests/backend_contracts.rs").read_text()
    names = re.findall(r'check::<[^>]+>\(\s*"([^"]+)"', tests)
    if not names:
        raise ValueError("no response contract bindings found")
    fixtures = {name: {"populated": sample(schemas[name], schemas, False),
                       "nullable": sample(schemas[name], schemas, True)} for name in names}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(fixtures, ensure_ascii=False, indent=2) + "\n")
    print(f"generated {len(fixtures)} response contracts: {args.output}")


if __name__ == "__main__":
    main()
