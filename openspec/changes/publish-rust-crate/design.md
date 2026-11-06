# Design

## Context

The crate `doyen` 0.0.1 on crates.io and the npm package `@leofdss/doyen` 0.0.2 only reserve
the names. The Redis and MongoDB adapters and their README sections come from the adapter
changes. This change makes the first real release and the process for every later one.

## Goals / Non-Goals

**Goals:**
- A release process that v0.2, v0.3 and v0.4 repeat with no manual step but the owner's
  approval: version number, changelog, tag, GitHub release, publish.
- Standard tools that open source projects already use.

**Non-Goals:**
- The npm and Go publish jobs (`add-node-binding`, `add-go-binding` add them to
  `release.yml`).
- Maintenance branches for old versions (only when a fix must reach an older minor version).

## Decisions

The owner confirmed all the decisions below on 2026-11-05, except the points marked "to
confirm while applying".

- **Branch flow: GitHub Flow.** `main` is always green and protected by a ruleset (pull
  request, green CI, rebase merge only, linear history, no force push). Work happens on
  short branches `<type>/<change-name>`. Alternative: git-flow (`develop`, `release/*`,
  `hotfix/*`); rejected: it is made for software with several versions in use at the same
  time, and its author recommends GitHub Flow for software that is released often.
- **Rebase merge:** each Conventional Commit of a branch reaches `main` as it is, so the
  changelog lists each task group. Alternative: squash merge; rejected, the pull request
  title would replace the commits of the task groups.
- **One version (lockstep) for the crate, the npm packages and the Go module**, one tag
  `v<version>` here and the same tag in `leofdss/doyen-go`. One core ships in three forms: a
  core fix must reach every package anyway. Alternative: one version per package with tags
  `v*`, `node-v*`, `go-v*`; rejected: three changelogs, and a core fix would not bump npm.
- **Version rules (SemVer 2.0.0):** before 1.0, `!` or `feat` → minor, `fix`/`perf` →
  patch (Cargo treats a 0.x minor as a possible break, so users opt in to new features);
  from 1.0, the usual SemVer. The release pull request collects the commits until the owner
  merges it, so each milestone of the plan is one minor version: v0.2 after
  `add-node-binding`, v0.3 after `add-go-binding`, v0.4 after `add-postgresql-adapter`.
- **Tool: release-please** (`googleapis/release-please-action`, pinned by SHA), manifest
  mode:
  - `release-please-config.json`: one package at `.` (every commit counts, the bindings
    too), `release-type: simple`, `bump-minor-pre-major: true`,
    `include-component-in-tag: false` (tag `v0.1.0`), `changelog-sections` that show
    breaking changes, features, fixes and performance and hide the other types,
    `extra-files`: `crates/doyen/Cargo.toml` (TOML, `$.package.version`) and
    `bindings/node/package.json` (JSON, `$.version`). To confirm while applying: how the
    release pull request also updates the `doyen` entry of `Cargo.lock` (a TOML
    `extra-files` entry with a filter, or a workflow step that runs
    `cargo update --workspace` on the release branch).
  - `.release-please-manifest.json` starts at the version of the last placeholder (`0.0.2`,
    or the number of a later republish; the crate, the npm package and the manifest must
    hold the same number, so the version check passes). The first `feat` gives
    `0.1.0`; if not, set `release-as: 0.1.0` for that release.
  - Alternatives: semantic-release (publishes at each merge, with no point to review;
    made for Node.js); release-plz (Rust only); changesets (needs a file written by hand
    for each change); git-cliff and a tag made by hand (manual steps at each release).
- **release-please token:** a GitHub App token (`actions/create-github-app-token`, app
  `doyen-release` with contents and pull requests write, made by the owner). A pull request
  opened with `GITHUB_TOKEN` starts no workflow, so its CI would never run and the ruleset
  would block it.
- **`scripts/check-versions.py`** (Python, standard library), three commands:
  - `same`: the versions in `crates/doyen/Cargo.toml`, `bindings/node/package.json` and
    `.release-please-manifest.json` are equal (CI job `versions` on every pull request).
  - `next`: on the release pull request, the proposed version equals the version that the
    version rules give from the commits since the last tag (`git log <tag>..HEAD`; subject
    `type(scope)!:` or a `BREAKING CHANGE:` footer). It checks release-please against the
    rules of this spec. When the config has `release-as` or a commit has a
    `Release-As: x.y.z` footer, that version is the expected one.
  - `tag <tag>`: the tag equals the versions (release workflow, before any publish).
- **Workflow `release.yml`** on push to `main`: job `release-please` (outputs
  `release_created`, `tag_name`); if a release was created: job `check`
  (`check-versions.py tag`, `cargo package -p doyen`), then job `crates` in the environment
  `release` (`permissions: id-token: write`, `rust-lang/crates-io-auth-action` for a
  short-lived token, `cargo publish -p doyen`).
- **SBOM and provenance (OpenSSF):** the job `check` also runs `cargo cyclonedx` (tool
  `cargo-cyclonedx`, version pinned) and `actions/attest-build-provenance` (pinned;
  `attestations: write`, `id-token: write`) on the `.crate` file, and uploads the `.crate`
  and the SBOM to the GitHub release (`gh release upload`). npm gets its provenance from
  Trusted Publishing; the Go libraries get attestations in `add-go-binding`. The publish jobs live in the same workflow
  because a tag made with a workflow token starts no other workflow.
- **API check:** job `semver` in `ci.yml`, only on the release branch
  (`release-please--branches--main`; to confirm while applying: the branch name of a manifest
  release pull request): `obi1kenobi/cargo-semver-checks-action` (pinned)
  against the last version on crates.io.
- **Trusted Publishing on crates.io:** the owner adds the publisher on crates.io (repository
  `leofdss/doyen`, workflow `release.yml`, environment `release`). It needs a crate that
  already exists: the 0.0.1 placeholder is enough.
- **docs.rs:** `[package.metadata.docs.rs] all-features = true`, and
  `#[cfg_attr(docsrs, doc(cfg(feature = "…")))]` on the adapter items.
- **Use cases** (owner picks 2–3): a scheduler that must run once (cron jobs), a consumer
  that owns a stream or a partition, a cache warm-up or a cleanup job. To confirm (owner).

## Risks / Trade-offs

- [release-please reads a commit in a wrong way] → `check-versions.py next` checks its
  version; the owner reads the release pull request before the merge.
- [A breaking change marked as `feat` or `fix`] → `cargo semver-checks` on the release pull
  request.
- [A changed workflow could publish] → publishing runs only in the environment `release`,
  after the owner's approval; actions are pinned by SHA and checked by zizmor.
- [Trusted Publishing is down] → the owner can still publish by hand with 2FA (written in
  `AGENTS.md`).
- [The placeholder README on crates.io says "work in progress"] → 0.1.0 replaces it; the
  0.0.1 page stays as an old version (it cannot be removed, only yanked: yank it after
  0.1.0).
