# Spec Delta

## Purpose

Defines the Go module github.com/leofdss/doyen-go: the same five operations and status rules
as the core in Go style, with context cancel, a status channel, errors, warnings and releases.

## ADDED Requirements

### Requirement: Module, platforms and cgo
The module `github.com/leofdss/doyen-go` (package `doyen`) SHALL build with `CGO_ENABLED=1`
and a C compiler on linux/amd64, linux/arm64 and darwin/arm64, with no Rust toolchain, using
the prebuilt static libraries it contains.

#### Scenario: go get
- **WHEN** a user runs `go get github.com/leofdss/doyen-go` and builds on linux/amd64 with cgo
- **THEN** the build works with no Rust toolchain

#### Scenario: cgo off
- **WHEN** a user builds with `CGO_ENABLED=0`
- **THEN** the build fails, and the package docs say that cgo is required

### Requirement: Go public API
The package SHALL have `New() *Doyen` and the methods `Join(ctx, cfg) error`, `Status()
Status`, `StatusChanges() <-chan Status`, `Token() (Token, bool)` and `Leave(ctx) error`,
and nothing more on `Doyen`. They SHALL behave as `election-core` and `status-reporting` say.

#### Scenario: Typical use
- **WHEN** an app calls `d := doyen.New()`, `d.Join(ctx, cfg)`, reads `d.Status()` and `d.Token()`, and calls `d.Leave(ctx)` at shutdown
- **THEN** every call compiles and works as the core capabilities say

### Requirement: Go types
`Status` SHALL be a struct `{State, Reason, Err}`: `Reason` is `ReasonNone` and `Err` is `nil`
unless the state is offline. `State` and `Reason` SHALL have a `String()` method with the
`status-reporting` names. `Token` SHALL have `Generation` and `Number` as `uint64`. The config
SHALL be a struct per database with the shared settings.

#### Scenario: Print a reason
- **WHEN** an app prints `d.Status().Reason` while offline because the database is down
- **THEN** the text is `connectionLost`

### Requirement: Cancel a join with the context
When the `ctx` of `Join` ends before `Join` finished, `Join` SHALL return an error with the
code `DOYEN_CANCELLED` (a Go-only code), the status SHALL be offline `notJoined`, and a later
`Join` on the same object SHALL keep the same membership tag.

#### Scenario: Join with a deadline
- **WHEN** `Join` runs with a 1 s deadline and the database does not answer
- **THEN** `Join` returns `DOYEN_CANCELLED` after 1 s, and a later `Join` finds a place the first one may have written

### Requirement: Status changes on a channel
`StatusChanges()` SHALL return the same channel on every call (one subscriber per object), and
it SHALL get every status change in order. It SHALL be closed after `Leave` (or when the
object is collected). A status the app did not read when the channel closes SHALL be dropped;
`Status()` has the last one.

#### Scenario: Read changes
- **WHEN** an app ranges over `d.StatusChanges()` and the instance becomes master
- **THEN** it gets a `Status` with the state master

#### Scenario: Channel after Leave
- **WHEN** `Leave` finished
- **THEN** the channel is closed and the range loop ends

### Requirement: Nothing waits for the channel reader
`Leave` and the cleanup of a collected object SHALL finish even when nobody reads the status
channel. After them the core SHALL stop and SHALL NOT renew.

#### Scenario: App stopped reading
- **WHEN** the app stopped reading `StatusChanges()` and then calls `Leave(ctx)`
- **THEN** `Leave` returns, and the instance is no longer in the line

### Requirement: Object collected without Leave
When the app drops every reference to a joined `*Doyen`, the garbage collector SHALL stop the
core and free it; the place SHALL expire after `timeoutMs`. Calls from many goroutines at the
same time SHALL be safe, and no call SHALL use freed memory.

#### Scenario: Forgotten object
- **WHEN** an app joins in a function and keeps no reference after it returns
- **THEN** after a garbage collection the renews stop and the place expires

#### Scenario: Concurrent calls
- **WHEN** ten goroutines call `Status()` and `Token()` while another calls `Leave`
- **THEN** no call crashes, and the race detector (`go test -race`) finds nothing

### Requirement: Go errors
Errors SHALL be Go `error` values with a `Code` (one of the `client-config` codes or
`DOYEN_CANCELLED`) usable with `errors.As`. `Leave` SHALL NOT return an error for a network
failure.

#### Scenario: Bad config
- **WHEN** `Join` gets a config with an empty group
- **THEN** the error's `Code` is `DOYEN_INVALID_CONFIG`

### Requirement: Warnings through slog
Each connect warning SHALL be logged once with `slog.Default()` at level `Warn`, with the
attribute `code`.

#### Scenario: PSA replica set
- **WHEN** the MongoDB adapter returns `DOYEN_MONGODB_ARBITER`
- **THEN** the default logger gets one `Warn` record with `code=DOYEN_MONGODB_ARBITER`

### Requirement: Release of the module
Each release SHALL build the static library per platform with debug sections removed, check
that each file is below 100 MiB and the module zip below 500 MiB, commit the Go code and the
libraries to `leofdss/doyen-go`, and tag the version there with the tag of the release
`v<version>` (see `release-process`), in the GitHub environment `release` after the owner's
approval.

#### Scenario: Release 0.3.0
- **WHEN** the release `v0.3.0` is made and the owner approves the environment `release`
- **THEN** `leofdss/doyen-go` gets a commit with the code and three libraries, and the tag `v0.3.0`

#### Scenario: Library too large
- **WHEN** a stripped library is over 100 MiB
- **THEN** the release workflow fails before it pushes anything

### Requirement: Evidence of the module release
Each release SHALL make a GitHub artifact attestation of each library. The module SHALL have
both license files, and the GitHub release SHALL get an SBOM of each library.

#### Scenario: Library provenance
- **WHEN** the release `v0.3.0` is in `leofdss/doyen-go`
- **THEN** `gh attestation verify <library> -R leofdss/doyen` passes for each of the three libraries

#### Scenario: Module contents
- **WHEN** the release `v0.3.0` is in `leofdss/doyen-go`
- **THEN** the module has `LICENSE-MIT` and `LICENSE-APACHE`, and the GitHub release `v0.3.0` has an SBOM of each library
