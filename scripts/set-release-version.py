#!/usr/bin/env python3
"""Set the local crate version from a release tag.

The repository keeps a placeholder package version in source control. Release
workflows call this script with the tag name that triggered the workflow so the
published crate/binary metadata always matches that tag.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SEMVER_RE = re.compile(r"^v?(?P<version>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$")


def version_from_tag(tag: str) -> str:
    match = SEMVER_RE.fullmatch(tag)
    if not match:
        raise SystemExit(f"invalid release tag: {tag!r}; expected vMAJOR.MINOR.PATCH")
    return match.group("version")


def replace_package_version(path: Path, package_name: str, version: str) -> None:
    lines = path.read_text().splitlines()
    in_package = False
    found_name = False

    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "[[package]]" or stripped == "[package]":
            in_package = True
            found_name = stripped == "[package]"
            continue
        if in_package and stripped.startswith("["):
            in_package = False
            found_name = False
            continue
        if in_package and stripped == f'name = "{package_name}"':
            found_name = True
            continue
        if in_package and found_name and stripped.startswith("version ="):
            lines[index] = f'version = "{version}"'
            path.write_text("\n".join(lines) + "\n")
            return

    raise SystemExit(f"{path}: package {package_name!r} version not found")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: set-release-version.py <release-tag>")

    version = version_from_tag(sys.argv[1])
    replace_package_version(Path("Cargo.toml"), "jbx", version)
    replace_package_version(Path("Cargo.lock"), "jbx", version)
    print(f"Set jbx release version to {version}")


if __name__ == "__main__":
    main()
