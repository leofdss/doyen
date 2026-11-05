# Proposal

## Why

v0.1 is done when the crate is on crates.io, with docs a new user can follow and a release
process that the next versions repeat. The crate name `doyen` is reserved with a 0.0.1
placeholder; this change replaces it with the real 0.1.0.

Release: v0.1 (the release itself). Depends on: `add-redis-adapter` and
`add-mongodb-adapter`.

## What Changes

- Crate metadata for 0.1.0: description, docs.rs settings (all features), the feature list.
- README for users: 2–3 real use cases, a Rust quick start for Redis and MongoDB, the status
  and token rules, the database notes (already written by the adapter changes).
- `CHANGELOG.md` made from the Conventional Commits.
- A release workflow: on a tag `v*`, check the package (`cargo package`), make the GitHub
  release with the changelog section. The owner runs `cargo publish` (2FA).

## Capabilities

### New Capabilities

- `crate-release`: what a published version of the crate must have (metadata, docs,
  changelog, tag, checks) and how a release is made.

### Modified Capabilities

(none)

## Impact

- `crates/doyen/Cargo.toml`, `crates/doyen/README.md`, `README.md`, new `CHANGELOG.md`,
  `cliff.toml`, `.github/workflows/release.yml`.
- crates.io: `doyen` 0.1.0 (the owner publishes).
- The npm and Go releases (`add-node-binding`, `add-go-binding`) reuse the changelog and the
  tag rules.
