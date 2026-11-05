# SPDX-License-Identifier: MIT OR Apache-2.0
"""Tests of scripts/check-commit-specs.py (capability ci-checks)."""

import unittest

from temp_repo import TempRepo

SCRIPT = "check-commit-specs.py"
CHANGE_TASKS = "openspec/changes/add-core-election/tasks.md"
CODE = "crates/doyen/src/lease.rs"
ALL_ZEROS = "0" * 40


def start(repo: TempRepo) -> str:
    """Commit an open change on main and return that commit (the base of the branches)."""
    return repo.commit("docs: propose add-core-election", CHANGE_TASKS)


class CommitSpecCheck(unittest.TestCase):
    def test_code_with_its_spec(self) -> None:
        # spec: ci-checks / Code with its spec
        with TempRepo() as repo:
            base = start(repo)
            repo.git("switch", "--quiet", "--create", "feat/add-core-election")
            repo.commit("feat(core): add the local lease", CODE, CHANGE_TASKS)

            result = repo.run_script(SCRIPT, base, "HEAD", "feat/add-core-election")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_code_without_its_spec(self) -> None:
        # spec: ci-checks / Code without its spec
        with TempRepo() as repo:
            base = start(repo)
            repo.git("switch", "--quiet", "--create", "feat/add-core-election")
            repo.commit("feat(core): add the local lease", CODE, CHANGE_TASKS)
            bad_commit = repo.commit("feat(core): renew the lease", CODE)

            result = repo.run_script(SCRIPT, base, "HEAD", "feat/add-core-election")

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn(bad_commit[:12], result.stdout)
            self.assertNotIn("add the local lease", result.stdout)

    def test_docs_only(self) -> None:
        # spec: ci-checks / Docs only
        with TempRepo() as repo:
            base = start(repo)
            repo.git("switch", "--quiet", "--create", "feat/add-core-election")
            repo.commit("docs: explain the lease", "README.md")

            result = repo.run_script(SCRIPT, base, "HEAD", "feat/add-core-election")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_branch_without_a_change(self) -> None:
        # spec: ci-checks / Branch without a change
        with TempRepo() as repo:
            base = start(repo)
            repo.git("switch", "--quiet", "--create", "fix/typo")
            repo.commit("fix(core): fix a typo", CODE)

            result = repo.run_script(SCRIPT, base, "HEAD", "fix/typo")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_branch_name_without_a_type(self) -> None:
        with TempRepo() as repo:
            base = start(repo)
            repo.commit("fix(core): fix a typo", CODE)

            self.assertEqual(repo.run_script(SCRIPT, base, "HEAD", "main").returncode, 0)

    def test_merge_commit_is_skipped(self) -> None:
        with TempRepo() as repo:
            start(repo)
            repo.git("switch", "--quiet", "--create", "feat/add-core-election")
            repo.commit("feat(core): add the local lease", CODE, CHANGE_TASKS)
            repo.git("switch", "--quiet", "main")
            base = repo.commit("fix(core): fix the clock", "crates/doyen/src/clock.rs")
            repo.git("switch", "--quiet", "feat/add-core-election")
            repo.git("merge", "--quiet", "--no-edit", "main")
            self.assertEqual(len(repo.git("log", "-1", "--format=%P").split()), 2)

            result = repo.run_script(SCRIPT, base, "HEAD", "feat/add-core-election")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_first_push_of_a_branch_checks_only_the_head(self) -> None:
        with TempRepo() as repo:
            start(repo)
            repo.git("switch", "--quiet", "--create", "feat/add-core-election")
            repo.commit("feat(core): renew the lease", CODE)
            repo.commit("feat(core): add the local lease", CODE, CHANGE_TASKS)
            result = repo.run_script(SCRIPT, ALL_ZEROS, "HEAD", "feat/add-core-election")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            repo.commit("feat(core): fix the lease", CODE)
            result = repo.run_script(SCRIPT, ALL_ZEROS, "HEAD", "feat/add-core-election")
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
