# Tasks

Test first: in each group with behaviour, the first task writes the tests (each one names
its scenario: `// spec: <capability> / <scenario>`) and shows that they fail; the next tasks
write the code until they pass. Never change a test only to make it pass.

## 0. Decision

- [ ] 0.1 (owner) Review the "to confirm (owner)" decisions in design.md (one crate with one cargo feature per adapter and every adapter in the default features, the private adapter trait, the hidden binding hooks, Windows later) and approve them or ask for changes

## 1. Crate setup (no behaviour)

- [ ] 1.1 Add the dependencies from design.md and the empty features `redis` and `mongodb` (both default) to `crates/doyen/Cargo.toml`; verify `distrobox enter doyen -- cargo build --all-features` and the CI `msrv` job pass
- [ ] 1.2 Create the modules `config`, `status`, `token`, `error`, `backend`, `engine`, `clock` as empty files with `#![deny(missing_docs)]`; verify `cargo clippy --all-features -- -D warnings` passes
- [ ] 1.3 Commit as `build(core): set up the crate modules and features`

## 2. Config and errors

- [ ] 2.1 Write the tests: every scenario of `client-config`, and the `rust-api` scenarios "Build a config", "Match a code" and "Feature off"; declare only the public types they need (no logic) and verify the tests fail (red)
- [ ] 2.2 Write `Error`, `ErrorCode` (the five codes), the config structs with `new(url, group)`, the defaults, the shared checks, the feature-off rule and the URL redaction (`***` in errors, logs and `Debug`); verify the tests pass (green), also with `cargo test --no-default-features --features redis`
- [ ] 2.3 Write the rustdoc of the config and error types with one example each; verify `cargo test --doc`
- [ ] 2.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add the config, its checks and the error codes`

## 3. Status and token types

- [ ] 3.1 Write the tests for the `rust-api` scenarios "Display a reason" and "Compare tokens" and the `status-reporting` scenario "Log a reason"; verify they fail (red)
- [ ] 3.2 Write `Status`, `OfflineReason` (camelCase `Display`) and `Token` (`Ord`, generation first); verify the tests pass (green)
- [ ] 3.3 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add the status and token types`

## 4. Clock

- [ ] 4.1 Write the clock tests: the real clock against a real 2 s sleep, the fake clock in paused Tokio time; verify they fail (red)
- [ ] 4.2 Write the `Clock` trait, the real clock (Linux `CLOCK_BOOTTIME` through `rustix`) and the fake clock on `tokio::time::Instant`; verify the tests pass (green)
- [ ] 4.3 Choose the suspend-aware clock for macOS and record it in design.md; add a `compile_error!` for other targets that names the supported platforms; add the CI jobs `test-macos` (macOS arm64 runner, `cargo test --all-features`, no database) and `check-linux-arm64` (`cargo check --all-features --target aarch64-unknown-linux-gnu`); verify both jobs pass and a Windows `cargo check` fails with the message, then write "Build on macOS" and "Build on Windows" in `openspec/manual-checks.md`
- [ ] 4.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add a clock that counts suspend time`

## 5. Adapter contract and conformance suite

- [ ] 5.1 Write the `Backend` trait, `Membership`, `View` and `Warning` (types only); verify it compiles behind `pub(crate)`
- [ ] 5.2 Write the conformance suite, generic over `Backend`: one test per scenario of `adapter-contract`; run it against a stub adapter whose methods return an error and verify every test fails (red)
- [ ] 5.3 Write the in-memory adapter on the fake clock (counter, line, places with expiry, tags, wake-ups, a switch to end the watch stream, the data format, a switch to "lose all data" and a test hook that writes a newer format); verify the whole conformance suite passes (green)
- [ ] 5.4 Write the tests of the database test switch (no `DOYEN_TEST_*_URL` → skipped; `DOYEN_TEST_REQUIRE_DB=1` and no URL → failure), see them fail, then write the switch; verify they pass
- [ ] 5.5 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add the in-memory adapter and the database test switch`

## 6. Engine

- [ ] 6.1 Write the engine tests with the in-memory adapter and the fake clock: every scenario of `election-core` and `status-reporting` (except "Log a reason", done in group 3), and the `rust-api` scenarios "New object", "Clone", "Typical use", "Two streams", "End of the stream", "Join with a timeout", "Runtime shut down", "No Tokio runtime", "Object dropped while master" and "Clone still alive"; give the engine stub bodies and verify the tests compile and fail (red)
- [ ] 6.2 Write the join path (`connect` once, retries with backoff, tag kept until a place, join lease from the first send); verify the join, retry, lease-of-a-join and tag tests pass
- [ ] 6.3 Write the lease and the fixed-rate schedule with the call timeout; verify the "Local lease", "Renews at a fixed rate" and "Takeover times with the defaults" tests pass (7–9 s offline, 8–12 s takeover with three instances, a 5 s problem changes nothing)
- [ ] 6.4 Write the snapshot, `status()` / `token()` at call time, the lost place → new membership, the after-join retries with the last error, the reopen of an ended watch stream; verify the master rule, token, status-at-call-time and never-give-up tests pass
- [ ] 6.5 Write `leave` (lease first, cancel a pending join, one call timeout, no network error) and the one-life rules; verify their tests pass
- [ ] 6.6 Write the subscribers (one unbounded queue per stream, error alone is not a change, `left` last) the warnings and the status-change and failed-call events through `tracing`; verify every `status-reporting` test and the stream tests pass
- [ ] 6.7 Make a dropped `join` future go back to `notJoined` with the tag kept, make the drop of the last handle stop the driver, and check the Tokio runtime rule; verify the "Join with a timeout", "Object dropped while master", "Clone still alive", "Runtime shut down" and "No Tokio runtime" tests pass
- [ ] 6.8 Refactor with every test green; verify `cargo clippy -- -D warnings` and that no test changed
- [ ] 6.9 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add the election engine`

## 7. Binding hooks

- [ ] 7.1 Write the tests: a false liveness check skips the renews and the status turns offline `connectionLost`; the warnings stream gets each connect warning once; verify they fail (red)
- [ ] 7.2 Write `__binding::set_liveness` and `__binding::warnings`; verify the tests pass (green)
- [ ] 7.3 Commit the group in one commit (tests, code, spec changes, ticks): `feat(core): add the hidden hooks for the bindings`

## 8. Integration check

- [ ] 8.1 Run `python3 scripts/check-scenario-tests.py --change add-core-election`; verify every scenario of the five capabilities is named by a test
- [ ] 8.2 Remove the placeholder case ("no coverage data found" → empty report) from the `coverage` job of `ci.yml`: the crate has code now; run the whole suite with coverage (`cargo llvm-cov --all-features`); verify the CI passes and the report shows the engine and the conformance suite covered

## Workflow follow-up

- Archive the change with `/opsx:archive add-core-election` after the pull request is merged.
