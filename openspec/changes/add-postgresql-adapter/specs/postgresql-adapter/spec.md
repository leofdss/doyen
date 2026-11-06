# Spec Delta

## Purpose

Defines how doyen keeps the line, the places and the generation in PostgreSQL tables, its
settings, which PostgreSQL setups it accepts or refuses, and its documented limits.

## ADDED Requirements

### Requirement: PostgreSQL type and settings
The PostgreSQL config SHALL have `type` `postgresql`, the shared settings of `client-config`,
and `schema` (default `public`). `url` SHALL use `postgres://` or `postgresql://`.

#### Scenario: Default schema
- **WHEN** the config has only `type`, `url` and `group`
- **THEN** doyen uses the tables `public.doyen_group` and `public.doyen_member`

### Requirement: Supported versions
At connect the adapter SHALL refuse a server older than PostgreSQL 14 with
`DOYEN_UNSUPPORTED`, and SHALL refuse a connection to a standby (read-only) server with
`DOYEN_UNSUPPORTED`.

#### Scenario: PostgreSQL 13
- **WHEN** the server is PostgreSQL 13
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message says that 14 or newer is needed

#### Scenario: Standby URL
- **WHEN** the URL points to a hot standby
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message says that doyen needs the primary

### Requirement: Tables created at connect
At connect the adapter SHALL create its two tables if they do not exist. When the user cannot
create them, `join` SHALL fail with `DOYEN_AUTH`, and the README SHALL give the SQL to create
them by hand.

#### Scenario: No CREATE privilege
- **WHEN** doyen's user has no `CREATE` privilege on the schema and the tables do not exist
- **THEN** `join` fails with `DOYEN_AUTH` and the message points to the SQL in the README

#### Scenario: Tables made by hand
- **WHEN** the tables exist and the user has only `SELECT`, `INSERT`, `UPDATE`, `DELETE` on them
- **THEN** every doyen operation works

### Requirement: Join in one transaction
A join SHALL run in one transaction that locks the group row: it SHALL remove dead members,
return my place if a live member has my tag, else insert me with number max(`seq`, highest
live number) + 1 and update `seq`, then notify. Numbers SHALL enter the line in commit order.
The group row SHALL hold the data format (`format`, see `adapter-contract`).

#### Scenario: Concurrent joins
- **WHEN** ten instances join at the same moment
- **THEN** they get ten different numbers, in the order their transactions committed

#### Scenario: Retry with the same tag
- **WHEN** a join retry finds a live member with my tag
- **THEN** it returns that number and inserts nothing

### Requirement: Places expire by the server clock
A renew SHALL be one statement that sets my `expires_at` to `clock_timestamp()` + `ttl` only
when my number and my tag are there and not expired, removes dead members, and returns the
live line and the group's generation. No row for me SHALL mean "place gone".

#### Scenario: Process crash
- **WHEN** the master process is killed
- **THEN** its row expires `timeoutMs` after its last renew, by the server clock, and the next member takes over

#### Scenario: Network partition
- **WHEN** the master is cut off from the database
- **THEN** its renews fail, it goes offline when its lease ends, and its row expires on the server; the others take over with no TCP keepalive needed

### Requirement: Leave and wake-ups
`leave` SHALL delete my row (my number and my tag) and send `NOTIFY doyen, '<group>'`. `watch`
SHALL `LISTEN doyen` on its own connection and wake only for its group.

#### Scenario: Master leaves
- **WHEN** the master leaves
- **THEN** the next member is woken, renews at once and becomes master

#### Scenario: Pooler in transaction mode
- **WHEN** the URL goes through a pooler that drops notifications
- **THEN** every operation still works and the next scheduled renew shows the change

### Requirement: When the PostgreSQL generation changes
The adapter SHALL make a new generation when the group row is missing, when the server's
timeline ID differs from the stored one, or when the stored `gen` or `seq` is lower than the
highest generation or number this adapter saw for the group. The new value SHALL be
max(`gen` + 1, the server time in ms, the highest generation seen + 1).

#### Scenario: Failover to a standby
- **WHEN** a standby is promoted (a new timeline)
- **THEN** the next join or renew makes a new generation higher than the old one

#### Scenario: Dump and restore into a new cluster
- **WHEN** the tables are restored with `pg_dump` into a new cluster (timeline 1)
- **THEN** the stored timeline differs, and the next join or renew makes a higher generation

### Requirement: Timeline must be readable
The adapter SHALL read the timeline ID of the server. When the user cannot read it, `join`
SHALL fail with `DOYEN_UNSUPPORTED` and name the function.

#### Scenario: Function not allowed
- **WHEN** the managed service refuses the function that gives the timeline
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message names the function

### Requirement: Documented limits
The README SHALL say: with asynchronous replication (the default) two masters can exist for a
moment after a failover (the token covers it); the token can go back only when the data went
back with the same timeline and no running instance saw the newer generation; and how to
reset the fence row.

#### Scenario: README
- **WHEN** a user reads the PostgreSQL section and "Using the token"
- **THEN** both limits and the fix are there
