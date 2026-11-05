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

- **Jobs:** `fmt`, `clippy`, `test`, `msrv`, `coverage`, `commits`, in parallel. One job per
  check gives a clear failure name. Alternative: one job with many steps; rejected, a
  failure hides the next checks.
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

## Risks / Trade-offs

- [The first push of a new branch has no `before` commit] → the script checks only the
  pushed head commit when `before` is all zeros.
- [Clippy gets new lints with each stable Rust] → a new warning can fail a run with no code
  change; fix it in a `chore(ci)` or `style` commit.
