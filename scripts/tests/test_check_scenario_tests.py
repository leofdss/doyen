# SPDX-License-Identifier: MIT OR Apache-2.0
"""Tests of scripts/check-scenario-tests.py (capability ci-checks)."""

from __future__ import annotations

import unittest

from temp_repo import TempRepo

SCRIPT = "check-scenario-tests.py"
SCENARIOS = ["Three members", "Master dies", "Old master comes back"]
MANUAL_CHECKS = "openspec/manual-checks.md"


def spec_text(scenarios: list[str]) -> str:
    lines = ["# Spec", "", "### Requirement: Master is the oldest live member", "Text.", ""]
    for name in scenarios:
        lines += [f"#### Scenario: {name}", "- **WHEN** something", "- **THEN** something", ""]
    return "\n".join(lines)


def rust_test_naming(capability: str, scenario: str) -> str:
    """A Rust test whose comment names one scenario."""
    return "#[test]\n" + f"// spec: {capability} / {scenario}\n" + "fn it_works() {}\n"


def add_main_spec(repo: TempRepo) -> None:
    """A spec with three scenarios: one named by a test, one by a manual check, one by nothing."""
    repo.write("openspec/specs/election-core/spec.md", spec_text(SCENARIOS))
    repo.write("crates/doyen/tests/election.rs", rust_test_naming("election-core", "Three members"))
    repo.write(
        MANUAL_CHECKS,
        "# Manual checks\n\n- election-core / Old master comes back: run by hand on 2026-11-05\n",
    )


class ScenarioTestCheck(unittest.TestCase):
    def test_missing_test(self) -> None:
        # spec: ci-checks / Missing test
        with TempRepo() as repo:
            add_main_spec(repo)

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("election-core / Master dies", result.stdout)
            self.assertNotIn("Three members", result.stdout)
            self.assertNotIn("Old master comes back", result.stdout)

    def test_stale_test(self) -> None:
        # spec: ci-checks / Stale test
        with TempRepo() as repo:
            add_main_spec(repo)
            repo.write("crates/doyen/tests/dies.rs", rust_test_naming("election-core", "Master dies"))
            repo.write("crates/doyen/tests/old.rs", rust_test_naming("election-core", "Old name"))

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("crates/doyen/tests/old.rs", result.stdout)
            self.assertIn("election-core / Old name", result.stdout)

    def test_checked_by_hand(self) -> None:
        # spec: ci-checks / Checked by hand
        with TempRepo() as repo:
            add_main_spec(repo)
            repo.write("crates/doyen/tests/dies.rs", rust_test_naming("election-core", "Master dies"))

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_manual_check_needs_how_and_when(self) -> None:
        with TempRepo() as repo:
            add_main_spec(repo)
            repo.write("crates/doyen/tests/dies.rs", rust_test_naming("election-core", "Master dies"))
            repo.write(MANUAL_CHECKS, "# Manual checks\n\n- election-core / Old master comes back:\n")

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("election-core / Old master comes back", result.stdout)

    def test_open_change(self) -> None:
        # spec: ci-checks / Open change
        with TempRepo() as repo:
            change_spec = "openspec/changes/add-core-election/specs/election-core/spec.md"
            repo.write(change_spec, spec_text(["Master dies"]))
            self.assertEqual(repo.run_script(SCRIPT).returncode, 0)

            result = repo.run_script(SCRIPT, "--change", "add-core-election")
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("election-core / Master dies", result.stdout)

            repo.write("crates/doyen/tests/dies.rs", rust_test_naming("election-core", "Master dies"))
            for args in [(), ("--change", "add-core-election")]:
                result = repo.run_script(SCRIPT, *args)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_archived_change_is_not_open(self) -> None:
        with TempRepo() as repo:
            archived = "openspec/changes/archive/2026-11-05-add-ci-baseline/specs/ci/spec.md"
            repo.write(archived, spec_text(["Old name"]))
            repo.write("scripts/tests/test_x.py", "# spec: ci / Old name\n")

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("scripts/tests/test_x.py", result.stdout)

    def test_comment_in_a_workflow_step(self) -> None:
        # A release check can be a workflow step that names its scenario in a YAML comment.
        with TempRepo() as repo:
            repo.write("openspec/specs/release-process/spec.md", spec_text(["Tests from the package"]))
            repo.write(
                ".github/workflows/release.yml",
                "jobs:\n  check:\n    steps:\n"
                + "      # spec: release-process / Tests from the package\n"
                + "      - run: cargo test\n",
            )

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_unknown_change(self) -> None:
        with TempRepo() as repo:
            result = repo.run_script(SCRIPT, "--change", "add-nothing")

            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
