# Spec Delta

## Purpose

Defines the public Rust API of the crate doyen: the object, its five operations, the config,
status, token and error types, and the cargo features for the adapters.

## ADDED Requirements

### Requirement: Create the object before join
`Doyen::new()` SHALL create an object with no connection and the status offline
`notJoined`. The object SHALL be `Send` and `Sync`, and its clones SHALL share one membership.

#### Scenario: New object
- **WHEN** an app calls `Doyen::new()`
- **THEN** `status()` is `Status::Offline` with reason `NotJoined` and no database call is made

#### Scenario: Clone
- **WHEN** an app clones the object and calls `leave()` on the clone
- **THEN** the first object also shows offline `left`

### Requirement: The five operations
The object SHALL have exactly these public operations: `join(config)` (async, returns
`Result<(), Error>`), `status()` (returns `Status`), `status_changes()` (returns a `Stream` of
`Status`), `token()` (returns `Option<Token>`) and `leave()` (async, returns
`Result<(), Error>`).

#### Scenario: Typical use
- **WHEN** an app calls `d.join(config).await?`, reads `d.status()` and `d.token()`, and calls `d.leave().await?` at shutdown
- **THEN** every call compiles and works as the `election-core` capability says

### Requirement: Status type
`Status` SHALL be an enum with `Master`, `Slave` and `Offline { reason: OfflineReason, error:
Option<String> }`. `OfflineReason` SHALL have `NotJoined`, `Connecting`, `ConnectionLost`,
`Reconnecting` and `Left`, and its `Display` SHALL give the camelCase names.

#### Scenario: Display a reason
- **WHEN** an app formats `OfflineReason::ConnectionLost` with `{}`
- **THEN** the text is `connectionLost`

### Requirement: Status changes as a stream
`status_changes()` SHALL return a stream that yields every status change in order, from the
moment it was called. Each call SHALL make a new, independent stream. The stream SHALL end
after the `left` status.

#### Scenario: Two streams
- **WHEN** an app calls `status_changes()` twice and the status changes once
- **THEN** both streams yield that change

#### Scenario: End of the stream
- **WHEN** `leave()` finished
- **THEN** the stream yields `Offline { reason: Left, .. }` and then ends

### Requirement: Token type
`Token` SHALL have the public fields `generation: u64` and `number: u64`, and SHALL implement
`Ord` with the generation compared first.

#### Scenario: Compare tokens
- **WHEN** an app compares `Token { generation: 2, number: 1 }` with `Token { generation: 1, number: 9 }`
- **THEN** the first one is greater

### Requirement: Config type
`DatabaseConfig` SHALL be an enum with one variant per database, each with a config struct.
The shared settings SHALL be the fields `url`, `group`, `timeout_ms: Option<u64>`,
`renew_interval_ms: Option<u64>` and `instance_name: Option<String>`. Each struct SHALL have a
constructor that takes the required fields (`url`, `group`).

#### Scenario: Build a config
- **WHEN** an app writes `DatabaseConfig::Redis(RedisConfig::new("redis://localhost", "billing-jobs"))`
- **THEN** the config uses the defaults for every optional setting

### Requirement: Error type
Errors SHALL be one type, `doyen::Error`, with a method `code()` that returns the code (one
of the `client-config` error codes) and a `Display` that starts with the code, for example
`DOYEN_INVALID_CONFIG: …`.

#### Scenario: Match a code
- **WHEN** `join` fails because of a bad config
- **THEN** `err.code()` is `ErrorCode::InvalidConfig` and the message starts with `DOYEN_INVALID_CONFIG`

### Requirement: Cancel a join by dropping it
When the app drops a `join` future before it finished, the status SHALL go back to offline
`notJoined`, and a later `join` on the same object SHALL keep the same membership tag.

#### Scenario: Join with a timeout
- **WHEN** an app wraps `join` in `tokio::time::timeout` and the timeout ends first
- **THEN** the status is offline `notJoined`, and a new `join` finds the place the first one may have written

### Requirement: Cargo features per adapter
Each database adapter SHALL be a cargo feature named after its `type` (`redis` and `mongodb`
in v0.1), and the default features SHALL include every adapter. The config types SHALL exist
with any features; a config for a database whose feature is off SHALL make `join` fail with
`DOYEN_UNSUPPORTED`.

#### Scenario: Feature off
- **WHEN** the crate is built with `default-features = false, features = ["redis"]` and an app joins with a MongoDB config
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message names the missing feature

### Requirement: Tokio runtime
`join` and `leave` SHALL run on a Tokio runtime, and the core SHALL start its own tasks on the
runtime where `join` was called. If that runtime stops, the renews SHALL stop and the lease
SHALL end on its own.

#### Scenario: Runtime shut down
- **WHEN** the app's Tokio runtime shuts down while the instance is master
- **THEN** no renew is sent, and the status is not master after the lease ends
