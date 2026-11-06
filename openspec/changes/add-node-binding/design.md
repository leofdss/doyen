# Design

## Context

`bindings/node` today is a placeholder package (0.0.2 on npm) whose `index.js` throws. The
core gives the public Rust API and the hidden hooks `__binding::set_liveness` and
`__binding::warnings` (`add-core-election`). The behaviour is in the `node-binding` spec.

## Goals / Non-Goals

**Goals:**
- JavaScript never waits for the database, and the database never waits for JavaScript.
- The generated `index.d.ts` is the public API; runtime values match it.

**Non-Goals:**
- A pure JavaScript version.
- musl (Alpine), Windows and macOS x64 binaries in 0.2.0 (later, if asked).
- Bun and Deno support (not tested; Node-API may work there).

## Decisions

### Layers and threads

- `bindings/node` holds the crate `doyen-node` (`crate-type = ["cdylib"]`, `publish = false`,
  workspace member) and the npm files. napi-rs v3 with the `tokio_rt` feature: the core, its
  timers and all database calls run on the napi-rs Tokio runtime; JavaScript runs on its own
  thread.
- Sync methods (`new Doyen()`, `status()`, `token()`, `onStatusChange()`) read the core
  snapshot: no `await`, no database call. `join` and `leave` return Promises.

### What crosses the border

| Direction | What | napi-rs tool |
|---|---|---|
| JS → Rust, async | `join(config)`, `leave()` | Promise; `Err` → reject |
| Rust → JS | status changes | one `ThreadsafeFunction` per callback: weak, `CalleeHandled = false`, `NonBlocking`, unbounded queue |
| Rust → JS | tick + keep-alive | one strong `ThreadsafeFunction` from `join` to `leave` |
| Rust → JS | warnings | a `ThreadsafeFunction` that calls `process.emitWarning` |
| Rust → JS | errors | `Error` with `code` = the `DOYEN_*` code |

### Types

| TS | Rust in the binding | Note |
|---|---|---|
| `DatabaseConfig` | structured enum, `discriminant = "type"`, `discriminant_case = "camelCase"` | Numbers as `f64`, checked by the core (the Node-API `u32` turns −1 into 4294967295). napi-rs keeps the Rust names of variant fields, so the fields are written in camelCase with `#[allow(non_snake_case)]` (or `js_name` if variants support it). |
| `Status` | structured enum, `discriminant = "state"`, `discriminant_case = "camelCase"` | Without the case option the value would be `"Master"`. `error: Option<String>` is absent when `None`. |
| `State`, `OfflineReason`, `DatabaseType` | `#[napi(string_enum = "camelCase")]` | |
| `Token` | `#[napi(object)]` with two `i64` (JS numbers) | `Option<Token>` → `null`; exact below 2^53 |

- **`join` takes `Unknown`** with `ts_args_type = "config: DatabaseConfig"` and converts it
  inside: napi-rs converts typed arguments before the Promise exists, so a bad config would
  throw with `InvalidArg`. The conversion error becomes a rejected Promise with
  `DOYEN_INVALID_CONFIG`.
- **Two tests in CI:** `tsc --noEmit` on the README example (types), and a runtime test that
  compares `status().state`, `status().reason` and the config read-back with the enums. A
  hand-written `ts_type` must never hide a runtime difference.

### Tick

At each renew interval the core sends a tick through the strong thread-safe function; the
tick is native code that only writes the time. A new tick goes only after the last one ran.
`__binding::set_liveness` checks "a tick ran in the last 2 × `renewIntervalMs`" before each
renew. With the defaults: blocked < 8 s → nothing (the last renew can be 2 s old, the lease
lasts 9 s after it, the next renew waits for the next tick); > 13 s → the lease ends. Between
the two it depends on the phase of the block.

### Life of the object

- `join`: rejects with `DOYEN_ALREADY_JOINED` if already called (and not rejected); converts
  the config; creates the strong function (the process stays alive); starts the core.
- `join` rejected: drop the strong function; status `notJoined`.
- `leave`: the core ends the lease, removes the place (one call timeout at most), never
  rejects on a network error; drop the strong function; a pending `join` rejects with
  `DOYEN_LEFT`; a second `leave` resolves at once.
- Process exit without `leave`: no hidden `leave` at exit, because `process.exit()` does not
  wait for a network call. The place expires after `timeoutMs`.
- The `join` Promise and the first `master` or `slave` callback come through two different
  queues, so either one can run first. The README says: do not depend on that order.
- The binding keeps a reference to the JS object while joined (no garbage collection).
- Worker threads: the thread-safe functions belong to the worker's environment; an env
  cleanup hook stops the core when the worker ends.
- `catch_unwind` around each method: a panic becomes a JS error, never a crash.

### Build, platforms and release

- `@napi-rs/cli` builds the binaries. CI matrix: `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu` (cross), `aarch64-apple-darwin`. Platforms to confirm (owner).
- Node.js versions: the active and maintenance LTS lines at release time (22 and 24 in
  2026), in the test matrix. To confirm (owner).
- Each binary is built with the crate's default features: every adapter of the release
  (Redis and MongoDB in 0.2.0). One package works with every database, so the install stays
  simple. To confirm (owner).
- Release: the job `npm` in `release.yml` (from `publish-rust-crate`) runs on a release
  `v<version>`: it builds every platform, runs `napi prepublish` (the platform packages get
  the same version), and runs `npm publish --provenance` for each package, in the
  environment `release`, with npm Trusted Publishing (`id-token: write`, npm 11.5.1 or
  newer). One version for every package (`release-process`): the first npm release is the
  release that ends this change, 0.2.0 by the plan. The platform package folders
  (`npm/<platform>/`) are made in CI and not committed, so their versions never drift from
  the main `package.json` (the `@napi-rs/cli` v3 command for it: to confirm while applying). To confirm while applying: whether npm
  needs each platform package to exist before a Trusted Publisher can be added to it (then
  the owner publishes a placeholder of each by hand once).

## Risks / Trade-offs

- [napi-rs keeps the Rust field names in enum variants] → camelCase Rust fields + the
  read-back test.
- [A blocked event loop cannot be seen from Rust otherwise] → the tick; cost: one tiny call
  per interval.
- [A callback that throws ends the process if the app has no `uncaughtException` handler] →
  the same as `EventEmitter`; documented.
- [Weak thread-safe functions for callbacks] → they do not keep the process alive; the
  strong tick function does that from `join` to `leave`.
