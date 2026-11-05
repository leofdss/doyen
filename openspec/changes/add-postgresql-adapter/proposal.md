# Proposal

## Why

PostgreSQL is the fourth release (v0.4) and the safest database of the plan: committed data
does not go back on a restart, and a failover starts a new timeline that doyen can see. Many
teams run PostgreSQL and no Redis. The adapter follows the same `adapter-contract`.

Release: v0.4. Depends on: `add-go-binding` (v0.3 done), because the bindings get the new
`postgresql` type in the same release.

**Two questions are open for v0.4: the generation after a dump/restore (a new cluster starts
again at timeline 1), and how a crash is seen (a session advisory lock is freed only when TCP
keepalive or a server timeout ends the session). design.md proposes answers marked "to
confirm (owner)". Review them before `/opsx:apply` (task 0.1).**

## What Changes

- A PostgreSQL adapter behind the cargo feature `postgresql`: two tables (`doyen_group`,
  `doyen_member`) in a schema chosen by the config, created at connect if missing.
- Places expire by the server clock (`clock_timestamp()` + `ttl`), like Redis and MongoDB;
  no advisory locks.
- Join in one transaction with a row lock on the group row (commit order = number order);
  renew in one statement; wake-ups with `LISTEN`/`NOTIFY`.
- A generation from the server clock that changes when the timeline ID changes, when the
  group row is missing, or when the data went back below what an instance saw.
- The type `postgresql` in the Rust config, the Node.js package and the Go module.
- CI jobs with PostgreSQL (the oldest supported and the latest), and a primary + standby
  failover.

## Capabilities

### New Capabilities

- `postgresql-adapter`: how doyen keeps the line, the places and the generation in
  PostgreSQL, its settings, the setups it accepts or refuses, and the documented limits.

### Modified Capabilities

(none — the bindings take every `type` an adapter defines, see `node-binding` and
`go-binding`)

## Impact

- `crates/doyen/src/postgresql/` (feature `postgresql`, in the default features), dependency
  `tokio-postgres` with a TLS connector.
- `bindings/node` and `bindings/go`: the `postgresql` value and its config struct; new
  releases of the crate, the npm package and the Go module.
- `README.md`: PostgreSQL section, the SQL to create the tables by hand.
