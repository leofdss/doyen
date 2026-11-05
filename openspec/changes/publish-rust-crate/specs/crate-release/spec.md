# Spec Delta

## Purpose

Defines what each published version of the crate doyen must have (metadata, docs, changelog,
tag, package checks) and how a release is made, so every version is released the same way.

## ADDED Requirements

### Requirement: Crate metadata
Each published version SHALL have a description, the license `MIT OR Apache-2.0`, the
repository, `rust-version`, keywords, categories and a README. docs.rs SHALL build it with all
features.

#### Scenario: docs.rs page
- **WHEN** a version is published
- **THEN** its docs.rs page shows the Redis and MongoDB config types

### Requirement: Docs a new user can follow
The README SHALL have 2 or 3 real use cases, a quick start for each database with code that
compiles, and links to the status, token and database sections. Every public item SHALL have
rustdoc.

#### Scenario: Quick start compiles
- **WHEN** the CI runs the doc tests
- **THEN** the README quick start code compiles

#### Scenario: Missing docs
- **WHEN** a public item has no rustdoc
- **THEN** the build fails (`missing_docs` is denied)

### Requirement: Changelog from the commits
`CHANGELOG.md` SHALL have one section per version, made from the Conventional Commits since
the last tag (features, fixes, breaking changes first).

#### Scenario: New version
- **WHEN** the release `v0.1.0` is prepared
- **THEN** `CHANGELOG.md` has a `0.1.0` section with the `feat` and `fix` commits since the start

### Requirement: Tag and release
Each version SHALL have a Git tag `v<version>` that matches the crate version, and a GitHub
release with its changelog section. The release workflow SHALL fail when the tag and the crate
version differ.

#### Scenario: Wrong tag
- **WHEN** the tag `v0.1.1` is pushed and the crate version is `0.1.0`
- **THEN** the release workflow fails and no GitHub release is made

### Requirement: Package check before publish
Before a release, `cargo package` SHALL pass for the crate, and the CI (all checks and all
database jobs) SHALL be green on the tagged commit.

#### Scenario: File missing from the package
- **WHEN** the crate's `include` list misses a needed file
- **THEN** `cargo package` fails in the release workflow

### Requirement: The owner publishes
Publishing to crates.io SHALL be done by the owner (`cargo publish`, login with 2FA), after the
release workflow passed. No CI job SHALL hold a crates.io token.

#### Scenario: Release day
- **WHEN** the release workflow passed on `v0.1.0`
- **THEN** the owner runs `cargo publish -p doyen` and the version appears on crates.io
