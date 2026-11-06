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

### Crate layout — to confirm (owner)

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
| `conformance` | the conformance suite, generic over `Backend` (`#[cfg(test)]`) |
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

`async_trait` because the core holds a `Box<dyn Backend>` (the database is chosen at run
time from the config). Native `async fn` in traits is not object-safe.

### Driver task, snapshot and subscribers

- `join` starts one Tokio task per object (the driver). It owns the membership, the tag and
  the schedule. The public object holds an `Arc<Shared>`:
  - `snapshot: std::sync::Mutex<Snapshot>`: my membership, the head of the line, the lease
    end, the generation, the reason, the last error. `status()` and `token()` lock it for a
    moment and compute the answer with `clock.now()`; they never `await`.
  - `subscribers: Mutex<Vec<mpsc::UnboundedSender<Status>>>`: one queue per
    `status_changes()` call. The driver pushes every change to all queues and removes closed
    ones. Unbounded: a change is never lost or merged (`status-reporting`).
- The driver computes the status after each answer and each lease deadline; it sends a
  notification only when the state or the reason changed.
- A panic in the driver ends it; the lease then ends on its own (safe).
- The public object holds an `Arc<Handle>`; clones share it. `Handle` keeps a
  `oneshot::Sender`, and the driver waits on its receiver next to its timers. When the last
  handle drops, the sender drops, and the driver stops at once (no renew, no `leave`: a
  `Drop` cannot wait for the database). The driver keeps only `Arc<Shared>`, never the
  handle, so it cannot keep the object alive. The Go binding's free uses the same path.
- Data format: the in-memory adapter and every adapter store `format` (1) with the group
  data; the conformance suite writes format 2 through a test hook of each adapter.
- Log events (`tracing`, target `doyen`): info for each status change, debug for each
  failed database call; the URL in an event is redacted like in errors.

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
- Tests: the fake clock reads `tokio::time::Instant` and the tests run with
  `tokio::time::pause()`, so the lease and the timers move together. One test also checks
  the real clock against a real sleep.

### Join, retries and reconnect

- Before the first join, the driver calls `connect` once. Network errors are retried like a
  join; refusals end `join` with the error.
- Retries (connecting and reconnecting): backoff from 100 ms, × 2, at most
  `renewIntervalMs`, with jitter ± 20%. A retry keeps the tag.
- The join lease counts from the send time of the first attempt with this tag; the driver
  keeps that time with the tag.
- A lost place (renew `None`, or my number not in the line) → new tag, reason
  `reconnecting`, join again.

### Leave

`leave` sets "left" in the snapshot first (so `status()` is offline `left` and `token()` is
`None` at once), cancels a pending join (`DOYEN_LEFT`), then calls the adapter's `leave`
with one call timeout, ignores a network error, sends the last notification `left`, closes
the queues and stops the driver.

### Binding hooks — to confirm (owner)

A `#[doc(hidden)] pub mod __binding` (not semver-covered) gives the bindings what the public
API does not:
- `set_liveness(check)`: a function the driver calls before each renew; if it returns false
  (no tick in the last `2 × renewIntervalMs`), the renew is skipped (Node.js event loop check).
- `warnings()`: a stream of the connect warnings, for Node.js `process.emitWarning` and Go
  `slog`. Rust apps get them through `tracing` only.

Alternative: public methods for these. Rejected: the public API is five operations.

### Dependencies

`tokio` (rt, time, sync; macros and test-util in dev), `futures-core`/`futures-util`
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
