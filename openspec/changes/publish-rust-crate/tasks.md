# Tasks

Test first where there is behaviour: the release checks get their tests before their code,
and the README examples are doc tests (they fail until the code they show exists).

## 1. Package and docs

- [ ] 1.1 Set the crate metadata for 0.1.0 (description without "work in progress", docs.rs `all-features`, `doc(cfg)` on the adapter items); verify `cargo doc --all-features` shows the feature labels
- [ ] 1.2 Check the package contents with `cargo package -p doyen --list`; verify it has the sources, the README and both license files, and no test Compose files
- [ ] 1.3 (owner) Pick 2–3 use cases from design.md (or others)
- [ ] 1.4 Write the README quick starts (Redis and MongoDB) as doc tests first (`#![doc = include_str!("../README.md")]` or a doc test file), each with `// spec: crate-release / Quick start compiles`; verify `cargo test --doc --all-features` runs them, then write the rest of the user README (use cases, links to the status, token and database sections)
- [ ] 1.5 Add `openspec/manual-checks.md` lines for "docs.rs page" and "Missing docs" (the `missing_docs` lint, checked by building a crate item without docs); verify the lint fails the build on a throwaway item, then remove it
- [ ] 1.6 Commit as `docs(release): write the README for the first release`

## 2. Changelog and release checks (test first)

- [ ] 2.1 Write `scripts/tests/test_release_checks.py` for the scenarios "New version" (git-cliff output has a `0.1.0` section with the `feat` and `fix` commits) and "Wrong tag" (tag `v0.1.1` vs version `0.1.0` fails); verify they fail (red)
- [ ] 2.2 Add `cliff.toml`, `git-cliff` in the dev image, and `scripts/check-release-tag.sh <tag>` (compares the tag with `crates/doyen/Cargo.toml`); verify the tests pass (green)
- [ ] 2.3 Add `.github/workflows/release.yml` (tag check script, `cargo package`, GitHub release with `git cliff --latest`); add "File missing from the package" and "Release day" to `openspec/manual-checks.md` after checking them on a throwaway tag (deleted after)
- [ ] 2.4 Commit the tests (`test(release): add the tests for the release checks`), then the code (`ci(release): add the changelog and the release workflow`)

## 3. Release v0.1.0

- [ ] 3.1 Run `python3 scripts/check-scenario-tests.py --change publish-rust-crate`; verify every `crate-release` scenario is named by a test or a manual check
- [ ] 3.2 Set the version to 0.1.0 and generate `CHANGELOG.md`; verify the CI is green on that commit
- [ ] 3.3 Commit as `chore(release): 0.1.0` and push the tag `v0.1.0` (owner's go); verify the release workflow passes and the GitHub release has the changelog
- [ ] 3.4 (owner) Run `cargo publish -p doyen`; verify the crates.io page shows 0.1.0 and docs.rs builds it
- [ ] 3.5 (owner) Yank the placeholder `doyen` 0.0.1 (`cargo yank --version 0.0.1 doyen`); verify crates.io marks it as yanked

## Workflow follow-up

- Archive the change with `/opsx:archive publish-rust-crate` after the release.
