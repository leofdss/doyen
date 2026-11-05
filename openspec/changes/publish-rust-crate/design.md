# Design

## Context

The crate `doyen` 0.0.1 on crates.io only reserves the name. The Redis and MongoDB adapters
and their README sections come from the adapter changes. This change makes the first real
release.

## Goals / Non-Goals

**Goals:**
- A release process that v0.2 and v0.3 repeat (tag, changelog, GitHub release).

**Non-Goals:**
- Automatic publishing from CI (the owner publishes, with 2FA).
- The npm package and the Go module (their own changes).

## Decisions

- **Changelog tool: `git-cliff`** (reads Conventional Commits; in the Arch repos, so it can go
  in the dev image). Config in `cliff.toml`: groups Features, Fixes, Documentation, Other;
  breaking changes (`!`) first. Alternative: `release-please`; rejected, it opens pull
  requests and needs more GitHub setup than a one-person project needs.
- **Release workflow `release.yml`** on `push: tags: ['v*']`: check that the tag matches the
  version in `crates/doyen/Cargo.toml`, run `cargo package -p doyen`, make the GitHub release
  with `git-cliff --latest` as the body. It needs no secret but `GITHUB_TOKEN`.
- **docs.rs:** `[package.metadata.docs.rs] all-features = true`, and
  `#[cfg_attr(docsrs, doc(cfg(feature = "…")))]` on the adapter items.
- **Use cases** (owner picks 2–3): a scheduler that must run once (cron jobs), a consumer
  that owns a stream or a partition, a cache warm-up or a cleanup job. To confirm (owner).
- **Version:** 0.1.0. Until 1.0, a minor version can break the API (Cargo rules for 0.x);
  the changelog marks it.

## Risks / Trade-offs

- [The placeholder README on crates.io says "work in progress"] → 0.1.0 replaces it; the
  0.0.1 page stays as an old version (it cannot be removed, only yanked: yank it after 0.1.0).
- [A forgotten file in the package] → `cargo package` in the workflow catches a build that
  fails; the `include` list is checked by hand in task 1.2.
