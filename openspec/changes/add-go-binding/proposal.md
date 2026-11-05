# Proposal

## Why

Go is the third language of the plan (v0.3). Go services have the same "only one instance"
work, and the owner chose to keep the adapters written once, in Rust: the Go package calls
the core through a small C ABI and cgo, with prebuilt static libraries (the wasmtime-go
model). The Go API must feel like Go (`context`, channels, `error`) and keep the same rules.

Release: v0.3. Depends on: `add-node-binding` (v0.2 done; the release process is reused).

## What Changes

- A crate `doyen-ffi` (`staticlib`, `publish = false`) with a hand-written C ABI of 9
  functions and a header made by cbindgen.
- A Go package in `bindings/go` (`doyen.New()`, `Join(ctx, cfg)`, `Status()`,
  `StatusChanges()`, `Token()`, `Leave(ctx)`), with events by pull, never callbacks from Rust.
- Warnings through `slog`; errors as Go `error` values with a code; `DOYEN_CANCELLED` when
  the `context` ends a join.
- A release workflow that builds and strips the static libraries per platform, copies the Go
  code to `github.com/leofdss/doyen-go`, commits and tags the version there.

## Capabilities

### New Capabilities

- `go-binding`: the Go module: its API, cancel, status changes, object life, errors,
  warnings, platforms, cgo needs and release.

### Modified Capabilities

(none)

## Impact

- New: `crates/doyen-ffi`, `bindings/go` (`lib/` git-ignored), a release workflow.
- New repo `leofdss/doyen-go` (the owner creates it; CI pushes to it with a deploy key).
- Users need `CGO_ENABLED=1` and a C compiler.
