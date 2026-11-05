# Tasks

## 1. Crate setup

- [ ] 1.1 Add the dependencies from design.md and the empty features `redis` and `mongodb` (both default) to `crates/doyen/Cargo.toml`; verify `distrobox enter doyen -- cargo build --all-features` and the CI `msrv` job pass
- [ ] 1.2 Create the modules `config`, `status`, `token`, `error`, `backend`, `engine`, `clock` with `#![deny(missing_docs)]`; verify `cargo clippy --all-features -- -D warnings` passes
- [ ] 1.3 Commit as `build(core): set up the crate modules and features`

## 2. Config and errors

- [ ] 2.1 Write `Error`, `ErrorCode` (the five `client-config` codes) and `Display` with the code first; verify unit tests for `code()` and the message prefix
- [ ] 2.2 Write `DatabaseConfig`, the config structs with `new(url, group)`, the defaults (10000, 2000, host name) and the shared checks (group, numbers, 3 × rule, URL scheme hook per adapter); verify one unit test per scenario of `client-config` and the "Config type" scenario of `rust-api`
- [ ] 2.3 Make a config for a database whose feature is off fail with `DOYEN_UNSUPPORTED`; verify with `cargo test --no-default-features`
- [ ] 2.4 Write the rustdoc of the config and error types with one example each; verify `cargo test --doc`
- [ ] 2.5 Commit as `feat(core): add the config, its checks and the error codes`

## 3. Status and token types

- [ ] 3.1 Write `Status`, `OfflineReason` (camelCase `Display`) and `Token` (`Ord` generation first); verify unit tests for the `rust-api` "Status type" and "Token type" scenarios and the `status-reporting` string values
- [ ] 3.2 Commit as `feat(core): add the status and token types`

## 4. Clock

- [ ] 4.1 Write the `Clock` trait, the real clock (Linux `CLOCK_BOOTTIME` through `rustix`) and the fake clock on `tokio::time::Instant`; verify a test with the real clock and a real 2 s sleep, and a paused-time test with the fake clock
- [ ] 4.2 Choose the suspend-aware clock for macOS (and note Windows) and record it in design.md; verify `cargo check --target aarch64-apple-darwin` (or the CI macOS job when it exists)
- [ ] 4.3 Commit as `feat(core): add a clock that counts suspend time`

## 5. Adapter contract, in-memory adapter, conformance suite

- [ ] 5.1 Write the `Backend` trait, `Membership`, `View` and `Warning`; verify it compiles behind `pub(crate)`
- [ ] 5.2 Write the in-memory adapter on the fake clock (counter, line, places with expiry, tags, wake-ups, a switch to "lose all data"); verify its own unit tests
- [ ] 5.3 Write the conformance suite: one test per requirement of `adapter-contract` (number identity, tag, atomic join with concurrent joins, counter back, idempotent join, generation never back, live line, dead membership, cancel, connect writes no group data); verify the in-memory adapter passes all of it
- [ ] 5.4 Add the database test switch (`DOYEN_TEST_*_URL`, `DOYEN_TEST_REQUIRE_DB`); verify a run without variables skips and a run with `DOYEN_TEST_REQUIRE_DB=1` and no URL fails
- [ ] 5.5 Commit as `feat(core): add the adapter contract and its conformance suite`

## 6. Engine

- [ ] 6.1 Write the driver: `connect` once, join with retries and backoff, tag kept until a place, join lease from the first send; verify the `election-core` scenarios "Join takes a place…", "Join retries…", "Lease of a join" and "Membership tag" with the in-memory adapter
- [ ] 6.2 Write the lease and the fixed-rate schedule with the call timeout; verify the "Local lease", "Renews at a fixed rate" and "Takeover times with the defaults" scenarios (7–9 s offline, 8–12 s takeover with three instances, a 5 s problem changes nothing)
- [ ] 6.3 Write the snapshot, `status()`, `token()` at call time, lost place → new membership, after-join retries with the last error; verify the "Master is the oldest live member", "Fencing token", "Tokens of later masters are higher", "Status and token at the moment of the call" and "After join the core never gives up" scenarios
- [ ] 6.4 Write `leave` (lease first, cancel a pending join, one call timeout, no network error) and the one-life rules; verify the "Leave stops first…" and "One object, one life" scenarios
- [ ] 6.5 Write the subscribers (one unbounded queue per stream, every change, error alone is not a change, `left` last) and the warnings through `tracing`; verify every scenario of `status-reporting` and the `rust-api` "Status changes as a stream" scenarios
- [ ] 6.6 Make a dropped `join` future go back to `notJoined` and keep the tag; verify the `rust-api` "Cancel a join by dropping it" scenario
- [ ] 6.7 Check the Tokio runtime rule; verify the `rust-api` "Tokio runtime" scenario (runtime shut down → not master after the lease)
- [ ] 6.8 Commit as `feat(core): add the election engine`

## 7. Binding hooks

- [ ] 7.1 Write `__binding::set_liveness` and `__binding::warnings`; verify a test where a false liveness check skips the renews and the status turns offline `connectionLost`
- [ ] 7.2 Commit as `feat(core): add the hidden hooks for the bindings`

## 8. Integration check

- [ ] 8.1 Run the whole suite with coverage (`cargo llvm-cov --all-features`); verify the CI passes and the coverage report shows the engine and the conformance suite covered

## Workflow follow-up

- Archive the change with `/opsx:archive add-core-election` after the pull request is merged.
