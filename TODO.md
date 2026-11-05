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
- **Finish:** pull request, CI green, merge, then `/opsx:archive <change>`: its specs move to
  `openspec/specs/`. Mark the row below as done.
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
| 2 | `add-core-election` | v0.1 | 1 | `election-core`, `status-reporting`, `client-config`, `adapter-contract`, `rust-api` | — | todo |
| 3 | `add-redis-adapter` | v0.1 | 2 | `redis-adapter` | choose the Redis client (task 1.1) | todo |
| 4 | `add-mongodb-adapter` | v0.1 | 2 | `mongodb-adapter` | — | todo |
| 5 | `publish-rust-crate` | v0.1 | 3, 4 | `crate-release` | pick the use cases (task 1.3); publish | todo |
| 6 | `add-node-binding` | v0.2 | 5 | `node-binding` | approve the napi-rs design (task 0.1); publish | todo |
| 7 | `add-go-binding` | v0.3 | 6 | `go-binding` | create `leofdss/doyen-go` and its deploy key (task 3.2) | todo |
| 8 | `add-postgresql-adapter` | v0.4 | 7 | `postgresql-adapter` | approve the design (task 0.1); publish | todo |

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
- **Tags:** the crate `v<version>`, the npm package `node-v<version>`, the Go module
  `go-v<version>` here and `v<version>` in `leofdss/doyen-go`.

## Other tasks

- [ ] (owner) Remove the npm version `0.0.0-stage` (left by the first staged publish):
  `npm unpublish @leofdss/doyen@0.0.0-stage` before 2026-11-06 20:56 UTC.
- [ ] (owner, optional) Publish the placeholders again (npm 0.0.3, crate 0.0.2) so they carry
  the license texts.
- [ ] Read the code of mongo-leader and @platformatic/leader: note what doyen does better and
  what it should copy (before change 4).

The other open questions of the planning are tasks inside the changes: the Redis client
(change 3, task 1.1), the napi-rs design (change 6, task 0.1), the use cases (change 5, task
1.3), the PostgreSQL decisions (change 8, task 0.1).

## Done

(nothing yet)
