# Tasks

## 1. Client and connection

- [ ] 1.1 (owner) Choose the Redis client: check `redis` (redis-rs) and `fred` against the three needs in design.md with a small test program, and record the choice and why in design.md; verify the program runs a `WATCH`/`MULTI`/`EXEC` on one server and on a Cluster, and logs in to a TLS Sentinel with its own login
- [ ] 1.2 Add the client behind the feature `redis`, parse the four URL schemes and the Sentinel query; verify unit tests for each "Redis type and URLs" scenario
- [ ] 1.3 Commit as `build(redis): add the Redis client behind the redis feature`

## 2. Operations

- [ ] 2.1 Write join (transaction, n rule, skip a live number, retry by tag, `WAIT`, `PUBLISH`); verify the "Atomic join" scenarios against a local Redis (`distrobox enter doyen -- cargo test --features redis` with `DOYEN_TEST_REDIS_URL`)
- [ ] 2.2 Write renew (`SET … XX PX … GET`, tag check, live line, `ZREM` of dead numbers in the next renew); verify the "Renew only my own live place" and "Live line and dead numbers" scenarios
- [ ] 2.3 Write the generation rules and value (missing `meta`, `run_id`/`replid`, data went back, `seq` raised, the highest-seen state per group); verify the three "When the Redis generation changes" scenarios and the two "Value of a new Redis generation" scenarios
- [ ] 2.4 Write `WAIT` (online replicas only, 25% timeout, fewer acks not an error); verify the "Wait for replicas" scenarios
- [ ] 2.5 Write leave and watch; verify the "Clean leave" and "Wake-ups through pub/sub" scenarios
- [ ] 2.6 Run the conformance suite on the Redis adapter; verify every rule passes
- [ ] 2.7 Commit as `feat(redis): add the Redis adapter`

## 3. Connect checks and warnings

- [ ] 3.1 Write the version, eviction and `INFO` field checks; verify the "Minimum version", "Eviction policy" and "Needed INFO fields" scenarios (Redis 6.0 container, `allkeys-lru`, `volatile-lru`, a test double that hides `run_id`)
- [ ] 3.2 Write the warnings `DOYEN_REDIS_NO_MIN_REPLICAS` (not in Cluster) and `DOYEN_REDIS_SENTINEL_TIMING`; verify their scenarios
- [ ] 3.3 Find how Redis Active-Active can be detected and refuse it; verify with a test double, or record in design.md that it cannot be detected and document it (owner decides)
- [ ] 3.4 Map `NOPERM` to `DOYEN_AUTH` with the command name and `READONLY` to a reconnect; verify the "Access rules" and "Reconnect on READONLY" scenarios (ACL user with `-@dangerous`; a replica promoted behind the same name)
- [ ] 3.5 Check that no script command is sent; verify a run with an ACL user with `-@scripting`
- [ ] 3.6 Commit as `feat(redis): check the Redis setup at connect`

## 4. Topologies in CI

- [ ] 4.1 Add the Compose files (Sentinel, Cluster) and the CI jobs `redis-single` (6.2 and latest), `redis-sentinel`, `redis-cluster` with `DOYEN_TEST_REQUIRE_DB=1`; verify the three jobs pass on a pull request
- [ ] 4.2 Add the failure tests: restart without persistence, `FLUSHDB`, `meta` deleted, Sentinel failover, Sentinel failback with old data, partition with `docker network disconnect` (default and 5 s `down-after-milliseconds`), Cluster failover, a replica in a full sync; verify each one shows the token still growing and the expected warning
- [ ] 4.3 Commit as `test(redis): run the adapter on one server, Sentinel and Cluster`

## 5. Documentation

- [ ] 5.1 Update the README Redis section (URLs, partition, Sentinel timing, Cluster, memory, ACL list with the exact commands, proxies, Active-Active, the generation exception in "Using the token"); verify the "Documented generation exception" scenario and that every command in the ACL list is one the adapter sends
- [ ] 5.2 Add `crates/doyen/examples/redis.rs` (join, status changes, token, leave on Ctrl-C); verify `cargo run --example redis --features redis` against a local Redis
- [ ] 5.3 Commit as `docs(redis): document the Redis adapter`

## Workflow follow-up

- Archive the change with `/opsx:archive add-redis-adapter` after the pull request is merged.
