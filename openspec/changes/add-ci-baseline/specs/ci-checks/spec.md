# Spec Delta

## Purpose

Defines the checks that every push to main and every pull request must pass, so that the
code always builds, is formatted, has no lint warnings and keeps the commit rules.

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

### Requirement: Code is committed with its spec
The CI SHALL fail when a commit of a branch `<type>/<change>`, where
`openspec/changes/<change>/` exists, changes a file that is outside `openspec/` and is not a
Markdown file, and changes no file in `openspec/changes/<change>/`. It SHALL skip merge
commits. On any other branch the check SHALL pass.

#### Scenario: Code with its spec
- **WHEN** a commit on the branch `feat/add-core-election` changes `crates/doyen/src/lease.rs` and `openspec/changes/add-core-election/tasks.md`
- **THEN** the commit-spec check passes

#### Scenario: Code without its spec
- **WHEN** a commit on the branch `feat/add-core-election` changes `crates/doyen/src/lease.rs` and no file in `openspec/changes/add-core-election/`
- **THEN** the commit-spec check fails and names that commit

#### Scenario: Docs only
- **WHEN** a commit on the branch `feat/add-core-election` changes only `README.md`
- **THEN** the commit-spec check passes

#### Scenario: Branch without a change
- **WHEN** a commit on the branch `fix/typo` changes code and `openspec/changes/typo/` does not exist
- **THEN** the commit-spec check passes

### Requirement: Every scenario has a test
The CI SHALL fail when a scenario of a spec in `openspec/specs/` is named by no test (a
comment `spec: <capability> / <scenario>`) and by no line of `openspec/manual-checks.md`, and
when a test names a scenario that no spec has. The same check SHALL run on the delta specs of
one open change with `--change <name>`.

#### Scenario: Missing test
- **WHEN** a spec in `openspec/specs/` has the scenario "Master dies" and no test names it
- **THEN** the check fails and names the capability and the scenario

#### Scenario: Stale test
- **WHEN** a test names `election-core / Old name` and no spec has that scenario
- **THEN** the check fails and names the test file

#### Scenario: Checked by hand
- **WHEN** a CI scenario is listed in `openspec/manual-checks.md` with how and when it was checked
- **THEN** the check accepts it without a test

#### Scenario: Open change
- **WHEN** the check runs with `--change add-core-election` before the archive
- **THEN** it checks the scenarios of that change's delta specs

### Requirement: Specs are checked
The CI SHALL run `openspec validate --all --strict` and `scripts/check-specs.py`, and SHALL
fail when either reports an error.

#### Scenario: Conflict between changes
- **WHEN** a commit adds a capability that another open change already has
- **THEN** the specs check fails
