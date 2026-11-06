# Spec Delta

## Purpose

Defines the config every database shares (type, url, group, timing, instance name), its
defaults and checks, and the error codes that every language returns.

## ADDED Requirements

### Requirement: Config typed by database
The config SHALL be one of several shapes chosen by its `type` field. Each database adapter
SHALL define its `type` value and its extra settings in its own capability. The adapter SHALL
come from `type`, never from the URL scheme.

#### Scenario: Redis config
- **WHEN** the config has `type` set to the Redis value
- **THEN** the core uses the Redis adapter and accepts only the Redis settings

#### Scenario: Unknown type
- **WHEN** the config has a `type` no adapter defines
- **THEN** `join` fails with `DOYEN_INVALID_CONFIG`

### Requirement: Shared settings and defaults
Every config SHALL have `url` (required), `group` (required), `timeoutMs` (default 10000),
`renewIntervalMs` (default 2000) and `instanceName` (default: the host name; only a label for
logs and the database records).

#### Scenario: Only the required settings
- **WHEN** the config has only `type`, `url` and `group`
- **THEN** the core uses a timeout of 10000 ms, a renew interval of 2000 ms and the host name as the instance name

### Requirement: Group name rules
`group` SHALL NOT be empty and SHALL NOT contain `{` or `}`. Each group SHALL have its own
line and its own master.

#### Scenario: Braces in the group
- **WHEN** `group` is `billing{jobs}`
- **THEN** `join` fails with `DOYEN_INVALID_CONFIG`

#### Scenario: Two groups
- **WHEN** two services use the same database with the groups `billing` and `mail`
- **THEN** each group has its own master

### Requirement: Timing checks
`timeoutMs` and `renewIntervalMs` SHALL be positive integers, and `timeoutMs` SHALL be at
least 3 × `renewIntervalMs`. Otherwise `join` SHALL fail with `DOYEN_INVALID_CONFIG`.

#### Scenario: Timeout too short
- **WHEN** `timeoutMs` is 5000 and `renewIntervalMs` is 2000
- **THEN** `join` fails with `DOYEN_INVALID_CONFIG` and the message says that the timeout must be at least 3 times the interval

#### Scenario: Zero interval
- **WHEN** `renewIntervalMs` is 0
- **THEN** `join` fails with `DOYEN_INVALID_CONFIG`

### Requirement: URL check
`url` SHALL be a valid URL with a scheme the chosen adapter accepts. Otherwise `join` SHALL
fail with `DOYEN_INVALID_CONFIG`.

#### Scenario: Wrong scheme
- **WHEN** the config has the MongoDB type and the URL `redis://localhost`
- **THEN** `join` fails with `DOYEN_INVALID_CONFIG`

### Requirement: Secrets are never shown
doyen SHALL NOT show a secret of a URL (the password of the user info, and query parameters
that hold a password, such as `sentinelPassword`) in an error, a warning, a log line, a
status or the `Debug` output of a config. Where it shows a URL, it SHALL write `***` in
place of each secret.

#### Scenario: Password in an error
- **WHEN** `join` fails with the URL `redis://app:s3cret@db:6379`
- **THEN** no error, warning, log line or status contains `s3cret`, and the URL is shown as `redis://app:***@db:6379`

#### Scenario: Debug of a config
- **WHEN** a config with the URL `mongodb://app:s3cret@db/doyen` is printed with `{:?}`
- **THEN** the output contains `app:***@db` and not `s3cret`

### Requirement: Error codes
Every error the core returns SHALL have one of these codes and a message that says what to
fix: `DOYEN_INVALID_CONFIG`, `DOYEN_AUTH`, `DOYEN_UNSUPPORTED`, `DOYEN_ALREADY_JOINED`,
`DOYEN_LEFT`. Network errors SHALL NOT be returned as errors. A binding SHALL define any
extra code in its own capability.

#### Scenario: Refused command
- **WHEN** the database refuses a command doyen needs because of its access rules
- **THEN** the error code is `DOYEN_AUTH` and the message names the command

#### Scenario: Network error
- **WHEN** the database does not answer during `join`
- **THEN** no error is returned; the core retries and the status shows the problem
