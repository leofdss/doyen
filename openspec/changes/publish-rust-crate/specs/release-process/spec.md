# Spec Delta

## Purpose

Defines how doyen is versioned and released, the same way for every version and every
package, and what each published crate version must have.

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

### Requirement: One version for every package
The crate `doyen`, the npm package `@leofdss/doyen` with its platform packages, and the Go
module SHALL have the same version. The CI SHALL fail when the versions in
`crates/doyen/Cargo.toml`, `bindings/node/package.json` and `.release-please-manifest.json`
differ.

#### Scenario: Versions differ
- **WHEN** `bindings/node/package.json` has the version `0.2.0` and the crate has `0.2.1`
- **THEN** the version check fails and names both files

### Requirement: Version from the commits
The next version SHALL come from the Conventional Commits since the last release, with
SemVer. Before 1.0: a breaking change (`!`) or a `feat` gives the next minor version, and only
`fix` or `perf` commits give the next patch version. From 1.0: a breaking change gives the
next major version, a `feat` the next minor, a `fix` or `perf` the next patch. Other types
alone give no release.

#### Scenario: Feature release
- **WHEN** since `v0.1.0` the main branch has `feat(redis): add the Sentinel URL` and `fix(core): renew on time`
- **THEN** the next version is `0.2.0`

#### Scenario: Fix release
- **WHEN** since `v0.1.0` the main branch has only `fix(core): renew on time`
- **THEN** the next version is `0.1.1`

#### Scenario: Breaking change after 1.0
- **WHEN** since `v1.2.0` the main branch has `feat(core)!: rename the status reasons`
- **THEN** the next version is `2.0.0`

#### Scenario: No release
- **WHEN** since `v0.1.0` the main branch has only `docs:`, `ci:` and `test:` commits
- **THEN** there is no next version

### Requirement: Release pull request
release-please SHALL keep one open release pull request with the next version in every
version file and a new `CHANGELOG.md` section: breaking changes, features, fixes and
performance, from the commits since the last release. The CI SHALL fail on that pull request
when its version is not the one the version rules give (or the one set with `Release-As`). Only the owner SHALL merge it.
Merging it SHALL create the tag `v<version>` and a GitHub release with that section.

#### Scenario: New version
- **WHEN** the release pull request for `0.1.0` is open
- **THEN** `CHANGELOG.md` has a `0.1.0` section with the `feat` and `fix` commits since the start

#### Scenario: Wrong proposed version
- **WHEN** the release pull request proposes `0.1.1` and the commits since `v0.1.0` have a `feat`
- **THEN** the version check fails

#### Scenario: Release pull request merged
- **WHEN** the owner merges the release pull request for `0.1.0`
- **THEN** the tag `v0.1.0` and a GitHub release with the `0.1.0` changelog section exist

### Requirement: Tag matches the versions
The release workflow SHALL fail, before it publishes anything, when the tag and the package
versions differ.

#### Scenario: Wrong tag
- **WHEN** the release workflow runs for the tag `v0.1.1` and the crate version is `0.1.0`
- **THEN** the release workflow fails and publishes nothing

### Requirement: API change matches the version
On the release pull request, the CI SHALL run `cargo semver-checks` against the last version
on crates.io, and SHALL fail when the change of the public API needs a bigger version than
the one proposed.

#### Scenario: Hidden breaking change
- **WHEN** the release pull request proposes `0.2.1` and a public function was removed since `0.2.0`
- **THEN** the API check fails and names the function

### Requirement: Package check before publish
Before a release, `cargo package` SHALL pass for the crate, and the CI (all checks and all
database jobs) SHALL be green on the release commit.

#### Scenario: File missing from the package
- **WHEN** the crate's `include` list misses a needed file
- **THEN** `cargo package` fails in the release workflow

### Requirement: SBOM and provenance
The release workflow SHALL add to each GitHub release the packaged crate
(`doyen-<version>.crate`), an SBOM of the crate and its dependencies in CycloneDX format, and
a GitHub artifact attestation (SLSA build provenance) of the packaged crate.

#### Scenario: Release assets
- **WHEN** the release `v0.1.0` is made
- **THEN** its GitHub release has `doyen-0.1.0.crate` and `doyen-0.1.0.cdx.json`, and `gh attestation verify doyen-0.1.0.crate -R leofdss/doyen` passes

### Requirement: Publishing needs the owner's approval
Publishing SHALL run only in the release workflow, in the GitHub environment `release`, which
needs the owner's approval. It SHALL use Trusted Publishing (OIDC): no crates.io or npm token
SHALL be stored in the repository or in its secrets.

#### Scenario: Release day
- **WHEN** the owner approves the environment `release` for `v0.1.0`
- **THEN** `doyen` 0.1.0 is on crates.io, and the repository has no crates.io token secret

#### Scenario: No approval
- **WHEN** the release `v0.1.0` is made and nobody approves the environment `release`
- **THEN** nothing is published, and the tag and the GitHub release stay
