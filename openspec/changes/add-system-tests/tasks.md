# Tasks

Test first: the timeline check gets its tests before its code. The system tests check code
that already exists: when one passes at once, break the code on purpose once, see it fail,
and put the code back (design.md, TDD note). Each test names its scenario:
`// spec: system-tests / <scenario>`.

## 1. Test program and timeline check

- [ ] 1.1 Write the unit tests of the timeline check with hand-made timelines for "Timelines checked": no overlap passes, an overlap of 50 ms fails, a token that goes back fails; verify they fail (red), then stage the tests
- [ ] 1.2 Create the crate `crates/system-tests` (`publish = false`) with the timeline parser and the check; verify the tests pass (green)
- [ ] 1.3 Write the binary `member` (environment config, join, `status()` and `token()` every 10 ms, a JSON line on each change with the `CLOCK_BOOTTIME` time, `leave` from stdin, no URL in the output); verify by hand against a local Redis that it writes a timeline and leaves on `leave`
- [ ] 1.4 Commit the group in one commit (tests, code, ticks): `test(core): add the system test program and the timeline check`

## 2. System tests

- [ ] 2.1 Write the system tests "Kill the master", "Short pause", "Long pause" and "Leave hands over at once" for Redis and MongoDB, each one also running the check of "Timelines checked" (skipped without `DOYEN_TEST_SYSTEM=1`); run them against local containers (`distrobox-host-exec podman`); verify they pass, and that each one fails once with the code broken on purpose
- [ ] 2.2 If a test finds a bug, fix it in the core or the adapter with its own test first (in that capability), never by loosening the system test; verify every test passes
- [ ] 2.3 Commit the group in one commit (tests, ticks): `test(core): add the system tests for Redis and MongoDB`

## 3. CI and README

- [ ] 3.1 Add the job `system` to `ci.yml` (the Redis service and the MongoDB replica set container of the `add-mongodb-adapter` jobs, `DOYEN_TEST_SYSTEM=1`, `DOYEN_TEST_REQUIRE_DB=1`, the pause commands through `docker`, the timelines as an artifact; on pull requests that change `crates/` and on a nightly `schedule`); verify it passes, then write "Timeline" and "Pull request" in `openspec/manual-checks.md`
- [ ] 3.2 Write the README section *How doyen is tested* (unit tests with a fake clock, the conformance suite, the database failure tests, the system tests and their timelines), and add the command of the system tests to the *Commands* table of `AGENTS.md`
- [ ] 3.3 Commit the group in one commit (workflow, README, manual checks, ticks): `ci: run the system tests`

## 4. Integration check

- [ ] 4.1 Run `python3 scripts/check-scenario-tests.py --change add-system-tests`; verify every `system-tests` scenario is named by a test or by `openspec/manual-checks.md`

## Workflow follow-up

- Archive the change with `/opsx:archive add-system-tests` after the pull request is merged.
