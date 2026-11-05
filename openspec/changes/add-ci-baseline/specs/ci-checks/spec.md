# Spec Delta

## Purpose

Defines the checks that every push to main and every pull request must pass, so that the
code always builds, is formatted, has no lint warnings and keeps the commit message rule.

## ADDED Requirements

### Requirement: CI runs on every push and pull request
The CI SHALL run all checks of this capability on every push to `main` and on every pull
request to `main`. A failed check SHALL mark the run as failed.

#### Scenario: Pull request opened
- **WHEN** a pull request to `main` is opened or updated
- **THEN** the CI runs the format, lint, test, minimum Rust version and commit message checks

#### Scenario: A check fails
- **WHEN** one check fails
- **THEN** the whole CI run is marked as failed

### Requirement: Code format is checked
The CI SHALL fail when any Rust file is not formatted with the project's `rustfmt` settings.

#### Scenario: Unformatted file
- **WHEN** a commit adds a Rust file that `cargo fmt --check` would change
- **THEN** the format check fails

### Requirement: Lint warnings are errors
The CI SHALL run `clippy` on the whole workspace with all features and all targets, and SHALL
fail on any warning.

#### Scenario: Clippy warning
- **WHEN** the code has a clippy warning
- **THEN** the lint check fails

### Requirement: Tests run on stable Rust
The CI SHALL run the workspace tests with all features on the latest stable Rust.

#### Scenario: A test fails
- **WHEN** one test fails
- **THEN** the test check fails

### Requirement: The minimum Rust version builds
The CI SHALL build the workspace with the Rust version in `rust-version` (1.85) and SHALL fail
when it does not build.

#### Scenario: Code needs a newer Rust
- **WHEN** a commit uses a language or library feature that Rust 1.85 does not have
- **THEN** the minimum Rust version check fails

### Requirement: Coverage is reported
The CI SHALL produce a line coverage report of the workspace tests and keep it as a build
artifact. Coverage SHALL NOT fail the run.

#### Scenario: Report after the tests
- **WHEN** the tests pass
- **THEN** the run has a coverage report artifact

### Requirement: Commit messages follow Conventional Commits
The CI SHALL fail when a commit subject of the pull request (or of a push to `main`) does not
match `type(scope)!: subject`, where `type` is one of feat, fix, docs, style, refactor, perf,
test, build, ci, chore, revert, and `(scope)` and `!` are optional.

#### Scenario: Valid subject
- **WHEN** a pull request has the commit `feat(core): add the local lease`
- **THEN** the commit message check passes

#### Scenario: Invalid subject
- **WHEN** a pull request has the commit `Added lease`
- **THEN** the commit message check fails and names that commit

#### Scenario: Merge commit
- **WHEN** a pull request contains a merge commit made by Git (subject starts with `Merge`)
- **THEN** the check skips it
