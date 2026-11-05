# Tasks

Test first where there is behaviour: the version checks get their tests before their code,
and the README examples are doc tests (they fail until the code they show exists).

## 1. Package and docs

- [ ] 1.1 Set the crate metadata for 0.1.0 (description without "work in progress", docs.rs `all-features`, `doc(cfg)` on the adapter items); verify `cargo doc --all-features` shows the feature labels
- [ ] 1.2 Check the package contents with `cargo package -p doyen --list`; verify it has the sources, the README and both license files, and no test Compose files
- [ ] 1.3 (owner) Pick 2–3 use cases from design.md (or others)
- [ ] 1.4 Write the README quick starts (Redis and MongoDB) as doc tests first (`#![doc = include_str!("../README.md")]` or a doc test file), each with `// spec: release-process / Quick start compiles`; verify `cargo test --doc --all-features` runs them, then write the rest of the user README (use cases, links to the status, token and database sections)
- [ ] 1.5 Add `openspec/manual-checks.md` lines for "docs.rs page" and "Missing docs" (the `missing_docs` lint, checked by building a crate item without docs); verify the lint fails the build on a throwaway item, then remove it
- [ ] 1.6 Commit as `docs(release): write the README for the first release`

## 2. Version checks (test first)

- [ ] 2.1 Write `scripts/tests/test_check_versions.py` (temporary git repos with tags and commits) for the scenarios "Versions differ", "Feature release", "Fix release", "Breaking change after 1.0", "No release", "Wrong proposed version" and "Wrong tag"; verify they fail (red), then stage the tests
- [ ] 2.2 Write `scripts/check-versions.py` with the commands `same`, `next` and `tag`; set the crate version to `0.0.2` (same as the npm placeholder); verify the tests pass (green) and `check-versions.py same` passes on the repo
- [ ] 2.3 Add the job `versions` to `ci.yml` (`check-versions.py same` on every pull request, and `check-versions.py next` on the release branch); verify zizmor passes
- [ ] 2.4 Commit the group in one commit (tests, code, spec changes, ticks): `ci(release): check the versions of every package`

## 3. Release automation

- [ ] 3.1 (owner) Create the GitHub App `doyen-release` (contents and pull requests: write), install it on the repo and add the secrets `RELEASE_APP_ID` and `RELEASE_APP_KEY`; create the environment `release` with the owner as the required reviewer; add the Trusted Publisher for `doyen` on crates.io (repository `leofdss/doyen`, workflow `release.yml`, environment `release`); set the ruleset of `main` (pull request, every job of `ci.yml` as a required check, rebase merge only, linear history, no force push)
- [ ] 3.2 Add `release-please-config.json`, `.release-please-manifest.json` (`0.0.2`) and `.github/workflows/release.yml` (release-please with the app token; on a release: `check-versions.py tag`, `cargo package -p doyen`, the SBOM, the attestation and the upload of the release assets, then the job `crates` in the environment `release` with `crates-io-auth-action` and `cargo publish -p doyen`); settle the `Cargo.lock` update (design.md); verify zizmor passes and the release pull request appears with the CI checks running
- [ ] 3.3 Add the job `semver` to `ci.yml` (only on the release branch, `cargo-semver-checks` against crates.io); check "Hidden breaking change" on a throwaway branch (`cargo semver-checks --baseline-version 0.0.1` after removing a public item from a copy), write it in `openspec/manual-checks.md`, then delete the branch
- [ ] 3.4 Commit the group in one commit (workflows, config, manual checks, ticks): `ci(release): automate the releases with release-please`

## 4. Release v0.1.0

- [ ] 4.1 Run `python3 scripts/check-scenario-tests.py --change publish-rust-crate`; verify every `release-process` scenario is named by a test or by `openspec/manual-checks.md`
- [ ] 4.2 Read the release pull request: version `0.1.0` (else set `release-as: 0.1.0`), the `CHANGELOG.md` section, the jobs `versions` and `semver` green; write "New version" in `openspec/manual-checks.md`
- [ ] 4.3 (owner) Merge the release pull request; verify the tag `v0.1.0` and the GitHub release; write "Release pull request merged" in `openspec/manual-checks.md`
- [ ] 4.4 (owner) Wait before the approval and check that nothing is published ("No approval"), then approve the environment `release`; verify the crates.io page shows 0.1.0 and docs.rs builds it, run `gh attestation verify` on the `.crate` asset, and write "File missing from the package", "No approval", "Release day" and "Release assets" in `openspec/manual-checks.md`
- [ ] 4.5 (owner) Yank the placeholder `doyen` 0.0.1 (`cargo yank --version 0.0.1 doyen`); verify crates.io marks it as yanked

## Workflow follow-up

- Archive the change with `/opsx:archive publish-rust-crate` after the release.
