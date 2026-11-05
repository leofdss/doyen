# Spec Delta

## Purpose

Defines how doyen keeps the line, the places and the generation in one MongoDB document per
group, its settings, which MongoDB setups it accepts, refuses or warns about, and its
warning code.

## ADDED Requirements

### Requirement: MongoDB type and settings
The MongoDB config SHALL have `type` `mongodb`, the shared settings of `client-config`, and
`database` (default: the database in the URL path, else `doyen`) and `collection` (default
`doyen`). `url` SHALL use `mongodb://` or `mongodb+srv://`.

#### Scenario: URL without a database
- **WHEN** the URL is `mongodb+srv://cluster0.example.net/?retryWrites=true` and `database` is not set
- **THEN** doyen uses the database `doyen` and the collection `doyen`

#### Scenario: Database in the URL path
- **WHEN** the URL is `mongodb://h1,h2,h3/app?replicaSet=rs0`
- **THEN** doyen uses the database `app`

### Requirement: Supported deployments
At connect the adapter SHALL refuse, with `DOYEN_UNSUPPORTED`, a server older than MongoDB
5.0 and a standalone server (`hello` shows no `setName` and is not `mongos`). A replica set
and a sharded cluster SHALL be accepted.

#### Scenario: Standalone server
- **WHEN** the URL points to a standalone `mongod`
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message says that a replica set is needed (one node is enough for local work)

#### Scenario: MongoDB 4.4
- **WHEN** the server is MongoDB 4.4
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message says that 5.0 or newer is needed

### Requirement: One document per group
The adapter SHALL keep a group in one document with `_id` = the group name and the fields
`gen`, `seq`, `version` and `members` (each with `number`, `tag`, `id`, `expiresAt`, in
number order). Each adapter operation SHALL be one atomic write on that document.

#### Scenario: Two groups
- **WHEN** the groups `billing` and `mail` are used
- **THEN** the collection has two documents with these `_id` values

### Requirement: Majority writes with bounded waits
Every write SHALL use `w: "majority"`. `wtimeout` and `maxTimeMS` SHALL be 90% of
`renewIntervalMs`, so the server stops a blocked write before the core gives up.

#### Scenario: Secondary down in a three-node set
- **WHEN** one of three data nodes is down
- **THEN** the writes still get a majority and the master does not change

### Requirement: Server time for the expiry
`expiresAt` SHALL always come from `$$NOW` (the server clock) + `ttl`, inside the update
pipeline, never from the app's clock.

#### Scenario: App clock is wrong
- **WHEN** the app machine's clock is 1 hour behind
- **THEN** the member's `expiresAt` is the server time + `ttl`

### Requirement: Join in one write
A join SHALL be one `findOneAndUpdate` with upsert on the group document: `seq` + 1 (1 for a
new document), the live members kept, me added at the end with the new number, `version` + 1,
and `gen` set only when the document is created. If a live member has my tag, it SHALL add
nothing and return that number.

#### Scenario: First join of a group
- **WHEN** the group document does not exist
- **THEN** the join creates it with `seq` 1, `gen` = `$$NOW` in ms, and me as number 1

#### Scenario: Retry with the same tag
- **WHEN** a join retry finds a live member with my tag
- **THEN** it returns that number and adds no member

### Requirement: Renew in one write
A renew SHALL be one `findOneAndUpdate` with no upsert that keeps only live members and sets
my `expiresAt` when my number and my tag are there. If they are not in the returned document,
or the document does not exist, the renew SHALL return "place gone".

#### Scenario: Number given twice
- **WHEN** the document has my number with another tag
- **THEN** the renew returns "place gone" and does not change that member

#### Scenario: Document deleted
- **WHEN** someone deleted the group document
- **THEN** the renew returns "place gone", and the next join creates the document with a higher generation

### Requirement: Leave
`leave` SHALL remove the member that has my number and my tag (`$pull`) and add 1 to
`version`.

#### Scenario: Leave
- **WHEN** a member leaves
- **THEN** the document has no member with its number and tag, and `version` grew by 1

### Requirement: Wake-ups through a change stream
`watch` SHALL open a change stream on the collection, filtered to the group's document and to
inserts, deletes and updates that change `version`. Renews SHALL NOT change `version`, so
they wake nobody.

#### Scenario: Renew
- **WHEN** a member renews
- **THEN** no other member is woken

#### Scenario: Master leaves
- **WHEN** the master leaves
- **THEN** the next member is woken, renews at once and becomes master

### Requirement: Generation of a group
The generation SHALL be the time the group document was created (`$$NOW` in ms) and SHALL not
change while the document lives. A new document SHALL get a higher generation.

#### Scenario: Failover
- **WHEN** the primary fails over to a secondary
- **THEN** the generation and the numbers stay the same, and the token does not go back

### Requirement: Restore from a backup
The README SHALL say that a restore of doyen's collection from a backup can make the token go
back, and that the fix is to delete the group documents after the restore.

#### Scenario: README
- **WHEN** a user reads "Using the token" in the README
- **THEN** it names the restore case and the fix

### Requirement: Warning for an arbiter
At connect, when `hello` shows arbiters, the adapter SHALL give the warning
`DOYEN_MONGODB_ARBITER`: with one data node down, majority writes fail and no instance can be
master. It SHALL NOT refuse the setup.

#### Scenario: PSA set
- **WHEN** the replica set has a primary, a secondary and an arbiter
- **THEN** `join` works and the warning `DOYEN_MONGODB_ARBITER` is sent once

#### Scenario: Through mongos
- **WHEN** the URL points to `mongos`
- **THEN** no arbiter check is made (the README says that the shards' arbiters are not seen)
