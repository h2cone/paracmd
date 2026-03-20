#!/usr/bin/env python3
"""Extract a version section from CHANGELOG.md for GitHub Releases."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


VERSION_HEADING = re.compile(r"^## \[(?P<version>[^\]]+)\](?:\s+-\s+.+)?\s*$")


def normalize_version(value: str) -> str:
    return value[1:] if value.startswith("v") else value


def extract_section(lines: list[str], version: str) -> str:
    collecting = False
    section: list[str] = []

    for line in lines:
        match = VERSION_HEADING.match(line)
        if match:
            current_version = normalize_version(match.group("version"))
            if collecting:
                break
            if current_version == version:
                collecting = True
                continue

        if collecting:
            section.append(line)

    content = "\n".join(section).strip()
    if not content:
        raise ValueError(
            f"Unable to find a non-empty CHANGELOG.md section for version {version}."
        )
    return content + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract the release notes for a version from CHANGELOG.md."
    )
    parser.add_argument("changelog", type=Path, help="Path to CHANGELOG.md")
    parser.add_argument("version", help="Release version, with or without a leading v")
    parser.add_argument("output", type=Path, help="Path to the output markdown file")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    version = normalize_version(args.version)

    try:
        content = args.changelog.read_text(encoding="utf-8")
        notes = extract_section(content.splitlines(), version)
    except Exception as exc:  # noqa: BLE001
        print(exc, file=sys.stderr)
        return 1

    args.output.write_text(notes, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
