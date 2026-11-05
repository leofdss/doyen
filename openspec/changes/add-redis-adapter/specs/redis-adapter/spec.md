# Spec Delta

## Purpose

Defines how doyen keeps the line, the places and the generation in Redis with plain
commands, which Redis setups it accepts, refuses or warns about, and its Redis warning codes.

## ADDED Requirements

### Requirement: Redis type and URLs
The Redis config SHALL have `type` `redis` and only the shared settings of `client-config`.
`url` SHALL use `redis://` or `rediss://` (TLS) for one server or Redis Cluster, or
`redis+sentinel://` / `rediss+sentinel://` with the master name in the path for Sentinel.

#### Scenario: Sentinel URL
- **WHEN** the URL is `redis+sentinel://user:pass@s1:26379,s2:26379/mymaster`
- **THEN** doyen asks the sentinels for the primary of `mymaster` and logs in to it as `user`

#### Scenario: Sentinel login
- **WHEN** the URL has `?sentinelUsername=su&sentinelPassword=sp`
- **THEN** doyen logs in to the sentinels as `su` and to the data nodes with the user of the URL

### Requirement: Topology found at connect
With `redis://` or `rediss://`, doyen SHALL read `INFO cluster` at connect and use a Redis
Cluster client when `cluster_enabled` is 1. With Sentinel, doyen SHALL follow each failover
to the new primary.

#### Scenario: URL of a Cluster node
- **WHEN** the URL points to one node of a Redis Cluster
- **THEN** doyen works with the whole Cluster

#### Scenario: Sentinel failover
- **WHEN** Sentinel promotes a replica
- **THEN** the instances connect to the new primary and keep one master

### Requirement: Reconnect on READONLY
When a command fails with `READONLY` (the server is now a replica), the adapter SHALL close
the connection and connect again with a new name lookup. Meanwhile the status error SHALL
show the `READONLY` message.

#### Scenario: Managed endpoint moved
- **WHEN** the primary endpoint of a managed Redis now points to a new primary and the old connection gets `READONLY`
- **THEN** the adapter connects again and the renews work on the new primary

### Requirement: Minimum version
The adapter SHALL refuse a Redis server older than 6.2 with `DOYEN_UNSUPPORTED`.

#### Scenario: Redis 6.0
- **WHEN** the server is Redis 6.0
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message says that 6.2 or newer is needed

### Requirement: Keys of a group
All keys of a group SHALL start with `doyen:{group}:` (`meta` hash with `format`, `seq`, `gen`,
`runid`, `replid`; `line` sorted set; `alive:<number>` strings with an expiry and the value
`<tag> <instance name>`; `events` channel). Deleting them SHALL be safe: the members join
again with a higher generation.

#### Scenario: Shared Redis
- **WHEN** the app uses the same Redis for its own keys
- **THEN** doyen reads and writes only keys that start with `doyen:`

#### Scenario: Keys deleted by hand
- **WHEN** an operator deletes every `doyen:{billing}:*` key while members live
- **THEN** the members join again and the new master's token is higher than the old one

### Requirement: No server-side scripts
The adapter SHALL NOT use Lua scripts or Redis functions (`EVAL`, `EVALSHA`, `FCALL`,
`FUNCTION`). It SHALL use only plain commands and transactions.

#### Scenario: User without scripting
- **WHEN** the Redis user of doyen has `-@scripting`
- **THEN** every doyen operation still works

### Requirement: Atomic join
A join SHALL be one optimistic transaction (`WATCH` the `meta` and `line` keys, then `MULTI` /
`EXEC`). The new number SHALL be max(`seq`, highest number in the line) + 1, and SHALL skip a
number whose `alive` key still exists. A failed `EXEC` SHALL start the join again.

#### Scenario: Concurrent joins
- **WHEN** ten instances join at the same moment
- **THEN** they get ten different numbers and the line has them in number order

#### Scenario: Retry with the same tag
- **WHEN** a join retry finds an `alive` key with its own tag
- **THEN** it returns that number and writes nothing new

### Requirement: Renew only my own live place
A renew SHALL use `SET doyen:{group}:alive:<n> "<tag> <name>" XX PX <ttl> GET`. No old value
(the key expired) SHALL mean "place gone". An old value with another tag SHALL mean "place
gone" too.

#### Scenario: Key expired
- **WHEN** the `alive` key of a member expired before its renew
- **THEN** the renew writes nothing and returns "place gone"

#### Scenario: Another tag
- **WHEN** the `alive` key of my number holds another member's tag
- **THEN** the renew returns "place gone"

### Requirement: Live line and dead numbers
A renew SHALL return the live line from the `line` set and one `MGET` of the `alive` keys.
Dead numbers SHALL be removed from `line` in the next renew. A renew SHALL take 2 round
trips, 3 when the generation changes.

#### Scenario: Member expired
- **WHEN** the line has 1, 2, 3 and the key of 2 expired
- **THEN** the renew returns 1, 3, and number 2 is removed from `line` in a later renew

### Requirement: When the Redis generation changes
In each join and renew, the adapter SHALL make a new generation when `meta` is missing, when
the server's `run_id` or `master_replid` differs from the stored `runid` / `replid`, or when
the stored `gen` or `seq` is lower than the highest generation or number this adapter saw for
the group.

#### Scenario: Restart without persistence
- **WHEN** Redis restarts without persistence while members live
- **THEN** the next join or renew makes a new generation higher than the old one

#### Scenario: Old primary back with old data
- **WHEN** a primary that was not restarted becomes primary again with older data and the same `run_id`, and a running instance saw a higher generation
- **THEN** that instance makes a new generation higher than every generation it saw

#### Scenario: Counter went back
- **WHEN** the stored `seq` is lower than a number the adapter saw, with the same generation
- **THEN** the adapter makes a new generation

### Requirement: Value of a new Redis generation
A new generation SHALL be max(`gen` + 1, the server `TIME` in ms, the highest generation this
adapter saw + 1), written with `runid`, `replid` and `seq` = max(`seq`, the highest number
this adapter saw) in one transaction. A join SHALL return the valid generation.

#### Scenario: First master after a restart
- **WHEN** the first instance joins after a restart that lost the data
- **THEN** its first token already has the new generation

#### Scenario: Two instances change it at once
- **WHEN** two instances try to write a new generation at the same moment
- **THEN** one `EXEC` fails, that instance reads again, and both use the same new generation

### Requirement: Documented generation exception
The README SHALL say that the token can go back only when Redis goes back to older data and
no running instance saw the newer generation (for example all instances restarted), and how
to reset the fence row.

#### Scenario: README
- **WHEN** a user reads "Using the token" in the README
- **THEN** it names this Redis case and the fix

### Requirement: Wait for replicas
After a join and after a new generation, the adapter SHALL send `WAIT <n> <t>`, where n is the
number of replicas with `state=online` and t is 25% of `renewIntervalMs`. With no online
replica it SHALL skip `WAIT`. Fewer acks than n SHALL NOT be an error.

#### Scenario: Replica in a full sync
- **WHEN** one replica is still in its first full sync
- **THEN** it is not counted, and `join` finishes without waiting for it

#### Scenario: No replicas
- **WHEN** the server has no replicas
- **THEN** no `WAIT` is sent

### Requirement: Clean leave
`leave` SHALL remove the `alive` key and the number from `line` in one transaction only when
the key holds my tag, then publish on `events`.

#### Scenario: Place is not mine
- **WHEN** the `alive` key of my number holds another tag
- **THEN** `leave` removes nothing

### Requirement: Wake-ups through pub/sub
`watch` SHALL subscribe to `doyen:{group}:events`; join and leave SHALL publish on it.

#### Scenario: Master leaves
- **WHEN** the master leaves
- **THEN** the next member renews at once and becomes master without waiting for its timer

### Requirement: Eviction policy
At connect the adapter SHALL read the memory settings. With `maxmemory` set and an
`allkeys-*` policy it SHALL fail with `DOYEN_UNSUPPORTED`. With a `volatile-*` policy it
SHALL give the warning `DOYEN_REDIS_EVICTION`. `noeviction` or no limit SHALL be accepted
with no warning.

#### Scenario: allkeys-lru
- **WHEN** the server has `maxmemory 100mb` and `allkeys-lru`
- **THEN** `join` fails with `DOYEN_UNSUPPORTED`

#### Scenario: volatile-lru
- **WHEN** the server has `maxmemory 100mb` and `volatile-lru`
- **THEN** `join` works and the warning `DOYEN_REDIS_EVICTION` is sent once

### Requirement: Warning without min-replicas
At connect, when the primary has replicas, `min-replicas-to-write` is off and the server is
not part of a Redis Cluster, the adapter SHALL give the warning `DOYEN_REDIS_NO_MIN_REPLICAS`
(in a network partition, two masters for as long as it lasts).

#### Scenario: Sentinel without the setting
- **WHEN** a Sentinel primary with two replicas has `min-replicas-to-write 0`
- **THEN** the warning `DOYEN_REDIS_NO_MIN_REPLICAS` is sent once

#### Scenario: Redis Cluster
- **WHEN** the server is a Redis Cluster primary with replicas and the setting off
- **THEN** no `DOYEN_REDIS_NO_MIN_REPLICAS` warning is sent

### Requirement: Warning about Sentinel timing
With Sentinel and `min-replicas-to-write` on, the adapter SHALL read `down-after-milliseconds`
(`SENTINEL MASTER`) and `min-replicas-max-lag` (`CONFIG GET`, else 10 s), and SHALL give the
warning `DOYEN_REDIS_SENTINEL_TIMING` when `down-after-milliseconds` is less than
`min-replicas-max-lag` + `timeoutMs` + 1 s.

#### Scenario: Fast Sentinel
- **WHEN** `down-after-milliseconds` is 5000, `min-replicas-max-lag` is 10 and `timeoutMs` is 10000
- **THEN** the warning `DOYEN_REDIS_SENTINEL_TIMING` is sent once

#### Scenario: Default Sentinel
- **WHEN** `down-after-milliseconds` is 30000 with the same settings
- **THEN** no timing warning is sent

### Requirement: Needed INFO fields and Active-Active
The adapter SHALL fail with `DOYEN_UNSUPPORTED`, naming the field, when `INFO` lacks a field it
needs (`run_id`, `master_replid`, `maxmemory_policy`, `role`, `cluster_enabled`). It SHALL
refuse Redis Active-Active (several writable primaries) with `DOYEN_UNSUPPORTED`.

#### Scenario: Proxy hides run_id
- **WHEN** a proxy answers `INFO server` without `run_id`
- **THEN** `join` fails with `DOYEN_UNSUPPORTED` and the message names `run_id`

### Requirement: Access rules
A command refused by the Redis ACL (`NOPERM`) SHALL give `DOYEN_AUTH` with the command name.
The README SHALL list what doyen's user needs: keys `~doyen:*`, channels `&doyen:*`, the
commands it uses, `INFO`, `TIME`, `WAIT`, and `SENTINEL MASTER` on the sentinels.

#### Scenario: User without INFO
- **WHEN** doyen's user has `-@dangerous`
- **THEN** `join` fails with `DOYEN_AUTH` and the message names `INFO`

### Requirement: Redis Cluster keys in one slot
In Redis Cluster all keys of a group SHALL be in one slot (the `{group}` hash tag), and
`INFO` and `TIME` SHALL go to the node that owns that slot.

#### Scenario: Transaction in Cluster
- **WHEN** a join runs in a Redis Cluster
- **THEN** its transaction works with no `CROSSSLOT` error
