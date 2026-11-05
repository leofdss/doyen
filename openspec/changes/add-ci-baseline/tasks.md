# Tasks

## 1. Commit message check (test first)

- [ ] 1.1 Write `scripts/tests/test_check_commits.py` (`unittest`) for the scenarios "Valid subject", "Invalid subject" and "Merge commit", each test with its comment `spec: ci-checks / <scenario>`; verify `python3 -m unittest discover scripts/tests` fails because the script does not exist yet (red)
- [ ] 1.2 Write `scripts/check-commits.sh <base> <head>` (Conventional Commits regex, skip `Merge` subjects, all-zero base = head only); verify the tests pass (green) and the script passes on `git log` of this repo
- [ ] 1.3 Write `CONTRIBUTING.md` (commit types, scopes, the TDD commit order: tests first, then code; two examples); verify the examples pass the script
- [ ] 1.4 Commit the tests (`test(ci): add the tests for the commit message check`), then the code (`ci: add the commit message check`)

## 2. Scenario-test check (test first)

- [ ] 2.1 Write `scripts/tests/test_check_scenario_tests.py` with a small fixture tree (a spec with three scenarios, a test that names one, `manual-checks.md` with one, a test that names a scenario that does not exist) for the scenarios "Missing test", "Stale test", "Checked by hand" and "Open change"; verify they fail (red)
- [ ] 2.2 Write `scripts/check-scenario-tests.py` (`--change <name>` for an open change); verify the tests pass (green)
- [ ] 2.3 Create `openspec/manual-checks.md` (format and purpose at the top); verify `python3 scripts/check-scenario-tests.py` passes on the repo (no archived specs yet)
- [ ] 2.4 Commit the tests (`test(ci): add the tests for the scenario check`), then the code (`ci: add the scenario check`)

## 3. CI workflow

- [ ] 3.1 Add `.github/workflows/ci.yml` with the jobs `fmt`, `clippy` (`-D warnings`, all features, all targets), `test`, `msrv` (Rust 1.85, `cargo check`), `coverage` (`cargo llvm-cov --lcov`, artifact), `commits`, `scenarios` (the script tests + `check-scenario-tests.py`) and `specs` (`openspec validate --all --strict` + `check-specs.py`); verify a pull request shows the eight jobs green
- [ ] 3.2 Check the failure paths on a throwaway branch (an unformatted file, a clippy warning, a failing test, an API newer than the minimum Rust version, a subject `Added lease`, a duplicate capability); verify each one fails only its own job, write each check in `openspec/manual-checks.md` (scenarios "Pull request opened", "A check fails", "Unformatted file", "Clippy warning", "A test fails", "Code needs a newer Rust", "Report after the tests", "Conflict between changes"), then delete the branch
- [ ] 3.3 Commit as `ci: add the CI checks`

## 4. Integration check

- [ ] 4.1 Run `python3 scripts/check-scenario-tests.py --change add-ci-baseline`; verify every scenario of `ci-checks` is named by a test or by `openspec/manual-checks.md`

## Workflow follow-up

- Archive the change with `/opsx:archive add-ci-baseline` after the pull request is merged.
