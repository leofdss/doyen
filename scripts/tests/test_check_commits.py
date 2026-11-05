# SPDX-License-Identifier: MIT OR Apache-2.0
"""Tests of scripts/check-commits.sh (capability ci-checks)."""

import unittest

from temp_repo import TempRepo

ALL_ZEROS = "0" * 40


class CommitMessageCheck(unittest.TestCase):
    def test_valid_subject(self) -> None:
        # spec: ci-checks / Valid subject
        valid_subjects = [
            "feat(core): add the local lease",
            "fix: renew on time",
            "feat(core)!: rename the status reasons",
            "feat(node,go): add the postgresql database type",
            "ci(deps): bump the actions group",
            "revert: remove the local lease",
        ]
        with TempRepo() as repo:
            base = repo.commit("chore: start")
            for subject in valid_subjects:
                repo.commit(subject, "lease.rs")

            result = repo.run_script("check-commits.sh", base, "HEAD")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_invalid_subject(self) -> None:
        # spec: ci-checks / Invalid subject
        invalid_subjects = [
            "Added lease",
            "feature: add the lease",
            "Feat: add the lease",
            "feat(core) add the lease",
            "feat: ",
        ]
        for subject in invalid_subjects:
            with self.subTest(subject=subject), TempRepo() as repo:
                base = repo.commit("chore: start")
                bad_commit = repo.commit(subject, "lease.rs")
                repo.commit("feat(core): add the local lease", "lease.rs")

                result = repo.run_script("check-commits.sh", base, "HEAD")

                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(bad_commit[:12], result.stdout)
                self.assertNotIn("add the local lease", result.stdout)

    def test_merge_commit(self) -> None:
        # spec: ci-checks / Merge commit
        with TempRepo() as repo:
            base = repo.commit("chore: start")
            repo.git("switch", "--quiet", "--create", "topic")
            repo.commit("feat(core): add the local lease", "lease.rs")
            repo.git("switch", "--quiet", "main")
            repo.commit("docs: explain the lease", "README.md")
            repo.git("merge", "--quiet", "--no-ff", "--no-edit", "topic")
            self.assertTrue(repo.git("log", "-1", "--format=%s").startswith("Merge "))

            result = repo.run_script("check-commits.sh", base, "HEAD")

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_first_push_of_a_branch_checks_only_the_head(self) -> None:
        # A push of a new branch has no "before" commit: GitHub sends all zeros.
        with TempRepo() as repo:
            repo.commit("Added lease", "lease.rs")
            repo.commit("feat(core): add the local lease", "lease.rs")
            self.assertEqual(repo.run_script("check-commits.sh", ALL_ZEROS, "HEAD").returncode, 0)

            repo.commit("Fixed lease", "lease.rs")
            self.assertEqual(repo.run_script("check-commits.sh", ALL_ZEROS, "HEAD").returncode, 1)


if __name__ == "__main__":
    unittest.main()
