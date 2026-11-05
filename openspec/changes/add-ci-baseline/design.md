# Design

## Context

Today the repo has one workflow (`build-dev-image.yml`) and a placeholder crate. The dev
image has Rust, `cargo-llvm-cov` and LLVM, but GitHub runners do not use it: CI installs its
own toolchains, so the CI does not depend on the image.

## Goals / Non-Goals

**Goals:**
- Fast checks on every push and pull request, with no secrets.
- A commit message check with no third-party action.

**Non-Goals:**
- Database services, Node.js and Go jobs: each later change adds its own.
- Release or publish workflows (`publish-rust-crate` and the binding changes add them).

## Decisions

- **Jobs:** `fmt`, `clippy`, `test`, `msrv`, `coverage`, `commits`, `scenarios`, `specs`,
  `workflows`, `deps`, in parallel. One job per check gives a clear failure name. Alternative: one job with many steps; rejected, a
  failure hides the next checks.
- **Filters at job level only:** a job that runs only for some changes (the database jobs,
  `system`, `semver`) lives in `ci.yml` with an `if:` on the output of a small `changes` job,
  never with a workflow-level `paths:` filter. A skipped job counts as passed, so every job
  can be a required check of the `main` ruleset, and no pull request waits for a check that
  never starts.
- **Toolchains:** `dtolnay/rust-toolchain` (stable, and `1.85` for `msrv`), plus
  `Swatinem/rust-cache`. Alternative: the dev image as a container job; rejected for now,
  the image is large and built weekly.
- **MSRV job:** `cargo check --workspace --all-features` with 1.85 (a build check is enough;
  tests run on stable).
- **Coverage:** `taiki-e/install-action` installs `cargo-llvm-cov`; the report
  (`lcov.info`) is uploaded as an artifact. No external coverage service.
- **Commit check:** `scripts/check-commits.sh <base> <head>` reads `git log --format=%H %s`
  and checks each subject with one regular expression. The same script runs locally. For a
  push to `main` the range is the push (`before..after`); for a pull request, base..head.
- **Branch names** are not checked; `TODO.md` asks for `<type>/<change-name>`.
- **Commit-spec check: `scripts/check-commit-specs.py <base> <head> <branch>`** (Python,
  standard library). It takes the change name after the first `/` of the branch (in CI:
  `github.head_ref` for a pull request, `github.ref_name` for a push). If
  `openspec/changes/<change>/` does not exist, it passes. Else, for each commit that is not a
  merge, it reads `git show --name-only`: a file outside `openspec/` that is not `*.md` is
  code, and then some file in `openspec/changes/<change>/` must change too (the ticks are
  enough). It runs in the `commits` job, after `check-commits.sh`.
- **Scenario check: `scripts/check-scenario-tests.py`** (Python, standard library). It reads
  the scenarios of `openspec/specs/**/spec.md` (or of one open change with `--change`), and
  searches `crates/`, `bindings/` and `scripts/tests/` for comments
  `spec: <capability> / <scenario>` (any comment style: `//`, `#`). A scenario that CI checks
  by hand (for example "Unformatted file") goes in `openspec/manual-checks.md` as
  `- <capability> / <scenario>: <how and when it was checked>`. Scenario names must be unique
  inside one capability (`scripts/check-specs.py` checks it), so a test can name them.
- **Specs job:** installs `@fission-ai/openspec@1.15.0` (the version in the dev image) and runs
  `openspec validate --all --strict` and `python3 scripts/check-specs.py`.
- **Workflow security:** every action is pinned to a full commit SHA with the version in a
  comment (Dependabot updates them); `permissions: contents: read` at the top of `ci.yml`;
  `${{ }}` values reach `run:` scripts through `env:`; `actions/checkout` with
  `persist-credentials: false`. The `workflows` job runs `zizmor` (installed with `pipx`,
  version pinned) with `.github/zizmor.yml` asking for SHA pins on every action (to confirm
  while applying: the exact config key of the `unpinned-uses` audit).
- **Dependency check:** the `deps` job runs `cargo deny check` with `deny.toml`: advisories
  from RustSec, licenses allowed = MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
  BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib (add one only when a real dependency
  needs it), sources = crates.io only. Installed with `taiki-e/install-action`.
- **TDD for the scripts too:** the tests of `check-commits.sh`, `check-commit-specs.py` and
  `check-scenario-tests.py` (in `scripts/tests/`, run with `python3 -m unittest`) come before
  the scripts.

## Risks / Trade-offs

- [A change branch with another name (not `<type>/<change>`) skips the commit-spec check] →
  the review checks the branch name; the check cannot guess the change.
- [The check sees that the change's folder changed, not that the right spec changed] → it
  catches the common mistake (code committed alone); the review checks the content.
- [The first push of a new branch has no `before` commit] → the script checks only the
  pushed head commit when `before` is all zeros.
- [Clippy gets new lints with each stable Rust] → a new warning can fail a run with no code
  change; fix it in a `chore(ci)` or `style` commit.
