# Tasks

## 1. Commit message check

- [ ] 1.1 Write `scripts/check-commits.sh <base> <head>` (Conventional Commits regex, skip `Merge` subjects, all-zero base = head only); verify it passes on `git log` of this repo and fails on a test commit `Added lease`
- [ ] 1.2 Add `CONTRIBUTING.md` with the commit types, the scopes (core, redis, mongodb, postgresql, node, go, ci, release, docs) and two examples; verify the examples pass the script

## 2. CI workflow

- [ ] 2.1 Add `.github/workflows/ci.yml` with the jobs `fmt`, `clippy` (`-D warnings`, all features, all targets), `test`, `msrv` (Rust 1.85, `cargo check`), `coverage` (`cargo llvm-cov --lcov`, artifact) and `commits`; verify a pull request shows the six jobs green
- [ ] 2.2 Check the failure paths on a throwaway branch (an unformatted file, a clippy warning, a subject `Added lease`); verify each one fails only its own job, then delete the branch
- [ ] 2.3 Commit as `ci: add the CI checks and the commit message check`; verify the `commits` job passes on it

## Workflow follow-up

- Archive the change with `/opsx:archive add-ci-baseline` after the pull request is merged.
