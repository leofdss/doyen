# Spec Delta

## Purpose

Defines the tests that run several real instances of doyen against a real database, with
real clocks and real faults, and check the promises of `election-core` from the outside.

## ADDED Requirements

### Requirement: Real members and a real database
The system tests SHALL run three member processes of a test program against a real Redis
server and against a real MongoDB replica set, with the default settings. Each member SHALL
write every status and token it sees on a timeline, with the time of a clock that every
process on the host shares (Linux `CLOCK_BOOTTIME`), and SHALL read its status at least
every 10 ms.

#### Scenario: Timeline
- **WHEN** a system test ends
- **THEN** it has one timeline per member, and the CI keeps the timelines as an artifact

### Requirement: One master at a time
In every system test, the master periods of two members SHALL NOT overlap by more than the
sampling step (10 ms), and the token of each new master SHALL be greater than the token of
every earlier master.

#### Scenario: Timelines checked
- **WHEN** a system test ends
- **THEN** the check finds no overlap of master periods and no token that went back

### Requirement: Master killed
When the master process is killed with `SIGKILL`, the next member SHALL be master 8 to 12 s
later, and the other members SHALL stay slaves.

#### Scenario: Kill the master
- **WHEN** the master of three members is killed
- **THEN** member 2 is master 8 to 12 s later, and member 3 stays slave

### Requirement: Database paused
A pause of the database shorter than 6 s SHALL change nothing. A pause longer than
`timeoutMs` SHALL make every member offline, and after the pause there SHALL be exactly one
master again.

#### Scenario: Short pause
- **WHEN** the database is paused for 5 s
- **THEN** no member goes offline and the master stays the same

#### Scenario: Long pause
- **WHEN** the database is paused for 15 s
- **THEN** every member is offline 7 to 9 s after the pause started, and within 10 s after its end there is exactly one master, with a higher token than before

### Requirement: Master leaves
After the master calls `leave`, the next member SHALL be master within 1 s.

#### Scenario: Leave hands over at once
- **WHEN** the master of three members calls `leave`
- **THEN** member 2 is master less than 1 s later

### Requirement: System tests in CI
The system tests SHALL run in CI against Redis and MongoDB on every pull request that
changes `crates/`, and every night. A failed check SHALL fail the run.

#### Scenario: Pull request
- **WHEN** a pull request changes a file in `crates/doyen/src/`
- **THEN** the CI job `system` runs the system tests for Redis and MongoDB
