# Proposal

## Why

Redis is the first database the owner chose: many teams already run it. It is also the
weakest for safety (asynchronous replication, failover, eviction), so the adapter must
detect data loss, keep the token growing, refuse the setups that break the rules and warn
about the known risks. This change implements the `adapter-contract` for Redis with plain
commands only (no Lua).

Release: v0.1. Depends on: `add-core-election`.

## What Changes

- A Redis adapter behind the cargo feature `redis`: one server, Redis Cluster (found at
  connect) and Sentinel (`redis+sentinel://`), with TLS.
- Keys under `doyen:{group}:` (`meta` hash, `line` sorted set, `alive:<n>` keys, `events`
  channel); atomic join with `WATCH`/`MULTI`/`EXEC`; renew with `SET … XX PX … GET`.
- A generation from the server clock that changes after a restart, a failover, a key loss,
  or when the data went back below what an instance saw.
- Checks at connect: version 6.2+, the eviction policy, the `INFO` fields doyen needs,
  Active-Active; warnings `DOYEN_REDIS_EVICTION`, `DOYEN_REDIS_NO_MIN_REPLICAS`,
  `DOYEN_REDIS_SENTINEL_TIMING`.
- CI jobs with real Redis servers (6.2 and the latest), Sentinel and Cluster.
- README: the Redis section (URL, partition, Sentinel timing, Cluster, memory, ACL, proxies).

## Capabilities

### New Capabilities

- `redis-adapter`: how doyen keeps the line, the places and the generation in Redis, which
  setups it accepts, refuses or warns about, and its Redis-only error and warning codes.

### Modified Capabilities

(none)

## Impact

- `crates/doyen/src/redis/` (feature `redis`), one Redis client crate (chosen in task 1.1).
- `.github/workflows/ci.yml`: new jobs with Redis services; `tests/` helpers with Docker
  Compose files for Sentinel and Cluster.
- `README.md`: Redis section updated from "planned" to the real behaviour.
