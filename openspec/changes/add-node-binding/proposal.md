# Proposal

## Why

Node.js is the first language binding of the plan (v0.2): many services that need "only one
instance does this work" are Node.js services, and the closest existing libraries
(mongo-leader, @platformatic/leader) are Node.js packages with no line and no fencing token.
The binding must give the same five operations and the same status rules as the Rust core,
in the usual Node.js style.

Release: v0.2. Depends on: `publish-rust-crate` (v0.1 done).

The napi-rs design of this change is a proposal: the tick, the process kept alive from join
to leave, `leave` that never rejects, the error codes, the platforms and the Node.js
versions. It is written in full in design.md and in the `node-binding` spec. **Approving
this change is the owner's decision on it** (task 0.1).

## What Changes

- A napi-rs (v3) binding crate in `bindings/node`, built into the npm package
  `@leofdss/doyen` with prebuilt binaries in `@leofdss/doyen-<platform>` packages.
- The class `Doyen` with `join`, `status`, `onStatusChange`, `token`, `leave`; the string
  enums `DatabaseType`, `State`, `OfflineReason`; the types `DatabaseConfig`, `Status`, `Token`.
- An event loop check (tick): a blocked event loop stops the renews, so a stuck process does
  not stay master.
- Warnings through `process.emitWarning`; errors with `error.code`.
- CI: build and test on each platform, a type test and runtime tests; a job `npm` in
  `release.yml` publishes with Trusted Publishing after the owner's approval.

## Capabilities

### New Capabilities

- `node-binding`: the Node.js package: its API, types, runtime values, callbacks, event loop
  check, process life, errors, warnings, platforms and release.

### Modified Capabilities

(none)

## Impact

- `bindings/node`: the placeholder (`index.js` that throws) is replaced by the napi-rs
  package; new crate `doyen-node` (workspace member, `publish = false`).
- The core's hidden hooks (`__binding::set_liveness`, `__binding::warnings`) from
  `add-core-election` are used here.
- npm: `@leofdss/doyen` 0.2.0 and the platform packages (CI publishes after the owner's
  approval).
