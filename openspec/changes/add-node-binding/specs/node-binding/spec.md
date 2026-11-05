# Spec Delta

## Purpose

Defines the Node.js package @leofdss/doyen: the same five operations and status rules as the
core, with Node.js types, callbacks, an event loop check, errors, warnings and releases.

## ADDED Requirements

### Requirement: Package and platforms
The package `@leofdss/doyen` SHALL install with no build step on Linux x64 (glibc), Linux arm64
(glibc) and macOS arm64, through prebuilt binaries in the optional packages
`@leofdss/doyen-<platform>`. On another platform it SHALL fail at import with a message that
names the platform.

#### Scenario: Install on Linux x64
- **WHEN** a user runs `npm install @leofdss/doyen` on Linux x64 with glibc
- **THEN** the install needs no compiler and `import { Doyen } from "@leofdss/doyen"` works

#### Scenario: Unsupported platform
- **WHEN** the package is imported on Linux with musl
- **THEN** the import fails with a message that names the platform

### Requirement: Node.js public API
The package SHALL export the class `Doyen` with exactly `join(config): Promise<void>`,
`status(): Status`, `onStatusChange(callback): void`, `token(): Token | null` and
`leave(): Promise<void>`, the string enums `DatabaseType`, `State` and `OfflineReason`, and
the types `DatabaseConfig`, `Status` and `Token`. They SHALL behave as `election-core` and
`status-reporting` say.

#### Scenario: Types of the README example
- **WHEN** the CI type-checks the README example with `tsc --noEmit`
- **THEN** it passes with the package's `index.d.ts`

### Requirement: Runtime values match the enums
`status().state` SHALL be the string `master`, `slave` or `offline` (equal to `State.master`,
…), `status().reason` SHALL equal an `OfflineReason` value, and `DatabaseType` values SHALL be
the `type` values of the adapters (`redis`, `mongodb`).

#### Scenario: Compare with the enum
- **WHEN** an instance is master
- **THEN** `doyen.status().state === State.master` is true

#### Scenario: Offline status object
- **WHEN** the instance is offline because the database refuses the login
- **THEN** `status()` is `{ state: "offline", reason: "connectionLost", error: "<message>" }` and `error` is absent when there is no error

### Requirement: Config from JavaScript
`join` SHALL read the config fields with their camelCase names (`timeoutMs`,
`renewIntervalMs`, `instanceName`, and each adapter's settings). Any config problem — not an
object, a missing or wrong field, an unknown `type` — SHALL reject the Promise with
`DOYEN_INVALID_CONFIG`, never throw.

#### Scenario: Setting read back
- **WHEN** an app joins with `timeoutMs: 15000`
- **THEN** the core uses 15000 ms, not the default

#### Scenario: Missing url
- **WHEN** an app calls `doyen.join({ type: "redis", group: "g" })`
- **THEN** the call returns a Promise that rejects with `error.code === "DOYEN_INVALID_CONFIG"`

### Requirement: Status changes through callbacks
`onStatusChange(callback)` SHALL call the callback on the JavaScript thread for every change,
in order, also when added before `join`. A callback SHALL get the status of the moment of the
change; it can run late when the event loop is busy. `left` SHALL be the last call.

#### Scenario: Busy event loop
- **WHEN** three changes happen while the event loop is busy
- **THEN** the callback runs three times, in order, when the loop is free

### Requirement: A throwing callback
If a callback throws, the binding SHALL NOT catch it: it SHALL become an `uncaughtException`,
like an exception in an `EventEmitter` listener. The core SHALL keep running.

#### Scenario: Callback throws
- **WHEN** a status callback throws an error
- **THEN** `process.on("uncaughtException")` gets it and the renews continue

### Requirement: A blocked event loop stops the renews
The binding SHALL send a tick to the JavaScript thread each renew interval, and the core SHALL
send a renew only if a tick ran in the last 2 × `renewIntervalMs`. With the defaults, a loop
blocked less than 11 s SHALL change nothing, and a loop blocked more than 13 s SHALL make the
status offline `connectionLost`.

#### Scenario: Endless loop
- **WHEN** the master's event loop is blocked for 20 s
- **THEN** the next member becomes master, as after a crash, and the old one joins again or gets its place back when the loop is free

#### Scenario: Short block
- **WHEN** the event loop is blocked for 5 s
- **THEN** the master does not change

### Requirement: Process life
From `join` to `leave` the binding SHALL keep the process alive, like an open database
connection. After `leave`, or after `join` rejected, it SHALL NOT keep the process alive. On
process exit without `leave`, nothing more SHALL run and the place expires.

#### Scenario: Script ends after leave
- **WHEN** a script calls `join`, then `await doyen.leave()`, and has no other work
- **THEN** the process exits

### Requirement: Leave never rejects on a network error
`leave()` SHALL resolve when the database removed the place, or after one call timeout, and
SHALL NOT reject on a network error. A second `leave()` SHALL resolve at once.

#### Scenario: SIGTERM with the database down
- **WHEN** a `SIGTERM` handler calls `await doyen.leave()` while the database does not answer
- **THEN** the Promise resolves after one call timeout

### Requirement: Error codes in JavaScript
Every rejected Promise SHALL have an `Error` with `error.code` set to one of the
`client-config` codes and a message that says what to fix. Network errors SHALL NOT reach
JavaScript as errors.

#### Scenario: Join after leave
- **WHEN** `join` is called after `leave`
- **THEN** it rejects with `error.code === "DOYEN_LEFT"`

### Requirement: Warnings through process.emitWarning
Each connect warning SHALL be sent once with `process.emitWarning(message, { code })` on the
JavaScript thread.

#### Scenario: Redis with volatile eviction
- **WHEN** the Redis adapter returns `DOYEN_REDIS_EVICTION`
- **THEN** `process.on("warning")` gets a warning with `code === "DOYEN_REDIS_EVICTION"`

### Requirement: Worker threads and garbage collection
A `Doyen` object SHALL belong to the thread that created it (its callbacks and its tick use
that thread's event loop); when the worker ends, the core SHALL stop. While joined, the
object SHALL NOT be garbage collected.

#### Scenario: Object not kept by the app
- **WHEN** an app joins and keeps no reference to the object
- **THEN** the instance stays in the line until `leave` or process exit

### Requirement: Token numbers
`Token` SHALL have `generation` and `number` as JavaScript numbers (not `bigint`). They are
exact below 2^53, far above a time in ms or a member number.

#### Scenario: Token type
- **WHEN** an app reads `doyen.token()` as master
- **THEN** `typeof token.generation === "number"`

### Requirement: Release of the package
Each release `v<version>` (see `release-process`) SHALL publish `@leofdss/doyen` and its
platform packages with that version, built by CI from the tagged commit, in the GitHub
environment `release` after the owner's approval, with npm Trusted Publishing and
provenance. `publishConfig.registry` SHALL be `https://registry.npmjs.org/`.

#### Scenario: Release 0.2.0
- **WHEN** the release `v0.2.0` is made and the owner approves the environment `release`
- **THEN** the main package and every platform package are on npm as 0.2.0, with provenance
