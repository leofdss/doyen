# Proposal

## Why

v0.1 is done when the crate is on crates.io, with docs a new user can follow and a release
process that every later version repeats without manual steps, except the owner's approval.
The crate name `doyen` is reserved with a 0.0.1 placeholder; this change replaces it with the
real 0.1.0.

Release: v0.1 (the release itself). Depends on: `add-redis-adapter` and
`add-mongodb-adapter`.

## What Changes

- Crate metadata for 0.1.0: description, docs.rs settings (all features), the feature list.
- README for users: 2–3 real use cases, a Rust quick start for Redis and MongoDB, the status
  and token rules, the database notes (already written by the adapter changes).
- **Branch flow (owner decision 2026-11-05):** GitHub Flow. `main` is always green; work
  happens on short branches and reaches `main` through a pull request with a rebase merge.
- **One version for every package:** the crate, the npm package and the Go module share one
  SemVer version and one tag `v<version>`. A check fails when the versions differ.
- **Automatic version numbers:** release-please reads the Conventional Commits and keeps a
  release pull request with the next version and the new `CHANGELOG.md` section. The owner
  merges it; that makes the tag and the GitHub release.
- **API check:** `cargo semver-checks` on the release pull request fails when the public API
  change needs a bigger version than the one proposed.
- **Trusted Publishing:** CI publishes to crates.io with a short-lived OIDC token, only in the
  GitHub environment `release`, after the owner approves. No stored token.

## Capabilities

### New Capabilities

- `release-process`: how doyen is versioned and released (one version, the version rules,
  the release pull request, the tag, the changelog, the API check, the publish approval)
  and what each published crate version must have (metadata, docs, package check).

### Modified Capabilities

(none)

## Impact

- `crates/doyen/Cargo.toml`, `crates/doyen/README.md`, `README.md`, new `CHANGELOG.md`
  (written by release-please), `release-please-config.json`, `.release-please-manifest.json`,
  `.github/workflows/release.yml`, `scripts/check-versions.py` and its tests, a `versions`
  and a `semver` job in `ci.yml`.
- GitHub (owner): a GitHub App for release-please, the environment `release`, the ruleset
  for `main`. crates.io (owner): the Trusted Publisher for `doyen`.
- crates.io: `doyen` 0.1.0.
- `add-node-binding` and `add-go-binding` add their publish jobs to `release.yml` and follow
  the version and tag rules of `release-process`.
