#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check that every spec scenario has a test (spec ci-checks, "Every scenario has a test").

The tests are the source of truth: each scenario of a spec is named by a test, in a comment
at the start of a line, `// spec: <capability> / <scenario>` (`#` in Python, shell and YAML).
A scenario that only a person can check has a line in openspec/manual-checks.md instead:
`- <capability> / <scenario>: <how and when it was checked>`.

It fails when:
- a scenario of openspec/specs/ (or, with --change, of that open change's delta specs) is
  named by no test and no manual check;
- a test or a manual check names a scenario that no spec has (not in openspec/specs/ and not
  in the delta specs of an open change).

It reads the tests in crates/, bindings/, scripts/tests/ and .github/workflows/.

Usage: python3 scripts/check-scenario-tests.py [--change <name>]   (inside the repository)
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

TEST_FOLDERS = ["crates", "bindings", "scripts/tests", ".github/workflows"]
SKIPPED_FOLDERS = {".git", "target", "node_modules", "__pycache__"}
MANUAL_CHECKS = "openspec/manual-checks.md"

SCENARIO_HEADING = re.compile(r"^#### Scenario:\s*(.+?)\s*$")
SPEC_COMMENT = re.compile(r"^\s*(?://+|#)\s*spec:\s*([a-z0-9-]+)\s*/\s*(.+?)\s*$")
MANUAL_LINE = re.compile(r"^- ([a-z0-9-]+) / (.+?):\s*(.*?)\s*$")

Scenario = tuple[str, str]  # (capability, scenario name)


def scenarios_of(specs_folder: Path) -> set[Scenario]:
    """The scenarios of every <capability>/spec.md under a specs folder."""
    found: set[Scenario] = set()
    for spec in sorted(specs_folder.glob("*/spec.md")):
        capability = spec.parent.name
        for line in spec.read_text(encoding="utf-8").splitlines():
            match = SCENARIO_HEADING.match(line)
            if match:
                found.add((capability, match.group(1)))
    return found


def open_changes(root: Path) -> list[Path]:
    changes = root / "openspec" / "changes"
    if not changes.is_dir():
        return []
    return [folder for folder in sorted(changes.iterdir()) if folder.is_dir() and folder.name != "archive"]


def test_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for folder_name in TEST_FOLDERS:
        folder = root / folder_name
        if not folder.is_dir():
            continue
        for path in sorted(folder.rglob("*")):
            if path.is_file() and not SKIPPED_FOLDERS.intersection(path.relative_to(root).parts):
                files.append(path)
    return files


def named_by_tests(root: Path) -> dict[Scenario, list[str]]:
    """Each scenario named by a test comment, with the places (file:line) that name it."""
    named: dict[Scenario, list[str]] = {}
    for path in test_files(root):
        data = path.read_bytes()
        if b"\0" in data:  # a binary file
            continue
        for number, line in enumerate(data.decode("utf-8", errors="replace").splitlines(), 1):
            match = SPEC_COMMENT.match(line)
            if match:
                place = f"{path.relative_to(root)}:{number}"
                named.setdefault((match.group(1), match.group(2)), []).append(place)
    return named


def named_by_manual_checks(root: Path) -> dict[Scenario, list[str]]:
    """Each scenario of openspec/manual-checks.md that says how and when it was checked."""
    named: dict[Scenario, list[str]] = {}
    path = root / MANUAL_CHECKS
    if not path.is_file():
        return named
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = MANUAL_LINE.match(line)
        if match and match.group(3):
            named.setdefault((match.group(1), match.group(2)), []).append(f"{MANUAL_CHECKS}:{number}")
    return named


def repo_root() -> Path:
    top = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True, text=True
    )
    return Path(top.stdout.strip())


def main() -> int:
    parser = argparse.ArgumentParser(description="Check that every spec scenario has a test.")
    parser.add_argument("--change", help="check the delta specs of this open change")
    args = parser.parse_args()
    root = repo_root()

    if args.change:
        change_folder = root / "openspec" / "changes" / args.change
        if not change_folder.is_dir() or args.change == "archive":
            print(f"No open change {args.change} in openspec/changes/.", file=sys.stderr)
            return 2
        to_check = scenarios_of(change_folder / "specs")
    else:
        to_check = scenarios_of(root / "openspec" / "specs")

    known = scenarios_of(root / "openspec" / "specs")
    for change in open_changes(root):
        known |= scenarios_of(change / "specs")

    named = named_by_tests(root)
    for scenario, places in named_by_manual_checks(root).items():
        named.setdefault(scenario, []).extend(places)

    missing = sorted(to_check - named.keys())
    stale = sorted((scenario, places) for scenario, places in named.items() if scenario not in known)

    for capability, name in missing:
        print(f"No test for {capability} / {name}")
    for (capability, name), places in stale:
        for place in places:
            print(f"{place}: names {capability} / {name}, which no spec has")
    if missing or stale:
        print(
            "Each scenario needs a test with the comment `spec: <capability> / <scenario>`, "
            f"or a line in {MANUAL_CHECKS} (AGENTS.md, Tests first)."
        )
        return 1

    where = f"change {args.change}" if args.change else "openspec/specs/"
    print(f"Every scenario of {where} has a test ({len(to_check)} scenarios).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
