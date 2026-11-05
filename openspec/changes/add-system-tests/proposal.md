# Proposal

## Why

The core tests use an in-memory adapter and a fake clock. The adapter tests check each
operation against a real database. No test runs whole instances, with real clocks, against
a real database while things fail. But that is the promise of doyen: one master at a time,
a takeover in 8 to 12 s, and no change during a short database problem. v0.1 should prove
it from the outside before it is published.

Release: v0.1. Depends on: `add-redis-adapter` and `add-mongodb-adapter`.

## What Changes

- A test crate `crates/system-tests` (not published): a member program that joins a group
  and writes its status and token on a timeline, and a check that reads the timelines.
- System tests for Redis and MongoDB with three members: the master is killed, the
  database pauses for a short and a long time, the master leaves. Every test also checks
  that there is one master at a time and that the tokens grow.
- A CI job `system` on every pull request that changes the crates, and every night. The
  timelines are kept as CI artifacts.
- README: a short section on how doyen is tested.

## Capabilities

### New Capabilities

- `system-tests`: the tests that run several real instances against a real database, with
  real clocks and real faults, and check the promises of `election-core` from the outside.

### Modified Capabilities

(none)

## Impact

- New crate `crates/system-tests` (workspace member, `publish = false`), a CI job `system`,
  a README section.
- No change to the public API or to the published crate.
