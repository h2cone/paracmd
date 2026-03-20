#!/usr/bin/env python3
"""Create a release archive for a built binary."""

from __future__ import annotations

import argparse
import shutil
import tarfile
import zipfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Package a compiled binary and project metadata into a release archive."
    )
    parser.add_argument("--binary", required=True, type=Path, help="Path to the built binary")
    parser.add_argument("--version", required=True, help="Release version without a leading v")
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument(
        "--format",
        required=True,
        choices=("zip", "tar.gz"),
        help="Archive format to produce",
    )
    parser.add_argument(
        "--output-dir",
        default=Path("dist"),
        type=Path,
        help="Directory where the archive will be written",
    )
    return parser.parse_args()


def add_tree_to_zip(archive: zipfile.ZipFile, root: Path, base: Path) -> None:
    for path in sorted(root.rglob("*")):
        archive.write(path, path.relative_to(base))


def main() -> int:
    args = parse_args()
    binary = args.binary
    if not binary.is_file():
        raise FileNotFoundError(f"Built binary not found: {binary}")

    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    package_name = f"{binary.stem}-v{args.version}-{args.target}"
    staging_root = output_dir / package_name
    if staging_root.exists():
        shutil.rmtree(staging_root)

    staging_root.mkdir(parents=True)
    shutil.copy2(binary, staging_root / binary.name)
    for extra_file in ("README.md", "LICENSE"):
        source = Path(extra_file)
        if source.is_file():
            shutil.copy2(source, staging_root / source.name)

    archive_name = f"{package_name}.{args.format}"
    archive_path = output_dir / archive_name

    if args.format == "zip":
        with zipfile.ZipFile(
            archive_path, mode="w", compression=zipfile.ZIP_DEFLATED
        ) as archive:
            add_tree_to_zip(archive, staging_root, output_dir)
    else:
        with tarfile.open(archive_path, mode="w:gz") as archive:
            archive.add(staging_root, arcname=package_name)

    shutil.rmtree(staging_root)
    print(archive_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
