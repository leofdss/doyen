# Design

## Context

The core (`add-core-election`) gives the `Backend` trait, the conformance suite and the
database test switch (`DOYEN_TEST_REDIS_URL`, `DOYEN_TEST_REQUIRE_DB`). The behaviour is in
the `redis-adapter` spec. The Rust Redis client is not chosen yet.

## Goals / Non-Goals

**Goals:**
- Pass the conformance suite on one server, Sentinel and Cluster.
- The fewest round trips: 2 per renew (3 when the generation changes).

**Non-Goals:**
- Lua scripts or Redis functions (owner decision).
- Keyspace notifications (off by default, often blocked by managed services).
- Proxies such as Twemproxy or Envoy: refused when a needed `INFO` field is missing, not
  tested.

## Decisions

### Client library — to confirm (owner)

Task 1.1 checks `redis` (redis-rs) and `fred` against three needs, then records the choice
here:
1. `WATCH` / `MULTI` / `EXEC` on a **dedicated connection** (on a multiplexed connection,
   other commands enter the transaction).
2. Transactions on the keys of one slot in Redis Cluster.
3. Sentinel with TLS and its own login (user and password for the sentinels).

A client that misses one need is rejected. If both pass: the one with fewer dependencies.

### Connections

- One dedicated connection for the transactions of the object (join, renew, leave) and one
  for pub/sub (`watch`). Calls of one object never run in parallel (the core sends one
  renew at a time), so one transaction connection is enough.
- Cluster: the client routes by the `{group}` slot; `INFO` and `TIME` go to the node of that
  slot (the client's "route by key" or `CLUSTER KEYSLOT` + node lookup).

### Operations

- **Join:** `WATCH meta line`; pipeline `HGETALL meta`, `ZRANGE line 0 -1`, `INFO server`,
  `INFO replication`, `TIME` (Redis 6.2 takes one section per `INFO`); compute n and the
  generation check; pipeline `MGET` of the line's `alive` keys, `WATCH alive:<n>`,
  `EXISTS alive:<n>`; `MULTI` `HSET meta …`, `ZADD line n n`, `SET alive:<n> … PX ttl`;
  `EXEC`; `WAIT`; `PUBLISH events`.
- **Renew:** pipeline `WATCH meta`, `SET alive:<n> … XX PX ttl GET`, `ZRANGE line`, `ZREM` of
  the last renew's dead numbers, `HMGET meta format seq gen runid replid`, `INFO server`,
  `INFO replication`, `TIME`; then `MGET` of the `alive` keys + `UNWATCH`; with a generation
  change, `MULTI` `HSET meta …` `EXEC` + `WAIT` instead of `UNWATCH`.
- **Leave:** `WATCH alive:<n>`, `GET`; if my tag: `MULTI` `DEL` + `ZREM` `EXEC`; `PUBLISH`.
- **Data format:** `format` (1) in `meta`, written with the generation. The join reads it
  before its `MULTI` and writes nothing when it is higher. The renew's `SET … XX` runs in the
  same pipeline as the `HMGET`, so the first renew after a format change still refreshes its
  own `alive` key once; then the adapter returns the error and writes nothing more.
- **State kept by the adapter per group:** the highest generation and the highest number it
  saw. They live as long as the process; they are what makes rule 3 of the generation work.

### Connect checks

One pipeline at connect: `INFO server` (version, `run_id`), `INFO replication` (`role`,
`master_replid`, replica lines, `min_slaves_good_slaves`), `INFO memory` (`maxmemory`,
`maxmemory_policy`), `INFO cluster` (`cluster_enabled`). With Sentinel: `SENTINEL MASTER
<name>` (`down-after-milliseconds`) and `CONFIG GET min-replicas-max-lag` (a refused
`CONFIG` → assume 10 s). How Active-Active shows itself in `INFO` or a command: to check in
task 3.3 (to confirm (owner) if it cannot be detected: then document it as not supported).

### Tests and CI

- Compose files in `crates/doyen/tests/redis/`: one server (6.2 and latest), Sentinel
  (1 primary, 2 replicas, 3 sentinels), Cluster (3 primaries, 3 replicas). CI runs them with
  Docker Compose; locally they run on the host with `podman compose` (from the container:
  `distrobox-host-exec podman compose …`, see `distrobox/README.md`).
- CI jobs: `redis-single` (matrix 6.2 / latest, service container), `redis-sentinel` and
  `redis-cluster` (Compose on the runner). All set `DOYEN_TEST_REQUIRE_DB=1`.
- "Old primary back with the same `run_id`": simulated by writing an older `gen` and `seq` to
  `meta` with the current `runid` and `replid`; a real Sentinel failback is also run in the
  Sentinel job (stop the new primary before it syncs back).
- Network partition: `docker network disconnect` of the primary in the Sentinel job.

## Risks / Trade-offs

- [Neither client passes all three needs] → stop and ask the owner; option: a small own
  connection for the transactions on top of the client's protocol layer.
- [`WAIT` reduces the risk but does not stop data loss] → documented; the token covers it.
- [The adapter's "highest seen" is lost on a restart of every instance] → the documented
  exception; the README says how to reset the fence row.
- [Sentinel and Cluster jobs are slow] → they run on pull requests that change
  `src/redis/` or the Compose files, and on `main`.
