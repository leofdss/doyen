# Spec Delta

## Purpose

Defines what an instance reports about itself: master, slave or offline with a reason, the
last database error, one notification per change, and the warnings at connect.

## ADDED Requirements

### Requirement: Three states, offline with a reason
`status()` SHALL return one of three states: `master`, `slave` or `offline`. An offline status
SHALL always have a reason: `notJoined`, `connecting`, `connectionLost`, `reconnecting` or
`left`. Master and slave SHALL have no reason.

#### Scenario: New object
- **WHEN** an object was created and `join` was not called
- **THEN** `status()` is offline with reason `notJoined`

#### Scenario: Joined
- **WHEN** `join` finished
- **THEN** `status()` is master or slave, with no reason

### Requirement: Reasons before the first place
The reason SHALL be `connecting` while `join` runs and the instance has no place yet. It
SHALL be `notJoined` before `join`, and again after `join` failed or was cancelled.

#### Scenario: Join running
- **WHEN** `join` runs and the database does not answer
- **THEN** `status()` is offline with reason `connecting`

#### Scenario: Join failed
- **WHEN** `join` fails with `DOYEN_AUTH`
- **THEN** `status()` is offline with reason `notJoined`

### Requirement: Connection lost keeps the same place
When the local lease ends with no successful renew, the status SHALL be offline with reason
`connectionLost`, and the core SHALL keep trying to renew the same place. When a renew works
again, the status SHALL go back to master or slave.

#### Scenario: Short outage longer than the lease
- **WHEN** the database does not answer for 9.5 s and then answers before the place expired in the database
- **THEN** the status goes to offline `connectionLost`, then back to master or slave with the same number

### Requirement: Reconnecting after the place is gone
When the database says the place is gone, the status SHALL be offline with reason
`reconnecting` until a new join returns a place. A network error during `reconnecting` SHALL
keep `reconnecting`.

#### Scenario: Outage longer than the timeout
- **WHEN** the status is `connectionLost` and the next answer says the place expired
- **THEN** the status is offline with reason `reconnecting`, then master or slave with a new number

#### Scenario: Network error while reconnecting
- **WHEN** the status is `reconnecting` and the next join attempt has a network error
- **THEN** the status stays offline with reason `reconnecting`

### Requirement: Left is the last status
After `leave`, the status SHALL be offline with reason `left`, and it SHALL never change
again.

#### Scenario: After leave
- **WHEN** `leave` finished
- **THEN** `status()` is offline with reason `left` and no more notifications come

### Requirement: Last database error in the offline status
An offline status SHALL carry the message of the last failed database call, when there is
one (for example a login error or `READONLY`). `status()` SHALL always show the latest
message.

#### Scenario: Wrong password after join
- **WHEN** the instance is offline because the database refuses the login
- **THEN** `status()` has the reason and the login error message

### Requirement: One notification per change, in order
Each subscriber SHALL get every change of state or reason, in order, none lost or merged. A
new error message with the same state and reason SHALL NOT be a change. A subscriber SHALL
NOT get the current status when it subscribes.

#### Scenario: Master change
- **WHEN** an instance goes from slave to master
- **THEN** each subscriber gets one notification with the state master

#### Scenario: Error changes alone
- **WHEN** two retries in `connectionLost` fail with two different messages
- **THEN** no notification is sent, and `status()` has the second message

#### Scenario: Slow subscriber
- **WHEN** a subscriber reads its notifications late, after three changes happened
- **THEN** it gets the three changes, in order

### Requirement: Subscribe before join
A subscriber added before `join` SHALL get `connecting`, then master or slave. The last
notification of an object SHALL be `left`.

#### Scenario: Subscribe, then join
- **WHEN** a subscriber is added and then `join` runs and finishes
- **THEN** the subscriber gets offline `connecting`, then master or slave

#### Scenario: Leave
- **WHEN** `leave` is called on a joined object
- **THEN** the last notification is offline `left`

### Requirement: States and reasons are strings
States and reasons SHALL have the string values `master`, `slave`, `offline`, `notJoined`,
`connecting`, `connectionLost`, `reconnecting` and `left` in every language, so a log line
shows the name, not a number.

#### Scenario: Log a reason
- **WHEN** an app prints the reason of an offline status
- **THEN** the text is for example `connectionLost`

### Requirement: Status changes in the log
In Rust, the core SHALL write each status change as a `tracing` event at level info, target
`doyen`, with the fields `group`, `state`, `reason`, `number` and `generation` (the last two
when known), and each failed database call as an event at level debug with the error. Log
events SHALL follow the `client-config` rule "Secrets are never shown".

#### Scenario: Master change in the log
- **WHEN** an instance becomes master
- **THEN** one info event has `state=master`, the group, its number and its generation

#### Scenario: Failed call in the log
- **WHEN** a renew fails with a network error and the status does not change
- **THEN** one debug event has the error message, and no info event is written

### Requirement: Warnings once at connect
The core SHALL send each warning an adapter returns at connect once, with its `DOYEN_` code
and a message, through the usual channel of the language. In Rust it SHALL be a `tracing`
event at level warn with target `doyen` and a `code` field. A warning SHALL NOT change the
status. Each adapter defines its own warning codes.

#### Scenario: Setup with a known risk
- **WHEN** the adapter returns one warning at connect
- **THEN** the app gets it once, before `join` finishes, and the status is not changed by it
