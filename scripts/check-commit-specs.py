#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check that code is committed with its spec (spec ci-checks, "Code is committed with its spec").

On a branch `<type>/<change>` where `openspec/changes/<change>/` exists, every commit that is
not a merge and changes code (a file outside `openspec/` that is not Markdown) must also change
a file in `openspec/changes/<change>/`: spec changes, or at least the ticks in `tasks.md`. On
any other branch the check passes. A base of all zeros (the first push of a branch) checks only
<head>.

Usage: python3 scripts/check-commit-specs.py <base> <head> <branch>
"""

from __future__ import annotations

import subprocess
import sys


def git(*args: str) -> str:
    result = subprocess.run(["git", *args], check=True, capture_output=True, text=True)
    return result.stdout


def change_of_branch(branch: str) -> str | None:
    """The change name of a branch `<type>/<change>`, or None for any other branch."""
    _type, slash, change = branch.partition("/")
    if not slash or not change:
        return None
    return change


def commits_to_check(base: str, head: str) -> list[str]:
    """The commits of base..head that are not merges (only head when base is all zeros)."""
    if set(base) == {"0"}:
        return git("rev-list", "--no-merges", "--no-walk", head).split()
    return git("rev-list", "--no-merges", f"{base}..{head}").split()


def changed_files(commit: str) -> list[str]:
    names = git("show", "--name-only", "-z", "--format=", commit)
    return [name for name in names.split("\0") if name.strip()]


def is_code(path: str) -> bool:
    return not path.startswith("openspec/") and not path.endswith(".md")


def main(args: list[str]) -> int:
    if len(args) != 3:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    base, head, branch = args

    change = change_of_branch(branch)
    if change is None:
        print(f"Branch {branch} is not <type>/<change>: nothing to check.")
        return 0
    change_folder = f"openspec/changes/{change}"
    if not git("ls-tree", "-d", "--name-only", head, change_folder).strip():
        print(f"{change_folder}/ does not exist: nothing to check.")
        return 0

    failures: list[str] = []
    for commit in commits_to_check(base, head):
        files = changed_files(commit)
        code_files = [path for path in files if is_code(path)]
        changes_the_spec = any(path.startswith(change_folder + "/") for path in files)
        if code_files and not changes_the_spec:
            subject = git("log", "-1", "--format=%s", commit).strip()
            failures.append(f"{commit[:12]} {subject} (code: {', '.join(code_files)})")

    if failures:
        for failure in failures:
            print(f"Code without its spec: {failure}")
        print(
            f"A commit with code also changes {change_folder}/: the spec changes it needs "
            "and its ticks in tasks.md (AGENTS.md, Commits)."
        )
        return 1
    print(f"Every commit with code also changes {change_folder}/.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
