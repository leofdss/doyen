# Design

## Context

The design notes had a sketch: a counter row for the join, a session advisory lock as "I am
alive", `LISTEN`/`NOTIFY`, the timeline ID as the generation, and two open questions for
v0.4 (a dump/restore resets the timeline; a crash after a partition is seen only through TCP
keepalive). This design answers them. The behaviour is in the `postgresql-adapter` spec.

## Goals / Non-Goals

**Goals:**
- The same lease model as Redis and MongoDB: the database's clock decides who is alive.
- Renew in one round trip (two when the generation changes).

**Non-Goals:**
- Synchronous replication settings (a server setting the library cannot turn on;
  documented as "safer").
- MySQL or other SQL databases.

## Decisions

### Places as rows with a server-time expiry — to confirm (owner)

`doyen_member(grp, number, tag, id, expires_at)`; a place is alive while `expires_at >
clock_timestamp()`. Alternative from the sketch: a session advisory lock. Rejected: it needs
one pinned connection per instance, does not work through poolers in transaction mode, and
after a partition the lock stays until TCP keepalive or a server timeout ends the session
(2 h by default): no master for that time. With rows, a crash and a partition are both seen
after `timeoutMs`, like the other adapters, and the conformance suite is the same.

### Tables — to confirm (owner)

```sql
CREATE TABLE IF NOT EXISTS doyen_group (
  grp      text PRIMARY KEY,
  seq      bigint NOT NULL,
  gen      bigint NOT NULL,
  timeline integer NOT NULL
);
CREATE TABLE IF NOT EXISTS doyen_member (
  grp        text   NOT NULL REFERENCES doyen_group (grp) ON DELETE CASCADE,
  number     bigint NOT NULL,
  tag        uuid   NOT NULL,
  id         text   NOT NULL,
  expires_at timestamptz NOT NULL,
  PRIMARY KEY (grp, number)
);
```

In the config schema (`schema`, default `public`). The 128-bit tag fits `uuid`.

### Operations

- **Join:** `BEGIN`; `INSERT INTO doyen_group … ON CONFLICT DO NOTHING`; `SELECT … FOR
  UPDATE` the group row (the lock orders the joins, so commit order = number order; a plain
  sequence does not); generation check; delete dead members; tag check; insert; update
  `seq`; `NOTIFY`; `COMMIT`. Sent as one pipeline where the driver allows it.
- **Renew:** one statement with CTEs: update my row (number, tag, not expired), delete dead
  rows, read the live line, the group row and the timeline (`pg_walfile_name(
  pg_current_wal_lsn())`, first 8 hex digits). A generation change is a second statement
  (`UPDATE doyen_group … WHERE gen = $old`).
- **Leave:** `DELETE … WHERE grp = $1 AND number = $2 AND tag = $3`; `NOTIFY doyen, $grp`.
- **Watch:** a dedicated connection with `LISTEN doyen`; the payload is the group.
- **Generation:** the same rules as Redis, with the timeline in place of `run_id`/`replid`:
  this answers the dump/restore question (timeline 1 in the new cluster ≠ the stored one →
  a new, higher generation from the server clock).

### Driver — to confirm (owner)

`tokio-postgres` with `tokio-postgres-rustls` (TLS). Alternative: `sqlx`; rejected, heavier
(macros, its own pool) for five statements.

### Versions — to confirm (owner)

PostgreSQL 14 or newer (the supported versions in 2026). CI: 14 and the latest.

### Bindings

The Node.js `DatabaseType` gets `postgresql` and a config shape with `schema`; the Go module
gets a `PostgresConfig`. These follow the `node-binding` and `go-binding` rules (every
adapter type), so no spec delta is needed there.

## Risks / Trade-offs

- [The timeline functions are not allowed on a managed service] → `DOYEN_UNSUPPORTED` naming
  the function; task 4.1 checks Amazon RDS, Google Cloud SQL and Azure before the release.
- [Asynchronous replication loses the last commits in a failover] → the timeline changes, so
  the generation grows; two masters for a moment, covered by the token; documented.
- [One hot row per group] → every join locks it; renews do not lock it. Fine for tens of
  instances.
- [`clock_timestamp()` vs `now()`] → `now()` is the transaction start; `clock_timestamp()` is
  the real time of the statement, so a long transaction cannot make a place live longer.
