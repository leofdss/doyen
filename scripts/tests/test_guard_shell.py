# SPDX-License-Identifier: MIT OR Apache-2.0
"""Tests of .claude/hooks/guard-shell.py (AGENTS.md, AI agents, "Shell commands never hang")."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

HOOK = Path(__file__).resolve().parents[2] / ".claude" / "hooks" / "guard-shell.py"


def run_hook(command: str, in_background: bool = False, tool: str = "Bash") -> str | None:
    """The deny reason the hook prints, or None when it lets the call through."""
    call = {"tool_name": tool, "tool_input": {"command": command, "run_in_background": in_background}}
    result = subprocess.run(
        [sys.executable, "-I", str(HOOK)],
        input=json.dumps(call),
        capture_output=True,
        text=True,
        check=True,
    )
    if not result.stdout.strip():
        return None
    return json.loads(result.stdout)["hookSpecificOutput"]["permissionDecisionReason"]


class ShellGuard(unittest.TestCase):
    def test_a_foreground_command_passes(self) -> None:
        self.assertIsNone(run_hook("distrobox enter doyen -- bash -lc 'cargo test'"))

    def test_a_zsh_split_is_denied(self) -> None:
        reason = run_hook("python3 - <<'EOF'\nprint(1)\nEOF\n$=0 2>/dev/null;\ncargo test")
        self.assertIn("zsh", reason)

    def test_a_background_command_without_timeout_is_denied(self) -> None:
        reason = run_hook("cargo test </dev/null", in_background=True)
        self.assertIn("timeout", reason)

    def test_a_background_command_without_closed_input_is_denied(self) -> None:
        reason = run_hook("timeout 600 cargo test", in_background=True)
        self.assertIn("/dev/null", reason)

    def test_a_bounded_background_command_passes(self) -> None:
        self.assertIsNone(run_hook("timeout 600 cargo test </dev/null", in_background=True))
        self.assertIsNone(
            run_hook("cd /repo && timeout -k 10 600 cargo test < /dev/null", in_background=True)
        )

    def test_other_tools_pass(self) -> None:
        self.assertIsNone(run_hook("$=0", tool="Edit"))


if __name__ == "__main__":
    unittest.main()
