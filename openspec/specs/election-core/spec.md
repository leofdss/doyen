# election-core Specification

## Purpose

Defines how instances of one group agree on one master through a database: the seniority
line, the local lease, renews, the membership tag, the fencing token, join and leave.

## Requirements

### Requirement: Master is the oldest live member
An instance SHALL be master only while its number is the first number of the live line and
its local lease is valid. With a valid lease and another number first, it SHALL be slave.
Instances SHALL NOT vote: every instance reads the same line and gets the same answer.

#### Scenario: Three members
- **WHEN** members with numbers 1, 2 and 3 are live in the group
- **THEN** member 1 is master and members 2 and 3 are slaves

#### Scenario: Master dies
- **WHEN** member 1 stops renewing and its place expires in the database
- **THEN** member 2 becomes master at its next renew

#### Scenario: Old master comes back
- **WHEN** member 1 comes back after its place expired
- **THEN** it gets a new number above 3, joins at the end of the line and is slave

### Requirement: Join takes a place at the end of the line
`join` SHALL give the instance a new number above every number in the live line, and SHALL
finish when the instance has a place, with the status master or slave. A new member SHALL
never change who is master.

#### Scenario: Join while a master exists
- **WHEN** member 1 is master and a new instance calls `join`
- **THEN** the new instance gets a number above 1, `join` finishes and its status is slave

#### Scenario: First member
- **WHEN** an instance joins an empty group
- **THEN** `join` finishes and its status is master

### Requirement: Join retries errors it can fix
While `join` has no place, the core SHALL retry every network error and keep the reason
`connecting`. Besides the one-life rules and a cancel, `join` SHALL fail only on an error a
retry cannot fix: a bad config (`DOYEN_INVALID_CONFIG`), a refused login or command
(`DOYEN_AUTH`), or a refused setup (`DOYEN_UNSUPPORTED`). After such a failure the status SHALL
be offline `notJoined`.

#### Scenario: Database down at join
- **WHEN** `join` is called while the database does not answer, and the database starts 30 s later
- **THEN** the status is offline with reason `connecting` during the 30 s, and `join` finishes after the database answers

#### Scenario: Wrong password at join
- **WHEN** `join` is called with a wrong password
- **THEN** `join` fails with `DOYEN_AUTH` and the status is offline with reason `notJoined`

### Requirement: After join the core never gives up
After `join` finished, the core SHALL retry every database error, also a refused login, until
`leave` is called. It SHALL show the last error in the status.

#### Scenario: Password changed while running
- **WHEN** the database password changes after `join` finished
- **THEN** the instance goes offline when its lease ends, keeps trying, and shows the login error in the status

#### Scenario: Password fixed
- **WHEN** the password is fixed while the instance is offline
- **THEN** the instance gets a place again without a new `join` call

### Requirement: Local lease
The local lease SHALL end at the send time of the last successful renew + `timeoutMs` − a
margin of 10% of `timeoutMs`. The core SHALL measure it on a clock that counts suspend time
(Linux `CLOCK_BOOTTIME`). When the lease ends with no successful renew, the instance SHALL be
offline. The database expiry starts later (when the write arrives), so the master stops first.

#### Scenario: Database stops answering
- **WHEN** the database stops answering and the defaults are used (10000 ms, 2000 ms)
- **THEN** the status turns offline 7 to 9 s after the database stopped answering

#### Scenario: Host suspended
- **WHEN** the host is suspended for 20 s while the instance is master
- **THEN** after the resume the instance is not master before a new successful renew

### Requirement: Lease of a join
The local lease given by a join SHALL start at the send time of the first join attempt with
the same tag, because an earlier attempt may have written the place. If less than one renew
interval of that lease is left, the core SHALL renew at once.

#### Scenario: Join retried after a timeout
- **WHEN** a join attempt times out after the database wrote the place, and the retry finds that place
- **THEN** the local lease is counted from the send time of the first attempt

### Requirement: Renews at a fixed rate
The core SHALL send a renew every `renewIntervalMs`, counted from the join, whatever the
answer time. Each database call SHALL time out after `renewIntervalMs`, so at most one renew
runs at a time. An extra renew after a wake-up SHALL NOT move the schedule.

#### Scenario: Slow answers at the smallest timeout
- **WHEN** `timeoutMs` is 3 × `renewIntervalMs` and every renew answers just before its call timeout
- **THEN** the status never turns offline

#### Scenario: Wake-up between renews
- **WHEN** a wake-up comes 0.5 s after a scheduled renew
- **THEN** the core renews at once, and the next scheduled renew is still one interval after the last scheduled one

### Requirement: Takeover times with the defaults
With the default settings, after a crash of the master the next member SHALL become master 8
to 12 s after the crash. After `leave`, it SHALL become master at its wake-up (at once), or at
its next renew when the adapter has no push. A database problem shorter than 6 s SHALL NOT
change the master.

#### Scenario: Master crashes
- **WHEN** the master process is killed and the defaults are used
- **THEN** the next member is master 8 to 12 s later

#### Scenario: Short database problem
- **WHEN** the database does not answer for 5 s and then answers again
- **THEN** the master stays the same and no member goes offline

### Requirement: Wake-ups come back after an error
When the adapter's `watch` stream ends or fails, the core SHALL open it again with the same
backoff as a join retry, and SHALL use only its timer meanwhile. This SHALL NOT change the
status.

#### Scenario: Watch stream ends
- **WHEN** the watch stream of a slave ends
- **THEN** the core opens it again, the status stays slave, and a later `leave` of the master still wakes this member at once

### Requirement: A lost place means a new join at the end
When a renew says the place is gone (it expired, or it has another tag), or the instance's
number is not in the live line, the core SHALL join again with a new membership, at the end
of the line, and the status SHALL be offline with reason `reconnecting` until it has a place.

#### Scenario: Database lost the data
- **WHEN** the database loses all doyen data while an instance is master
- **THEN** the instance goes offline with reason `reconnecting`, then joins again and gets a new place

### Requirement: Membership tag
Each membership SHALL have a random 128-bit tag. The object SHALL keep the tag until a join
returns a place: the retries of one join, and a new `join` after a cancelled or failed one,
SHALL use the same tag. A new membership (after a lost place) SHALL get a new tag.

#### Scenario: Join cancelled after its write
- **WHEN** a join is cancelled after the database wrote its place, and `join` is called again on the same object
- **THEN** the new join finds the same place and the line has no second place for this instance

### Requirement: Fencing token
`token()` SHALL return `(generation, number)` only while the status is master, checked at the
moment of the call, and nothing otherwise. Tokens SHALL be compared generation first, then
number. A token change SHALL NOT send a status notification.

#### Scenario: Master reads its token
- **WHEN** an instance with number 1 is master in generation g
- **THEN** `token()` returns `(g, 1)`

#### Scenario: Lease ended
- **WHEN** the local lease of the master ended
- **THEN** `token()` returns nothing at once, before any database answer

#### Scenario: Generation grows while master
- **WHEN** the adapter gives a higher generation while the instance stays master
- **THEN** the next `token()` returns the new generation, and no status notification is sent

### Requirement: Tokens of later masters are higher
The token of a later master SHALL be greater than every token an earlier master of the same
group got, except in the cases each adapter documents.

#### Scenario: Takeover in the same generation
- **WHEN** master 1 dies and member 2 becomes master in the same generation g
- **THEN** the new token `(g, 2)` is greater than `(g, 1)`

#### Scenario: New generation
- **WHEN** the adapter moves the group to a higher generation g2
- **THEN** every token in g2 is greater than every token in the old generation

### Requirement: Leave stops first, then removes the place
`leave` SHALL first end the local lease (the status is no longer master and `token()` returns
nothing), then remove the place and wake the other members. It SHALL finish when the database
removed the place, or after one call timeout, and SHALL NOT fail on a network error.

#### Scenario: Master leaves
- **WHEN** the master calls `leave`
- **THEN** its status is not master before the database removes the place, and the next member becomes master without waiting for an expiry

#### Scenario: Database down during leave
- **WHEN** `leave` is called while the database does not answer
- **THEN** `leave` finishes after one call timeout with no error, and the place expires after `timeoutMs`

### Requirement: One object, one life
`leave` SHALL work from every status and end in offline with reason `left`. A pending `join`
SHALL fail with `DOYEN_LEFT`. `join` after `leave` SHALL fail with `DOYEN_LEFT`. A second
`leave` SHALL finish at once. A second `join` while joined SHALL fail with
`DOYEN_ALREADY_JOINED`.

#### Scenario: Leave during join
- **WHEN** `leave` is called while `join` is still connecting
- **THEN** `join` fails with `DOYEN_LEFT` and the status is offline with reason `left`

#### Scenario: Join again on the same object
- **WHEN** `join` is called after `leave`
- **THEN** it fails with `DOYEN_LEFT`

#### Scenario: Join twice
- **WHEN** `join` is called twice on one object and the first call did not fail
- **THEN** the second call fails with `DOYEN_ALREADY_JOINED`

### Requirement: Status and token at the moment of the call
`status()` and `token()` SHALL compute their answer from the lease and the last known line at
the moment of the call, and SHALL NOT wait for a database call or a task.

#### Scenario: Renew task stuck
- **WHEN** a renew call hangs and the lease end time passes
- **THEN** `status()` returns offline at once

#### Scenario: Renew task panics
- **WHEN** the renew task stops because of a panic
- **THEN** nothing renews, the status turns offline when the lease ends, and the place expires in the database
