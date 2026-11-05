# TODO

The task board of this repo: the order in which the OpenSpec changes in
[`openspec/changes/`](openspec/changes/) are applied, and the few tasks that are not in a
change.

## How to work

- **One change at a time, in the order below.** A change starts only when every change it
  depends on is archived.
- **Branch:** `<type>/<change-name>`, for example `feat/add-core-election`.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) —
  `type(scope): subject`, scopes `core`, `redis`, `mongodb`, `postgresql`, `node`, `go`, `ci`,
  `release`, `docs`. Each task group in a `tasks.md` is one commit and names it.
- **Code goes with its spec:** that commit holds the group's tests, code, spec changes and
  ticks in `tasks.md`. Never code in one commit and its spec in another (see `AGENTS.md`).
- **Tests first (TDD):** the tests are the source of truth. Each task group writes the tests
  for its scenarios, sees them fail, then writes the code until they pass (see `AGENTS.md`).
  A change is done when every scenario of its specs is named by a test (or by
  `openspec/manual-checks.md` when only a person can check it).
- **Code style:** familiar to TypeScript programmers; easy to read, not too short, not too
  long (see `AGENTS.md`, *Code style*).
- **Model:** the skill `model-tiering` says which model and effort each task group needs.
- **Start:** `/opsx:apply <change>` in Claude Code (other agents: the skill
  `openspec-apply-change`). Tick the tasks in the change's `tasks.md` as they are done.
- **Finish:** pull request, CI green, rebase merge (GitHub Flow), then `/opsx:archive
  <change>`: its specs move to `openspec/specs/`. Mark the row below as done. A release is
  made only when the owner merges the release pull request of release-please.
- **New work:** `/opsx:propose <idea>`, then add a row to the table.
- **Before a commit that changes `openspec/`:**

  ```bash
  openspec validate --all --strict
  python3 scripts/check-specs.py
  ```

## Order

| # | Change | Release | Depends on | Capabilities | Owner first | Status |
|---|---|---|---|---|---|---|
| 1 | `add-ci-baseline` | v0.1 | — | `ci-checks` | — | todo |
| 2 | `add-core-election` | v0.1 | 1 | `election-core`, `status-reporting`, `client-config`, `adapter-contract`, `rust-api` | approve the crate layout, the private trait and the hidden hooks (task 0.1) | todo |
| 3 | `add-redis-adapter` | v0.1 | 2 | `redis-adapter` | choose the Redis client (task 1.1) | todo |
| 4 | `add-mongodb-adapter` | v0.1 | 2 | `mongodb-adapter` | — | todo |
| 5 | `add-system-tests` | v0.1 | 3, 4 | `system-tests` | — | todo |
| 6 | `publish-rust-crate` | v0.1 | 5 | `release-process` | pick the use cases (task 1.4); GitHub App, environment, crates.io publisher (task 3.1); approve the release | todo |
| 7 | `add-node-binding` | v0.2 | 6 | `node-binding` | approve the napi-rs design (task 0.1); npm publishers (task 4.7); approve the release | todo |
| 8 | `add-go-binding` | v0.3 | 7 | `go-binding` | approve the Go design (task 0.1); create `leofdss/doyen-go` and its deploy key (task 3.2) | todo |
| 9 | `add-postgresql-adapter` | v0.4 | 8 | `postgresql-adapter` | approve the design (task 0.1); approve the release | todo |

Changes 3 and 4 depend only on 2, but both edit `Cargo.toml`, the CI workflow and the
README: do them one after the other (Redis first, the first database the owner chose).

## Rules that keep the changes free of conflicts

`scripts/check-specs.py` checks these rules.

- **One capability, one change** (the table above). No change writes a delta for a
  capability that another open change adds. A later change reaches an earlier capability
  without a delta: for example `node-binding` and `go-binding` take every `type` an adapter
  defines, so `add-postgresql-adapter` needs no delta on them.
- **Each code has one owner:**

  | Code | Owner capability |
  |---|---|
  | `DOYEN_INVALID_CONFIG` | `client-config` |
  | `DOYEN_AUTH` | `client-config` |
  | `DOYEN_UNSUPPORTED` | `client-config` |
  | `DOYEN_ALREADY_JOINED` | `client-config` |
  | `DOYEN_LEFT` | `client-config` |
  | `DOYEN_CANCELLED` | `go-binding` |
  | `DOYEN_REDIS_EVICTION` | `redis-adapter` |
  | `DOYEN_REDIS_NO_MIN_REPLICAS` | `redis-adapter` |
  | `DOYEN_REDIS_SENTINEL_TIMING` | `redis-adapter` |
  | `DOYEN_MONGODB_ARBITER` | `mongodb-adapter` |

- **Shared values** (the same in every change): `timeoutMs` 10000 and `renewIntervalMs` 2000
  by default; `timeoutMs` ≥ 3 × `renewIntervalMs`; lease margin 10%; one call timeout =
  `renewIntervalMs`; Redis `WAIT` timeout 25% and MongoDB `wtimeout` 90% of the interval;
  Rust 1.85, Redis 6.2, MongoDB 5.0, PostgreSQL 14, Go 1.24 as the minimum versions.
- **Versions and tags:** one SemVer version for the crate, the npm packages and the Go
  module; one tag `v<version>`, here and in `leofdss/doyen-go`. release-please computes the
  version from the commits and makes the tag when the owner merges the release pull request
  (`release-process`).

## Other tasks

- [ ] (owner) Remove the npm version `0.0.0-stage` (left by the first staged publish):
  `npm unpublish @leofdss/doyen@0.0.0-stage` before 2026-11-06 20:56 UTC.
- [ ] (owner, optional) Publish the placeholders again (npm 0.0.3, crate 0.0.2) so they carry
  the license texts.
- [ ] (owner) Turn on private vulnerability reporting, which `SECURITY.md` points to:
  `gh api -X PUT repos/leofdss/doyen/private-vulnerability-reporting` (or Settings →
  Code security).
- [ ] (owner) Decide on the DCO: ask for `Signed-off-by` in every commit, added by the person
  (never by an agent, see `AI_POLICY.md`). The OpenSSF Baseline level 2 asks for it
  (OSPS-LE-01). A simple flow: the agent commits without it; before the pull request, the
  owner runs `git rebase --signoff main` (the human act that certifies). If yes, the commit
  check of `add-ci-baseline` checks it.
- [ ] (owner) After `add-ci-baseline`: protect `main` with a ruleset (pull request and green
  CI before a merge with every job of `ci.yml` required, rebase merge only, linear history,
  no force push, no deletion of `main`), as the OpenSSF SCM guide and Baseline
  (OSPS-AC-03) ask and GitHub Flow needs. Until then, push work as a branch and a pull
  request too, not straight to `main`.
- [ ] (owner) GitHub settings for the OpenSSF Baseline level 1: two-factor login on the
  account (OSPS-AC-01); secret scanning and push protection on (OSPS-BR-07); default
  workflow token read-only (Settings → Actions, OSPS-AC-04).
- [ ] (owner, decision) Response time for security reports (OSPS-VM-01, level 2), for
  `SECURITY.md`. Proposal: an answer within 7 days, a fix or a plan within 90 days.
- [ ] (owner, decision) Code of Conduct: the [Contributor Covenant](https://www.contributor-covenant.org/)
  2.1 (used by Linux Foundation and CNCF projects) as `CODE_OF_CONDUCT.md`, with a contact
  for reports (an address the owner chooses).
- [ ] (owner, decision) Naming: the Linux kernel coding style asks new code not to use
  "master / slave" and proposes "leader / follower" (the usual words for leader election:
  Raft, Kubernetes). doyen's states are `master` / `slave` (owner decision 2026-11-04). The
  cheapest moment to rename is before 0.1.0: no user depends on the names yet.
- [ ] (owner, decision) Go libraries in git: the plan commits prebuilt static libraries to
  `leofdss/doyen-go`. The OpenSSF Baseline level 1 asks not to keep generated binaries in
  version control (OSPS-QA-05), and Fedora asks packages to build against system libraries
  when the project allows it. Proposal: keep the prebuilt module as a documented exception
  (with attestations and SBOMs), and add a path to build from source with a system
  `libdoyen` (pkg-config, a Go build tag) for distributions.
- [ ] (owner, decision) TLS for Fedora: Fedora requires TLS users to follow its system crypto
  policies, which OpenSSL follows and rustls does not. Proposal: rustls stays the default,
  plus an optional cargo feature that uses the system OpenSSL (`native-tls`).
- [ ] (owner) The dev image downloads VS Code "latest" with no checksum: pin a version with
  its SHA-256, or take VS Code out of the image.
- [ ] (owner, optional) Get the OpenSSF Best Practices badge (bestpractices.dev) and add the
  OpenSSF Scorecard workflow; later, an OpenSSF Security Insights file and Fedora Packit
  builds in Copr.
- [ ] (owner, optional) Try the Claude Code sandbox (`/sandbox`) with Podman and Distrobox.
  The Distrobox container is not an isolation boundary (`distrobox/README.md`); the sandbox
  may block Podman.
- [ ] Read the code of mongo-leader and @platformatic/leader: note what doyen does better and
  what it should copy (before change 4).

The other open questions of the planning are tasks inside the changes: the crate layout and
the hidden hooks (change 2, task 0.1), the Redis client (change 3, task 1.1), the use cases
(change 6, task 1.4), the napi-rs design (change 7, task 0.1), the Go design (change 8, task
0.1), the PostgreSQL decisions (change 9, task 0.1). Every open point is marked "to confirm
(owner)" in the design.md of its change.

## Done

(nothing yet)
