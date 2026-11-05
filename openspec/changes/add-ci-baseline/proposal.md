# Proposal

## Why

The repo has no CI for code yet: only the workflow that builds the dev image. Before the
first line of the core, every push and pull request must be checked the same way, so the
later changes only add their own tests. Commits already follow Conventional Commits by hand;
a check keeps it true when more people (or agents) commit.

Release: v0.1 (groundwork). Depends on: nothing.

## What Changes

- A GitHub Actions workflow `ci.yml` that runs on every push to `main` and every pull
  request: format check, lint (warnings are errors), tests, a build with the minimum Rust
  version (1.85), and a coverage report.
- A commit message check: every commit of a pull request (and every push to `main`) must
  follow Conventional Commits.
- A scenario check: every spec scenario is named by a test (`spec: <capability> /
  <scenario>`), or by a line of `openspec/manual-checks.md` for the CI scenarios checked by
  hand. Tests are the source of truth (TDD), so a scenario without a test is not done.
- A specs check: `openspec validate --all --strict` and `scripts/check-specs.py` in CI.
- A short `CONTRIBUTING.md` with the commit types, the scopes and the TDD commit order.

## Capabilities

### New Capabilities

- `ci-checks`: the checks that every push and pull request must pass: build, lint, tests,
  the commit message rule, the scenario-test check and the specs check.

### Modified Capabilities

(none)

## Impact

- New: `.github/workflows/ci.yml`, `scripts/check-commits.sh`,
  `scripts/check-scenario-tests.py`, their tests in `scripts/tests/`,
  `openspec/manual-checks.md`, `CONTRIBUTING.md`.
- No change to the crate or the npm package. The dev image workflow stays as it is.
- Later changes add their own CI jobs (databases, bindings) next to these checks.
