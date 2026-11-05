# Proposal

## Why

Every database adapter and every language binding needs the same rule: who is master, when
an instance must stop, what status it shows and which token it gives. That rule must live
once, in the Rust core, and be tested without a database. This change builds the core and
its Rust API, so the Redis and MongoDB adapters only implement a small contract.

Release: v0.1. Depends on: `add-ci-baseline`.

## What Changes

- The election core: the seniority line (master = oldest live member), the local lease, the
  renew schedule, the membership tag, the fencing token, `join` and `leave`.
- The status: `master`, `slave`, `offline` with a reason (`notJoined`, `connecting`,
  `connectionLost`, `reconnecting`, `left`) and the last database error; one notification
  per change, in order; warnings once at connect.
- The config shared by all databases (`type`, `url`, `group`, `timeoutMs`,
  `renewIntervalMs`, `instanceName`), its defaults, its checks and the error codes. A
  password in a URL never shows in an error, a log or a status.
- The adapter contract (`connect`, `join`, `renew`, `leave`, `watch`), an in-memory adapter
  with a fake clock, and one conformance test suite that every adapter must pass.
- The Rust public API: `Doyen::new()`, `join`, `status`, `status_changes`, `token`, `leave`.
- The placeholder crate `doyen` 0.0.1 becomes the real crate (not published yet:
  `publish-rust-crate` does it).

## Capabilities

### New Capabilities

- `election-core`: the seniority line, the master rule, the local lease, renews, the
  membership tag, the token, `join` and `leave`.
- `status-reporting`: states, offline reasons and their transitions, the `error` field,
  status notifications and the warnings channel.
- `client-config`: the config fields shared by all databases, defaults, checks, the
  database types and the `DOYEN_*` error codes.
- `adapter-contract`: the rules every database adapter must follow, the in-memory adapter and
  the conformance suite.
- `rust-api`: the public Rust types and methods of the crate `doyen`.

### Modified Capabilities

(none)

## Impact

- `crates/doyen`: new modules (core, config, status, token, adapter trait, in-memory adapter,
  conformance suite). New dependencies: `tokio`, `futures`, `tracing`, `async-trait`, `rand`,
  `thiserror`, `url` (exact list in design.md).
- The adapters (`add-redis-adapter`, `add-mongodb-adapter`, later `add-postgresql-adapter`)
  and the bindings (`add-node-binding`, `add-go-binding`) build on this change.
- No database is needed to test this change.
