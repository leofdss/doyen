# Design

## Context

The crate `crates/doyen` is a placeholder (0.0.1, only a doc comment). The behaviour is
fixed by the specs of this change (`election-core`, `status-reporting`, `client-config`,
`adapter-contract`, `rust-api`); this document says how to build it. The adapters come in
later changes, so the core is built and tested with an in-memory adapter only.

## Goals / Non-Goals

**Goals:**
- One state machine for every database and every language binding.
- Every timing rule testable with a fake clock, in milliseconds of test time.
- A small internal hook for the bindings, without growing the public API.

**Non-Goals:**
- A public adapter trait for third-party crates (not in v0.1; the trait is `pub(crate)`).
- Any database code (the adapter changes add it).
- Publishing to crates.io (`publish-rust-crate`).

## Decisions

### Crate layout (approved in task 0.1)

One crate, `doyen`:

| Module | What |
|---|---|
| `config` | `DatabaseConfig`, the config structs, defaults, shared checks |
| `status` | `Status`, `OfflineReason`, `Display` |
| `token` | `Token` (`Ord`: generation, then number) |
| `error` | `Error`, `ErrorCode` (`#[non_exhaustive]`) |
| `backend` | `pub(crate) trait Backend`, `Membership`, `View`, `Warning` |
| `engine` | the driver task, the lease, the schedule, the snapshot, the subscribers |
| `clock` | `Clock` trait, the real clock, the fake clock (tests) |
| `memory` | in-memory adapter (`#[cfg(test)]`) |
| `conformance` | the conformance suite, generic over `TestDatabase` (`#[cfg(test)]`) |
| `redis`, `mongodb` | added by the adapter changes, behind the cargo features |

Alternative: one crate per adapter (`doyen-redis`, …). Rejected: users then match versions
by hand, and the config enum would need a plugin registry. Cargo features give the same
choice with one dependency line.

To confirm (owner), task 0.1: one crate with one cargo feature per adapter and every adapter
in the default features (`rust-api`, "Cargo features per adapter"), and the adapter trait
`pub(crate)` (no public trait for third-party adapters in v0.1).

The conformance suite stays inside the crate (`#[cfg(test)]`), so the trait can stay
private. The database tests read `DOYEN_TEST_REDIS_URL`, `DOYEN_TEST_MONGODB_URL` (later
`DOYEN_TEST_POSTGRESQL_URL`); without the variable they are skipped, unless
`DOYEN_TEST_REQUIRE_DB=1` (set in CI), which makes a missing URL a failure.

### Trait

```rust
#[async_trait]
pub(crate) trait Backend: Send + Sync + 'static {
    async fn connect(&self) -> Result<Vec<Warning>>;
    async fn join(&self, group: &str, id: &str, tag: u128, ttl: Duration) -> Result<(u64, View)>;
    async fn renew(&self, group: &str, me: &Membership, ttl: Duration) -> Result<Option<View>>;
    async fn leave(&self, group: &str, me: &Membership) -> Result<()>;
    async fn watch(&self, group: &str) -> Result<Option<BoxStream<'static, ()>>>;
}
```

`async_trait` because the core holds an `Arc<dyn Backend>` (the database is chosen at run
time from the config). Native `async fn` in traits is not object-safe.

The types around it (task group 5):
- `Membership { number: u64, tag: u128 }`, `View { generation: u64, line: Vec<u64> }` (the
  live numbers in increasing order; no master field: the core decides) and
  `Warning { code: String, message: String }`.
- `Result<T>` is `Result<T, BackendError>`. `BackendError` has two variants: `Network(message)`
  (a network error, a timeout, a failover: the core retries it) and `Refused(Error)` (a
  `DOYEN_AUTH` or `DOYEN_UNSUPPORTED` error: it ends a `join` that has no place yet; after the
  first place the core retries it too). The crate `Error` has no network code, because a
  network error never reaches the user as an error, so the adapter needs this second type.
- `DATA_FORMAT` (1) and `newer_format_error(stored)` live in `backend`, so every adapter
  gives the same message: "the group data has format 2, but this version of doyen knows only
  format 1: …". `join`, `renew` and `leave` all return it and write nothing for a newer format.

### Conformance suite

- `conformance` holds one async function per rule, generic over a `TestDatabase` trait (the
  database plus the test hooks of its adapter): `new_adapter` (one more app instance),
  `group_name` (unique on a shared database), `lose_all_data`, `delete_generation_record`,
  `set_counter`, `write_format` and `adapter_on_old_server` (an adapter on a server below the
  minimum version, and that version as text). Each adapter change implements it for its
  database.
- An adapter runs every rule with one line, `conformance_tests!(start)`: the macro writes one
  `#[tokio::test]` per rule. `start` is an async function that returns `Some(database)`, or
  `None` to skip. A macro, because a test function cannot be generic and each adapter must
  run the same list.
- The rules wait with `tokio::time::sleep`: in paused time (in-memory adapter) the clock
  moves at once; on a real database it is a real wait. The places use a `ttl` of 3 s.
- A stub adapter whose calls all fail runs the suite with `conformance_tests!(start,
  every_rule_fails)`: each test passes only when its rule fails, so a broken adapter fails
  the run (`adapter-contract`, "New adapter").
- "App clock is wrong": a test cannot move the clock of the app machine. The adapter calls
  carry no app time (only `ttl`), so the test checks that a place lives `ttl` by the database
  time. "First join after a restart" joins again with the instance that ran before the data
  loss: a fresh process can meet the documented exception of an adapter.
- One more test, outside the scenarios: a join and a leave wake every watcher (when the
  adapter has push). The engine needs it for the takeover at once after a leave.

### In-memory adapter

`memory` (`#[cfg(test)]`): `MemoryDatabase` plays the server (shared by the adapters of one
test, on the fake clock); `MemoryBackend` is one adapter. Each call runs under one lock and
never waits inside it, so it is atomic and safe to cancel.
- Group data: `format`, the generation record, the number counter, the places (number →
  tag and expiry in database time). A dead place is removed by the next call.
- New number: max(counter, highest place) + 1. A live place with the same tag is returned
  again.
- Generation: the stored one, unless the record is missing or lower than the highest
  generation this adapter returned; then max(highest returned + 1, database time in ms),
  written to the record. The highest returned lives in the adapter, like in Redis.
- Switches and hooks: `lose_all_data`, `delete_generation_record`, `set_counter`,
  `write_format`, `end_watch_streams` (for "Watch stream ends"), and a server version (1.4
  minimum) for "Old server version".
- Switches of the engine tests (task group 6), on each `MemoryBackend` (the network of one
  app instance): a `Fault` per call kind (`NoAnswer`: the call waits until the fault is
  removed, then runs, unless the call timeout ends it first; `LostAnswer`: the call runs and
  its answer never comes; `NetworkError`, `Refused`, `SlowAnswer`, `Panic`), the warnings
  that `connect` gives, and a log of the calls with their send times. `MemoryDatabase` adds
  `live_numbers` and `stored_generation` to read the data.

### Database test switch

`conformance::database_url(variable)` reads the URL of a test database. The variable is not
set (or is empty) → `None`, the start function returns `None` and the test passes without
running (it prints `skipped: <variable> is not set`, never the URL). `DOYEN_TEST_REQUIRE_DB`
set to exactly `1` and no URL → panic, so the test fails. An adapter change uses it in its
start function: `let url = database_url("DOYEN_TEST_REDIS_URL")?;`, with real time (no
`tokio::time::pause()`), and its CI jobs set `DOYEN_TEST_REQUIRE_DB=1`. The decision itself is
a pure function, tested without changing the environment of the test process.

### Driver task, snapshot and subscribers

- The `join` future itself runs `connect` and the join attempts, so dropping it cancels
  them with no task to stop; a guard then sets `notJoined` (chosen in task group 6). After
  the first place, `join` starts one Tokio task per object (the driver). It owns the
  schedule and the watch stream, renews, and joins again after a lost place. The public
  object holds an `Arc<Shared>` with one `std::sync::Mutex<State>`:
  - the snapshot: the phase (`notJoined`, `connecting`, holding a place, `reconnecting`,
    `left`), my place (membership, last line, generation, lease end), the tag of the next
    place with the send time of its first attempt, and the last error. `status()` and
    `token()` lock it for a moment and compute the answer with `clock.now()`; they never
    `await`.
  - the subscribers: one `mpsc::UnboundedSender<Status>` per `status_changes()` call.
    Unbounded: a change is never lost or merged (`status-reporting`).
  - One lock for both (not two), so the notifications follow the order of the changes, and
    nothing comes after `left`.
- The status is computed after each answer and at each lease deadline (also while a call
  is stuck); a notification goes out only when the state or the reason changed. `status()`
  and `token()` also send a change they see first (the lease ended a moment ago and the
  driver did not run yet), so a subscriber never lags behind what `status()` returned.
- The last error is the message of the last failed call since the last call that worked;
  every offline status carries it.
- A panic in the driver ends it; the lease then ends on its own (safe).
- The public object holds an `Arc<Handle>`; clones share it. `Handle` keeps a
  `oneshot::Sender`, and the driver waits on its receiver next to its timers. When the last
  handle drops, the sender drops, and the driver stops at once (no renew, no `leave`: a
  `Drop` cannot wait for the database). The driver keeps only `Arc<Shared>`, never the
  handle, so it cannot keep the object alive. The Go binding's free uses the same path.
- Data format: the in-memory adapter and every adapter store `format` (1) with the group
  data; the conformance suite writes format 2 through a test hook of each adapter.
- Log events (`tracing`, target `doyen`): info for each status change (fields `group`,
  `state`, `reason` when offline, `number` and `generation` while there is a place), debug
  for each failed database call (fields `group`, `call`, `error`), warn for each connect
  warning (fields `group`, `code`). No event holds a URL.
- Until the adapter changes, `join` with a config whose feature is on fails with
  `DOYEN_UNSUPPORTED` ("the redis adapter is not in this version of doyen yet"). The engine
  tests give the in-memory adapter and the clock through a test-only constructor
  (`Doyen::with_backend`); that path runs every config check but the feature check
  (`validate_settings`), so the engine tests also run with `--no-default-features`.

### Clock and schedule

- `Clock::now()` is a monotonic time that **counts suspend**: Linux `CLOCK_BOOTTIME`
  (`rustix::time::clock_gettime`). macOS: `CLOCK_MONOTONIC` (the same call with
  `ClockId::Monotonic`). On macOS it is the clock that counts sleep (it follows
  `mach_continuous_time`), while `CLOCK_UPTIME_RAW`, which Rust's `Instant` uses there, does
  not. Chosen at implementation; a macOS runner checks the build (CI job `test-macos`). The
  supported targets are Linux x86_64 and aarch64 and macOS aarch64. Windows is not supported
  in v0.1, like in the Node.js and Go builds: a `compile_error!` names the supported
  platforms. To confirm (owner): Windows later (`QueryInterruptTime` counts sleep).
- `join` checks `tokio::runtime::Handle::try_current()` first: outside Tokio it returns
  `DOYEN_UNSUPPORTED` instead of the panic of `tokio::spawn`.
- The lease is checked with `Clock::now()`. The timers use Tokio (`interval_at` from the join,
  period `renewIntervalMs`, `MissedTickBehavior::Skip`): fixed rate, and a missed tick after a
  suspend does not send a burst. Each call is wrapped in `tokio::time::timeout(renewInterval)`.
- Tests: the fake clock reads `tokio::time::Instant` and the engine tests run with
  `#[tokio::test(start_paused = true)]`, and the memory and conformance tests call
  `tokio::time::pause()` in their start function, so the lease and the timers move together. One test also checks
  the real clock against a real sleep.

### Join, retries and reconnect

- Each `join` call runs `connect` once before its first join attempt. Network errors are
  retried like a join; refusals end `join` with the error.
- Then `join` opens the watch stream (one call timeout; when it fails, the driver opens it
  again later), before the first join attempt: a wake-up just after the join (the master
  leaves) is not lost. When the driver starts, it skips the wake-ups that wait in the
  stream: they came from my own join, or from a change that the join answer already showed.
- Retries (connecting and reconnecting): backoff from 100 ms, × 2, at most
  `renewIntervalMs` (jitter ± 20% on top, so one delay can be 1.2 × the interval). A retry
  keeps the tag.
- The join lease counts from the send time of the first attempt with this tag; the
  snapshot keeps that time with the tag. When less than one renew interval of it is left,
  `join` renews at once before it finishes; when that renew fails, `join` tries again with
  the same tag, so `join` always finishes with a valid lease (master or slave).
- A call timeout is a network error ("the database did not answer within N ms").
- A lost place (renew `None`, or my number not in the line) → new tag, reason
  `reconnecting`, join again. The schedule keeps its rate from the first join.
- The watch stream is a small state machine in the driver (opening, open, waiting to open
  again, no push); it opens again inside the driver's wait, so the renew timer goes on
  meanwhile.

### Leave

`leave` sets "left" in the snapshot first (so `status()` is offline `left` and `token()` is
`None` at once), sends the last notification `left` and closes the queues in the same step,
cancels a pending join (`DOYEN_LEFT`) and stops the driver, then calls the adapter's `leave`
with one call timeout. It ignores a network error (a debug event); a refused leave returns
its error. Outside a Tokio runtime it sets `left` and returns `DOYEN_UNSUPPORTED` (the place
then expires). The notification comes before the adapter call (chosen in task group 6), so
an app stops its master work before the place is removed.

### Binding hooks (approved in task 0.1)

A `#[doc(hidden)] pub mod __binding` (not semver-covered) gives the bindings what the public
API does not:
- `set_liveness(check)`: a function the driver calls before each renew; if it returns false
  (no tick in the last `2 × renewIntervalMs`), the renew is skipped (Node.js event loop check).
- `warnings()`: a stream of the connect warnings, for Node.js `process.emitWarning` and Go
  `slog`. Rust apps get them through `tracing` only.

Alternative: public methods for these. Rejected: the public API is five operations.

Chosen in task group 7:
- Both hooks are functions that take the object first: `set_liveness(&doyen, check)` and
  `warnings(&doyen)`. The module is `engine/binding.rs`, shown at the crate root as
  `__binding` (`#[doc(hidden)] pub use`), so it can reach the private parts of `Doyen`.
  `Warning` is public for it (`__binding::Warning`, fields `code` and `message`).
- `set_liveness`: the check is `Fn() -> bool + Send + Sync`; it can be set before or after
  `join`, and a new call replaces it. The driver calls it, without the lock, before each
  renew of its timer and of a wake-up; the join attempts (and the renew inside a join) do not
  call it. A skipped renew writes a debug event and does not change the last error. With no
  renew the lease ends (offline `connectionLost`); when the check returns true again, the
  next renew gets the place back if it did not expire, else the place is lost as usual.
- `warnings`: one unbounded queue per stream, like `status_changes()`. A stream gets the
  warnings from the moment it was made (no replay of earlier ones), each one once, before
  `join` finishes; it ends after `leave` (a stream made after `leave` ends at once). The
  bindings make their stream before they call `join`.

### Dependencies

`tokio` (rt, time, sync, macros: the driver waits on its timer, the watch stream, the lease
deadline and the stop signals with `select!`; test-util in dev), `futures-core`/`futures-util`
(`Stream`), `async-trait`, `tracing`, `rand` (the 128-bit tag, OS-seeded), `thiserror`, `url`,
`gethostname` (the default instance name), `rustix` (the clock). All of them must build with
Rust 1.85 (the `msrv` CI job checks it).

## Risks / Trade-offs

- [An app never reads a `status_changes()` stream] → its queue grows with each change. A
  change is rare (seconds apart), and dropping the stream frees it; documented.
- [The fake clock hides a bug of the real clock] → one test with the real clock and a real
  2 s sleep checks the lease end.
- [Tokio timers do not count suspend] → after a resume, the next renew may come late, but the
  lease uses the suspend-aware clock, so the status is right; only the takeover is later.
- [`async_trait` boxes each call] → one allocation per database call: small next to the
  network time.
