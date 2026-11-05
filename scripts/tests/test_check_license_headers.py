# SPDX-License-Identifier: MIT OR Apache-2.0
"""Tests of scripts/check-license-headers.py (capability ci-checks)."""

import unittest

from temp_repo import TempRepo

SCRIPT = "check-license-headers.py"
HEADER = "SPDX-License-Identifier: MIT OR Apache-2.0"


def add_fixture_tree(repo: TempRepo) -> None:
    repo.write("crates/doyen/src/lib.rs", f"// {HEADER}\n//! doyen\n")
    repo.write("scripts/run.sh", f"#!/usr/bin/env bash\n# {HEADER}\necho run\n")
    repo.write("bindings/node/index.d.ts", "/* generated */\nexport declare class Doyen {}\n")
    repo.write("README.md", "# doyen\n")
    repo.write("crates/doyen/src/lease.rs", "pub fn lease() {}\n")
    repo.git("add", "--all")
    repo.git("commit", "--quiet", "--message", "chore: start")


class LicenseHeaderCheck(unittest.TestCase):
    def test_file_without_a_header(self) -> None:
        # spec: ci-checks / File without a header
        with TempRepo() as repo:
            add_fixture_tree(repo)

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("crates/doyen/src/lease.rs", result.stdout)
            for good_file in ["lib.rs", "run.sh", "index.d.ts", "README.md"]:
                self.assertNotIn(good_file, result.stdout)

    def test_every_file_with_a_header_passes(self) -> None:
        with TempRepo() as repo:
            add_fixture_tree(repo)
            repo.write("crates/doyen/src/lease.rs", f"// {HEADER}\npub fn lease() {{}}\n")
            repo.git("commit", "--quiet", "--all", "--message", "fix: add the header")

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_header_after_the_third_line_fails(self) -> None:
        with TempRepo() as repo:
            add_fixture_tree(repo)
            repo.write("crates/doyen/src/lease.rs", f"//! a\n//! b\n//! c\n// {HEADER}\n")
            repo.git("commit", "--quiet", "--all", "--message", "fix: move the header")

            result = repo.run_script(SCRIPT)

            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn("crates/doyen/src/lease.rs", result.stdout)


if __name__ == "__main__":
    unittest.main()
