# SPDX-License-Identifier: MIT OR Apache-2.0
"""A throwaway git repository for the tests of the repo scripts."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent

# The tests must not depend on the git config of the machine (hooks, signing, user).
GIT_ENV = {
    **os.environ,
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "Test",
    "GIT_AUTHOR_EMAIL": "test@example.com",
    "GIT_COMMITTER_NAME": "Test",
    "GIT_COMMITTER_EMAIL": "test@example.com",
}


class TempRepo:
    """A git repository in a temporary folder, deleted at the end of the `with` block."""

    def __init__(self) -> None:
        self._folder = tempfile.TemporaryDirectory()
        self.path = Path(self._folder.name)
        self.git("init", "--quiet", "--initial-branch=main")

    def __enter__(self) -> TempRepo:
        return self

    def __exit__(self, *error: object) -> None:
        self._folder.cleanup()

    def git(self, *args: str) -> str:
        result = subprocess.run(
            ["git", *args], cwd=self.path, env=GIT_ENV, check=True, capture_output=True, text=True
        )
        return result.stdout.strip()

    def write(self, name: str, text: str) -> None:
        file = self.path / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text)

    def commit(self, subject: str, *files: str) -> str:
        """Change each file, commit everything with this subject, and return the hash."""
        for name in files:
            self.write(name, f"{subject}\n")
        self.git("add", "--all")
        self.git("commit", "--quiet", "--allow-empty", "--message", subject)
        return self.git("rev-parse", "HEAD")

    def run_script(self, name: str, *args: str) -> subprocess.CompletedProcess[str]:
        """Run scripts/<name> inside this repository."""
        script = SCRIPTS / name
        interpreter = "bash" if name.endswith(".sh") else sys.executable
        return subprocess.run(
            [interpreter, str(script), *args],
            cwd=self.path,
            env=GIT_ENV,
            capture_output=True,
            text=True,
        )
