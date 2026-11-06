#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""PreToolUse guard for the Bash tool: a shell command never hangs (AGENTS.md, AI agents).

Why: on 2026-11-06 a subagent command held a stray `$=0` line. In zsh that starts a second
shell that waits for input, and the command ran in the background, where no time limit
applies and nobody reads it. The shell was found 2 h 32 min later.

Rules (each one denies the call with the fix in the reason):
- `$=` anywhere: a zsh-only form; commands are written for bash.
- A background command (`run_in_background`) must start with `timeout <seconds>` (a `cd …
  &&` before it is fine) and must close its input with `</dev/null`, so a program that
  waits for a terminal ends at once and the whole command has a limit.
A foreground command already has the time limit of the tool, so it only needs the `$=` rule.
"""
import json
import re
import sys

ZSH_SPLIT = re.compile(r"\$=")
TIMEOUT_FIRST = re.compile(r"^\s*(?:cd\s+\S+\s*&&\s*)?timeout\s+(?:-[-\w]+\s+)*\d+")
NO_INPUT = re.compile(r"<\s*/dev/null")


def check(command: str, in_background: bool) -> str | None:
    """The reason to deny the command, or None when it may run."""
    if ZSH_SPLIT.search(command):
        return (
            "`$=` is a zsh form that starts a second shell waiting for input. Write the "
            "command for bash (AGENTS.md, AI agents)."
        )
    if in_background:
        if not TIMEOUT_FIRST.search(command):
            return (
                "A background command has no time limit and nobody reads it when it hangs. "
                "Run it in the foreground, or start it with `timeout <seconds>`: "
                "`timeout 600 <command> </dev/null` (AGENTS.md, AI agents)."
            )
        if not NO_INPUT.search(command):
            return (
                "A background command must close its input, so a program that waits for a "
                "terminal ends at once: add `</dev/null` (AGENTS.md, AI agents)."
            )
    return None


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    if data.get("tool_name") != "Bash":
        return 0
    tool_input = data.get("tool_input") or {}
    reason = check(tool_input.get("command", ""), bool(tool_input.get("run_in_background")))
    if reason is None:
        return 0
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
