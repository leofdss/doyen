# Design

## Context

`add-core-election` tests the engine with an in-memory adapter and a fake clock.
`add-redis-adapter` and `add-mongodb-adapter` run the conformance suite and failure tests
(restart, failover, partition) on real databases. This change adds the view from outside:
whole processes, real clocks, real timeouts, real faults.

## Goals / Non-Goals

**Goals:**
- Check the timing and safety promises of `election-core` end to end, on both v0.1
  databases.
- Keep a timeline of every run, to read when a test fails.

**Non-Goals:**
- Network partitions, Sentinel and Cluster (the Redis adapter tests have them).
- Several hosts, clock jumps, a Jepsen-style random fault search (maybe later).
- Node.js and Go (their own changes test their bindings).

## Decisions

- **A crate `crates/system-tests`** (workspace member, `publish = false`) with a binary
  `member` and the tests in `tests/`. The tests start the binary through
  `CARGO_BIN_EXE_member`. Alternative: an example in `crates/doyen/examples/`; rejected,
  the examples are for users.
- **Member program:** reads `DOYEN_TEST_SYSTEM_TYPE`, `DOYEN_TEST_SYSTEM_URL` and
  `DOYEN_TEST_SYSTEM_GROUP`, joins with the default settings, reads `status()` and `token()` every 10 ms, and writes a
  JSON line to stdout when one of them changes: `{"t": <CLOCK_BOOTTIME ms>, "state", "reason",
  "number", "generation"}`. A line `leave` on stdin calls `leave()`. The URL is never
  printed (`client-config`, "Secrets are never shown").
- **Shared clock:** all members run on the same host, so their `CLOCK_BOOTTIME` times can be
  compared. The tests run on Linux only.
- **Faults:** kill = `SIGKILL` of the member process; pause = `docker pause` /
  `docker unpause` of the database container (locally `distrobox-host-exec podman pause`),
  given by `DOYEN_TEST_SYSTEM_PAUSE` and `DOYEN_TEST_SYSTEM_UNPAUSE` (in CI: the Redis service
  by `${{ job.services.redis.id }}`, the MongoDB container by its name). A paused database
  keeps its TCP connections but answers nothing, like a hung server. Its expiry uses the wall clock, so
  after a long pause every place has expired.
- **Members start one after the other** (1 s apart), so their numbers are 1, 2 and 3 and
  member 1 is master before each fault.
- **Timeline check:** a master period goes from the first sample with `state=master` to the
  first sample without it. Two periods of different members may overlap by at most the
  sampling step (10 ms). The tokens of the periods, in time order, must grow.
- **Switches:** the tests run only with `DOYEN_TEST_SYSTEM=1` and the URL of each database
  (`DOYEN_TEST_REDIS_URL`, `DOYEN_TEST_MONGODB_URL`), and use the database test switch of
  `add-core-election` (`DOYEN_TEST_REQUIRE_DB=1` makes a missing URL a failure).
- **Time:** about 40 s per scenario, 4 scenarios × 2 databases: about 5 minutes. The CI job
  `system` runs on pull requests that change `crates/` (a job-level filter, see the
  `add-ci-baseline` design), and every night (`schedule`), so a slow CI runner is found
  early.
- **TDD note:** these tests check code that already exists, so they may pass at once. Each
  test is then checked by breaking the code on purpose once (for example a lease margin of
  0, or no `PUBLISH` on leave), seeing the test fail, and putting the code back.

## Risks / Trade-offs

- [A slow CI runner makes a timing test flaky] → the limits are the spec's (8–12 s,
  7–9 s), with their own margins; the timeline shows where the time went. A flaky test is a
  bug to fix, never a test to loosen without the owner.
- [`docker pause` is not a real network problem] → it is close to a hung server; the
  partition tests of the Redis adapter cover the network.
