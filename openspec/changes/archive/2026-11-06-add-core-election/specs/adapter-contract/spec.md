# Spec Delta

## Purpose

Defines the rules every database adapter must follow (connect, join, renew, leave, watch),
so the core can use any database the same way, and the test suite that checks them.

## ADDED Requirements

### Requirement: Five operations
An adapter SHALL offer exactly these operations to the core: `connect` (check the setup),
`join` (get a number and a place), `renew` (keep my place and read the live line), `leave`
(remove my place and wake the others) and `watch` (wake-ups when someone joins or leaves).
The master rule SHALL stay in the core.

#### Scenario: Adapter decides nothing
- **WHEN** an adapter returns the live line
- **THEN** the core, not the adapter, decides which member is master

### Requirement: A member is its number
An adapter SHALL name each member by its number, never by its instance name, so removing a
dead number never removes a newer membership of the same instance.

#### Scenario: Instance comes back with the same name
- **WHEN** instance `pod-a` had number 3, lost its place and joined again as number 7
- **THEN** removing the dead number 3 does not touch number 7

### Requirement: A membership is a number and a tag
`renew` and `leave` SHALL act only on a place whose tag is the caller's tag. If the database
gave one number twice (after a data loss), the member that sees another tag SHALL get "place
gone" and join again; two members SHALL never share one place.

#### Scenario: Number given twice
- **WHEN** after a data loss member B gets the number that member A still renews
- **THEN** at most one of them keeps the place, the other one joins again, and the line never has two owners of that number

### Requirement: Atomic join in number order
`join` SHALL create the number and the place in one atomic step, and numbers SHALL enter the
line in increasing order. The new number SHALL be above every number in the line, even when
the database counter went back.

#### Scenario: Two joins at the same time
- **WHEN** two instances join at the same moment
- **THEN** they get two different numbers, and the line has them in number order

#### Scenario: Counter went back
- **WHEN** the counter is 2 but the line has number 5
- **THEN** the next join gets a number above 5

### Requirement: Join is idempotent by tag
A `join` with a tag that a live place already has SHALL return that place, never a second
one.

#### Scenario: Retry after a lost answer
- **WHEN** a join wrote its place but the answer was lost, and the core retries with the same tag
- **THEN** the adapter returns the same number and the line has one place for that tag

### Requirement: The generation never goes back
The generation an adapter returns SHALL never be lower than one it returned before for the
same group, also when the database lost data or the record that holds the generation. Each
adapter SHALL document its rare exceptions. `join` SHALL return a valid generation too.

#### Scenario: Generation record deleted
- **WHEN** someone deletes the record that holds the generation while members live
- **THEN** the next generation the adapter returns is higher than the old one

#### Scenario: First join after a restart
- **WHEN** the database restarted and lost data, and the first instance joins
- **THEN** the join already returns a generation higher than before the restart

### Requirement: The database decides who is alive
A place SHALL be alive by the database's own rule: the server time of the last write +
`ttl`, or a live connection. The clocks of the app machines SHALL NOT matter.

#### Scenario: App clock is wrong
- **WHEN** one app machine's clock is 1 hour ahead
- **THEN** its place expires at the same moment as with a correct clock

### Requirement: A dead membership never comes back
`renew` SHALL fail (place gone) after the place expired, and SHALL NOT create it again.

#### Scenario: Renew after expiry
- **WHEN** a place expired and its owner sends a renew
- **THEN** `renew` returns "place gone" and the line does not have that number

### Requirement: Live line only, in number order
The line returned by `join` and `renew` SHALL have only live members, in increasing number
order.

#### Scenario: A member expired
- **WHEN** member 2 of the line 1, 2, 3 expired
- **THEN** the returned line is 1, 3

### Requirement: Wake-ups carry no data
A wake-up from `watch` SHALL carry no data; the core SHALL always read again with `renew`. An
adapter with no push SHALL say so, and the core then uses only its timer.

#### Scenario: Lost wake-up
- **WHEN** a wake-up message is lost
- **THEN** the next scheduled renew still shows the change

### Requirement: Every call can be cancelled
Every adapter call SHALL be safe to cancel at any point. A join cancelled after its write
SHALL leave at most one place, which a retry with the same tag finds, or which expires after
`ttl`.

#### Scenario: Join cancelled by a timeout
- **WHEN** a join call is cancelled after the database wrote the place and no retry comes
- **THEN** the place expires after `ttl`

### Requirement: Connect checks the setup and writes no group data
`connect` SHALL check the database setup (version, topology, settings), SHALL fail with
`DOYEN_UNSUPPORTED` or `DOYEN_AUTH` for a refused setup, and SHALL return the warnings for a
setup with a known risk. It SHALL NOT change any group's line, places or generation; it can
create the adapter's own empty tables or collections.

#### Scenario: Old server version
- **WHEN** the server is older than the adapter's minimum version
- **THEN** `connect` fails with `DOYEN_UNSUPPORTED` and the message names the minimum version

### Requirement: Data format of a group
Each adapter SHALL store a data format number with the group data (format 1 in v0.1) and
SHALL write only its own format, in a group that has no data or the same format. When the
stored format is higher, `join` SHALL fail with `DOYEN_UNSUPPORTED` naming both formats, and
`renew` SHALL return an error, not "place gone", so the instance goes offline at the end of
its lease; the adapter SHALL write nothing to the group after it saw the newer format.

#### Scenario: Newer format at join
- **WHEN** the group data has format 2 and the adapter knows format 1
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message names format 2 and format 1

#### Scenario: Newer format while running
- **WHEN** a member renews and the group data now has format 2
- **THEN** the renew returns an error, the instance goes offline when its lease ends, and it writes nothing more to the group

### Requirement: Conformance suite
One test suite SHALL check every rule of this capability, and every adapter SHALL pass it in
CI against a real database. An in-memory adapter with a fake clock SHALL pass the same suite
and SHALL be used for the core tests.

#### Scenario: New adapter
- **WHEN** a new adapter is added
- **THEN** CI runs the conformance suite on it, and a failing rule fails the run

#### Scenario: Core tests without a database
- **WHEN** the core tests run on a machine with no database
- **THEN** they use the in-memory adapter and the fake clock, and they pass
