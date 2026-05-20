#!/usr/bin/env python3
#  Project:   macbash
#  File:      scripts/release-prepare.py
#  Purpose:   Stamp VERSION / Cargo.toml / Cargo.lock with a release version.
#             Invoked by .releaserc.yaml's @semantic-release/exec prepareCmd.
#  Language:  Python 3
#
#  License:   Apache-2.0
#  Copyright: (c) 2025-2026 HYPERI PTY LIMITED
"""Stamp a release version into VERSION, Cargo.toml, and Cargo.lock.

Reads the workspace member name from Cargo.toml so the same script works
for any project that drops it in — no name hardcoded.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: release-prepare.py <version>")
    version = sys.argv[1]

    Path("VERSION").write_text(f"{version}\n")

    cargo_toml = Path("Cargo.toml")
    ct = cargo_toml.read_text()
    name_match = re.search(r'^name\s*=\s*"([^"]+)"', ct, flags=re.MULTILINE)
    if not name_match:
        sys.exit("release-prepare: could not find name in Cargo.toml")
    name = name_match.group(1)

    ct = re.sub(
        r'^version\s*=\s*"[^"]*"',
        f'version = "{version}"',
        ct,
        count=1,
        flags=re.MULTILINE,
    )
    cargo_toml.write_text(ct)

    lock_path = Path("Cargo.lock")
    if lock_path.exists():
        lock = lock_path.read_text()
        # Cargo.lock entries look like:
        #   [[package]]
        #   name = "macbash"
        #   version = "1.5.2"
        # Match exactly that block and bump only the version field.
        pattern = (
            r'(\[\[package\]\]\nname = "' + re.escape(name) + r'"\nversion = ")'
            r'[^"]+(")'
        )
        new_lock, count = re.subn(pattern, rf"\g<1>{version}\g<2>", lock, count=1)
        if count == 0:
            print(
                f"release-prepare: warning — no Cargo.lock entry found for "
                f"package {name!r}; lockfile unchanged",
                file=sys.stderr,
            )
        else:
            lock_path.write_text(new_lock)

    print(f"release-prepare: stamped {version} (package={name})")


if __name__ == "__main__":
    main()
