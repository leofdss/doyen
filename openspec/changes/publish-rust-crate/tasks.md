# Tasks

## 1. Package and docs

- [ ] 1.1 Set the crate metadata for 0.1.0 (description without "work in progress", docs.rs `all-features`, `doc(cfg)` on the adapter items); verify `cargo doc --all-features` shows the feature labels
- [ ] 1.2 Check the package contents with `cargo package -p doyen --list`; verify it has the sources, the README and both license files, and no test Compose files
- [ ] 1.3 (owner) Pick 2–3 use cases from design.md (or others)
- [ ] 1.4 Write the user README: use cases, the Redis and MongoDB quick starts as doc tests (`#![doc = include_str!("../README.md")]` or a doc test file), links to the status, token and database sections; verify `cargo test --doc --all-features`
- [ ] 1.5 Commit as `docs(release): write the README for the first release`

## 2. Changelog and release workflow

- [ ] 2.1 Add `cliff.toml` and `git-cliff` to the dev image; verify `git cliff --unreleased` lists the `feat` and `fix` commits
- [ ] 2.2 Add `.github/workflows/release.yml` (tag = version check, `cargo package`, GitHub release with `git cliff --latest`); verify it fails on a test tag that does not match the version (on a fork or a throwaway tag, deleted after)
- [ ] 2.3 Commit as `ci(release): add the changelog and the release workflow`

## 3. Release v0.1.0

- [ ] 3.1 Set the version to 0.1.0 and generate `CHANGELOG.md`; verify the CI is green on that commit
- [ ] 3.2 Commit as `chore(release): 0.1.0` and push the tag `v0.1.0`; verify the release workflow passes and the GitHub release has the changelog
- [ ] 3.3 (owner) Run `cargo publish -p doyen`; verify the crates.io page shows 0.1.0 and docs.rs builds it
- [ ] 3.4 (owner) Yank the placeholder `doyen` 0.0.1 (`cargo yank --version 0.0.1 doyen`); verify crates.io marks it as yanked

## Workflow follow-up

- Archive the change with `/opsx:archive publish-rust-crate` after the release.
