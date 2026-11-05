# Tasks

Test first: in each group with behaviour, the first task writes the tests (each one names
its scenario: `// spec: redis-adapter / <scenario>`) and shows that they fail against a real
Redis; the next tasks write the code until they pass. Never change a test only to make it
pass.

## 1. Client and URLs

- [ ] 1.1 (owner) Choose the Redis client: check `redis` (redis-rs) and `fred` against the three needs in design.md with a small test program, and record the choice and why in design.md; verify the program runs a `WATCH`/`MULTI`/`EXEC` on one server and on a Cluster, and logs in to a TLS Sentinel with its own login
- [ ] 1.2 Write the URL tests for the scenarios "Sentinel URL" and "Sentinel login" (parsing only); verify they fail (red)
- [ ] 1.3 Add the client behind the feature `redis` and parse the four URL schemes and the Sentinel query; verify the tests pass (green)
- [ ] 1.4 Commit the tests (`test(redis): add the URL tests`), then the code (`feat(redis): add the Redis client and the URL parsing`)

## 2. Operations

- [ ] 2.1 Add the Compose file for one Redis server (6.2 and latest) in `crates/doyen/tests/redis/`; verify it starts with `distrobox-host-exec podman compose up`
- [ ] 2.2 Write the operation tests against that server: the scenarios "Shared Redis", "Keys deleted by hand", "Concurrent joins", "Retry with the same tag", "Key expired", "Another tag", "Member expired", the three "When the Redis generation changes" scenarios, the two "Value of a new Redis generation" scenarios, "Replica in a full sync", "No replicas", "Place is not mine" and "Master leaves"; add the Redis adapter to the conformance suite; give the adapter stub bodies and verify every test fails (red)
- [ ] 2.3 Write join (transaction, n rule, skip a live number, retry by tag, `WAIT`, `PUBLISH`); verify the join tests pass
- [ ] 2.4 Write renew (`SET … XX PX … GET`, tag check, live line, `ZREM` of dead numbers in the next renew); verify the renew and live-line tests pass
- [ ] 2.5 Write the generation rules and value (missing `meta`, `run_id`/`replid`, data went back, `seq` raised, the highest-seen state per group); verify the generation tests pass
- [ ] 2.6 Write `WAIT` (online replicas only, 25% timeout, fewer acks not an error), leave and watch; verify the remaining operation tests and the whole conformance suite pass (green)
- [ ] 2.7 Commit the tests (`test(redis): add the operation tests and the conformance run`), then the code (`feat(redis): add the Redis adapter`)

## 3. Connect checks and warnings

- [ ] 3.1 Write the connect tests: "Redis 6.0", "allkeys-lru", "volatile-lru", "Sentinel without the setting", "Redis Cluster" (no min-replicas warning), "Fast Sentinel", "Default Sentinel", "Proxy hides run_id" (a test double), "User without INFO", "User without scripting", "Managed endpoint moved" (a replica promoted behind the same name); verify they fail (red)
- [ ] 3.2 Write the version, eviction and `INFO` field checks; verify their tests pass
- [ ] 3.3 Write the warnings `DOYEN_REDIS_NO_MIN_REPLICAS` (not in Cluster) and `DOYEN_REDIS_SENTINEL_TIMING`; verify their tests pass
- [ ] 3.4 Map `NOPERM` to `DOYEN_AUTH` with the command name and `READONLY` to a reconnect; verify their tests pass, and that no script command is ever sent
- [ ] 3.5 Find how Redis Active-Active can be detected; write its test first and refuse it, or record in design.md that it cannot be detected and document it (owner decides)
- [ ] 3.6 Commit the tests (`test(redis): add the connect check tests`), then the code (`feat(redis): check the Redis setup at connect`)

## 4. Topologies in CI

- [ ] 4.1 Write the topology tests: "URL of a Cluster node", "Sentinel failover", "Transaction in Cluster", and the failure tests (restart without persistence, `FLUSHDB`, Sentinel failback with old data, partition with `docker network disconnect` at the default and at a 5 s `down-after-milliseconds`, Cluster failover); add the Sentinel and Cluster Compose files; verify the tests that need code not written yet fail
- [ ] 4.2 Add the CI jobs `redis-single` (6.2 and latest), `redis-sentinel` and `redis-cluster` with `DOYEN_TEST_REQUIRE_DB=1`; fix the code until the tests pass; verify the three jobs pass on a pull request and each failure test shows the token still growing and the expected warning
- [ ] 4.3 Commit the tests (`test(redis): run the adapter on one server, Sentinel and Cluster`), then any code fix (`fix(redis): …`)

## 5. Documentation

- [ ] 5.1 Update the README Redis section (URLs, partition, Sentinel timing, Cluster, memory, ACL list with the exact commands, proxies, Active-Active) and the generation exception in *Using the token*; add `openspec/manual-checks.md` lines for the scenarios "README" and "Documented generation exception" (how they were checked); verify every command in the ACL list is one the adapter sends (grep the code)
- [ ] 5.2 Add `crates/doyen/examples/redis.rs` (join, status changes, token, leave on Ctrl-C); verify `cargo run --example redis --features redis` against a local Redis
- [ ] 5.3 Commit as `docs(redis): document the Redis adapter`

## 6. Integration check

- [ ] 6.1 Run `python3 scripts/check-scenario-tests.py --change add-redis-adapter`; verify every scenario of `redis-adapter` is named by a test or by `openspec/manual-checks.md`

## Workflow follow-up

- Archive the change with `/opsx:archive add-redis-adapter` after the pull request is merged.
