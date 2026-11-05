# Design

## Context

The core (`add-core-election`) gives the `Backend` trait, the conformance suite and the
database test switch (`DOYEN_TEST_MONGODB_URL`). The behaviour is in the `mongodb-adapter`
spec.

## Goals / Non-Goals

**Goals:**
- Every adapter operation is one atomic write (renew = 1 round trip).
- Pass the conformance suite on a one-node replica set and on a three-node set.

**Non-Goals:**
- MongoDB older than 5.0 (4.2 and 4.4 are end of life, and the change stream filter needs
  the 5.0 update events for pipeline updates).
- The TTL index (its cleanup runs every 60 s; the expiry is checked in each write instead).
- Thousands of instances in one group (one busy document per group).

## Decisions

- **Driver:** the official `mongodb` crate (async, Tokio). Alternative: none maintained.
- **One document per group.** Alternative: one document per member + a counter document;
  rejected: the join needs a multi-document transaction, and the line read is not atomic
  with the renew.
- **Pipeline updates with `$$NOW`.** `$$NOW` exists only in aggregation expressions, and it
  is the same value in every stage of one operation. The pipelines are declarative, act on
  one document and store no code on the server (allowed by the owner; the no-script rule is
  for Redis).
- **Renew pipeline:**

  ```js
  [{ $set: { members: { $map: {
      input: { $filter: { input: "$members", cond: { $gt: ["$$this.expiresAt", "$$NOW"] } } },
      in: { $cond: [
        { $and: [{ $eq: ["$$this.number", NUMBER] }, { $eq: ["$$this.tag", TAG] }] },
        { $mergeObjects: ["$$this", { expiresAt: { $add: ["$$NOW", TTL_MS] } }] },
        "$$this"
      ] }
  } } } }]
  ```

- **Join pipeline:** `$set` stages: `seq` = `{ $add: [{ $ifNull: ["$seq", 0] }, 1] }`;
  `members` = live members, then `$concatArrays` with the new member unless a live member has
  my tag; `version` + 1 only when a member was added; `gen` = `{ $ifNull: ["$gen", { $toLong:
  "$$NOW" }] }`. Upsert, `returnDocument: "after"`. Two upserts on a new `_id` at the same
  moment: the server retries the one that gets a duplicate key error (the filter is an
  equality on `_id`); the adapter also retries once on a duplicate key error, to be safe.
- **Data format:** `format: 1` is set with `gen` when the document is created. The join and
  renew pipelines change nothing when `format` is higher (each `$set` keeps the old value
  under a `$cond` on `format`); the adapter reads `format` in the returned document and
  returns the error of `adapter-contract`.
- **CI replica sets:** GitHub service containers cannot pass `--replSet`, so the jobs start
  `mongod --replSet rs0` with `docker run` in a step and call `rs.initiate()`. The system
  tests (`add-system-tests`) use the same container.
- **Tag storage:** the 128-bit tag as a 32-character hex string (easy to read in the shell).
- **Change stream filter:** `$match` on `documentKey._id` = group and
  (`operationType` in insert, delete, replace, or `updateDescription.updatedFields.version`
  exists). On an error the stream is opened again; wake-ups are only hints.
- **Arbiter check:** `hello.arbiters` not empty → the warning. Through `mongos`, `hello` has
  no arbiters list: no check, documented.

## Risks / Trade-offs

- [PSA with one data node down] → no master until it is back; safe, warned, documented.
- [A restore from a backup brings back an old `gen`] → documented fix: delete the group
  documents after the restore.
- [One busy document] → each renew is a majority write on it; fine for tens of instances,
  documented.
- [Change stream not allowed for the user] → `watch` returns "no push" and the core uses its
  timer (slower takeover after `leave`); the README lists the `changeStream` and `find`
  actions in the role doyen needs.
