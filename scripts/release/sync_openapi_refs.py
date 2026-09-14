#!/usr/bin/env python3
"""Sync the public OpenAPI docs into the skz-openapi skill references."""

from __future__ import annotations

import argparse
import json
import re
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "plugin-src/books/skz-openapi/references"
ROUTES = ROOT / "tests/plugins/open_api_routes.json"
DOMAINS = ("market", "payment", "research", "strategy")


def route_pages(source: Path) -> list[tuple[Path, str, str]]:
    pages = []
    for path in sorted(source.rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        method = re.search(r'method="(GET|POST|PUT|PATCH|DELETE)"', text)
        route = re.search(r'path="([^"]+)"', text)
        if method and route:
            pages.append((path, method.group(1), route.group(1)))
    return pages


def render(path: Path, source_root: Path) -> str:
    text = path.read_text(encoding="utf-8")
    _, frontmatter, body = text.split("---", 2)
    title = re.search(r"^title:\s*(.+)$", frontmatter, re.MULTILINE)
    description = re.search(r"^description:\s*(.+)$", frontmatter, re.MULTILINE)
    if not title or not description:
        raise SystemExit(f"missing title or description: {path}")
    body = re.sub(r"<!--.*?-->\s*", "", body, flags=re.DOTALL)
    body = re.sub(r"import ApiDebugger from '[^']+';\s*", "", body)
    body = re.sub(r'<div className="apiRail">.*?</div>\s*', "", body, flags=re.DOTALL)
    relative = path.relative_to(source_root).with_suffix("")
    source = f"https://docs.shengkezhi.com/api/{relative.as_posix()}"
    return (
        "---\n"
        f"title: {title.group(1).strip()}\n"
        f"description: {description.group(1).strip()}\n"
        f"source: {source}\n"
        "---\n"
        + body.lstrip()
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--emit-patches", type=Path)
    args = parser.parse_args()
    source = args.source.resolve()
    pages = route_pages(source)
    routes = sorted(f"{method} {route}" for _, method, route in pages)
    if len(routes) != len(set(routes)):
        raise SystemExit("source docs contain duplicate method/path routes")

    rendered = {
        OUTPUT / path.relative_to(source): render(path, source)
        for path, _, _ in pages
    }
    route_text = json.dumps(routes, ensure_ascii=False, indent=2) + "\n"

    if args.emit_patches:
        args.emit_patches.mkdir(parents=True, exist_ok=True)
        existing = sorted(
            path
            for domain in DOMAINS
            for path in (OUTPUT / domain).rglob("*.md")
        )
        delete_lines = ["*** Begin Patch"]
        for path in existing:
            delete_lines.append(f"*** Delete File: {path.relative_to(ROOT).as_posix()}")
        if ROUTES.exists():
            delete_lines.append(f"*** Delete File: {ROUTES.relative_to(ROOT).as_posix()}")
        delete_lines.append("*** End Patch")
        (args.emit_patches / "delete.patch").write_text("\n".join(delete_lines) + "\n")

        add_lines = ["*** Begin Patch"]
        for path, content in sorted(rendered.items()):
            add_lines.append(f"*** Add File: {path.relative_to(ROOT).as_posix()}")
            add_lines.extend("+" + line for line in content.splitlines())
        add_lines.append(f"*** Add File: {ROUTES.relative_to(ROOT).as_posix()}")
        add_lines.extend("+" + line for line in route_text.splitlines())
        add_lines.append("*** End Patch")
        (args.emit_patches / "add.patch").write_text("\n".join(add_lines) + "\n")
        print(f"emitted patches for {len(routes)} OpenAPI routes")
        return

    for domain in DOMAINS:
        target = OUTPUT / domain
        if target.exists():
            shutil.rmtree(target)

    for target, content in rendered.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    ROUTES.write_text(route_text, encoding="utf-8")
    print(f"synced {len(routes)} OpenAPI routes")


if __name__ == "__main__":
    main()
