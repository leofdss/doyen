# Design

## Context

The core gives the public Rust API and the hidden hooks (`__binding::warnings`). The
behaviour is in the `go-binding` spec. The model is wasmtime-go: a Rust library used from Go
through cgo, with prebuilt static libraries inside the Go module.

## Goals / Non-Goals

**Goals:**
- One small C ABI written by hand; no Rust type crosses the border.
- Rust threads never call Go code.

**Non-Goals:**
- A pure Go port (plan B if Go users reject cgo).
- purego (needs a shared library next to the program) and WebAssembly (no network inside the
  sandbox, so the adapters would be written twice).
- Alpine (musl), macOS amd64 and Windows in 0.3.0 (later).

## Decisions

### C ABI (crate `doyen-ffi`)

An opaque handle and 9 functions: `new`, `join`, `cancel`, `status`, `next_event`, `token`,
`leave`, `free`, `error_free`. The header comes from cbindgen. Strings are UTF-8 pointer +
length.

- **Errors in the same call:** a function that can fail returns a code and fills an
  out-parameter `DoyenError*` (code + message); Go copies the message and calls
  `error_free`. A thread-local "last error" breaks with cgo (a goroutine can move to another
  OS thread between two calls); a per-handle one is overwritten by another goroutine.
- **Handle:** an `Arc` inside; each running call holds its own reference, `free` drops only
  the handle's one, so `free` during a call is never a use after free.
- **`cancel(handle)`** wakes only a blocked `join` (→ `DOYEN_CANCELLED`). `next_event` is not
  cancelled: it takes a timeout (1 s), so the event goroutine checks between calls whether
  it must stop.
- **`next_event`** blocks until an event or its timeout. An event is a status change (two
  small integers: state and reason, and the error message in a `DoyenError` when there is
  one) or a warning (code + message).
- `join` blocks the calling OS thread; Go runs it in the goroutine that called `Join`, and a
  second goroutine calls `cancel` when `ctx` ends.

### Go package

- The event goroutine holds only an inner struct, never the `*Doyen` the user holds, so that
  object can be collected. It sends a status with `select { case ch <- s: case <-stop:
  return }`, and a warning to `slog.Default()`.
- `runtime.AddCleanup` (Go 1.24+) on the `*Doyen`: the cleanup only starts a goroutine that
  stops the event goroutine, waits for it (at most one `next_event` timeout), closes the
  channel and calls `free`. All cleanups share one goroutine, so a cleanup must not block.
- `Leave(ctx)`: C `leave` (one call timeout at most), then the same steps.
- Go version: 1.24 or newer (`runtime.AddCleanup`).

### Build and release

- `bindings/go` holds the Go code; `bindings/go/lib/` is git-ignored (local builds).
  `#cgo` lines pick the library file for each platform.
- Release profile with LTO, then `llvm-strip --strip-debug` on each `.a`: Cargo's `strip`
  does not strip a `staticlib` (a test: 12 MB with debug sections, 3.7 MB after).
- Jobs `go-build` and `go` in `release.yml` (from `publish-rust-crate`), on a release
  `v<version>`: build the three platforms (linux/amd64, linux/arm64 cross, darwin/arm64 on a
  macOS runner), strip, check sizes (< 100 MiB per file, zip < 500 MiB), run the Go tests
  with the built libraries, then, in the environment `release`, push the code + libraries to
  `leofdss/doyen-go` with a deploy key (secret `DOYEN_GO_DEPLOY_KEY`, stored in the
  environment `release` only) and the same tag `v<version>` there. The owner creates the
  repo and the key. Go has no Trusted Publishing: the module is a git repo.
- The Go package docs say: `CGO_ENABLED=1` and a C compiler are needed; cgo is off by default
  when cross-compiling.

## Risks / Trade-offs

- [Users do not want cgo] → plan B is a pure Go port (every adapter written twice); not now.
- [Static libraries make the module repo large] → each release adds ~10 MB per platform;
  acceptable for a small project; the size checks stop a mistake.
- [A blocked `next_event` call holds an OS thread] → one thread per object, for at most 1 s
  per call.
- [macOS runners cost more CI time] → the darwin build runs only on release tags and on
  `main`.
