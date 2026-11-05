#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check the license header of every source file (spec ci-checks, "Every source file has a license header").

Every tracked source file must have `SPDX-License-Identifier: MIT OR Apache-2.0` in one of its
first three lines (a shebang line can come first), the Linux kernel way: no copyright line per
file. Generated files get the line from their generator when it can write it; the ones it
cannot are on the GENERATED list below.

Usage: python3 scripts/check-license-headers.py   (inside the repository)
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HEADER = "SPDX-License-Identifier: MIT OR Apache-2.0"
SOURCE_SUFFIXES = {".rs", ".go", ".c", ".h", ".js", ".ts", ".py", ".sh"}
FIRST_LINES = 3

# Generated files whose generator cannot write the header. Keep this list short.
GENERATED = {
    "bindings/node/index.d.ts",  # the TypeScript types napi-rs writes (add-node-binding)
}


def tracked_files(root: Path) -> list[str]:
    names = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True, text=True
    ).stdout
    return [name for name in names.split("\0") if name]


def has_header(file: Path) -> bool:
    with file.open(encoding="utf-8", errors="replace") as text:
        first_lines = [text.readline() for _ in range(FIRST_LINES)]
    return any(HEADER in line for line in first_lines)


def main() -> int:
    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True, text=True
        ).stdout.strip()
    )
    missing = [
        name
        for name in tracked_files(root)
        if Path(name).suffix in SOURCE_SUFFIXES
        and name not in GENERATED
        and not has_header(root / name)
    ]

    if missing:
        for name in missing:
            print(f"No license header: {name}")
        print(f"Add `{HEADER}` in a comment in the first {FIRST_LINES} lines (AGENTS.md, Code style).")
        return 1
    print("Every source file has the license header.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
