# Proposal

## Why

The owner wants a second, very different database in the first release, to prove that the
adapter contract fits more than Redis before anything is published. MongoDB is common in
the same teams, and its majority writes make it safer than Redis: the token never goes
back in a failover.

Release: v0.1. Depends on: `add-core-election`.

It does not need the Redis adapter, but the two edit the same files: do them one after the
other (see TODO.md).

## What Changes

- A MongoDB adapter behind the cargo feature `mongodb`: one document per group, each
  operation one atomic write (aggregation pipeline updates with `$$NOW`), `w: "majority"`.
- Config settings `database` (default: the database in the URL path, else `doyen`) and
  `collection` (default `doyen`).
- Wake-ups through a change stream filtered on the group's `version` field.
- Checks at connect: MongoDB 5.0+, a replica set or `mongos`; the warning
  `DOYEN_MONGODB_ARBITER` for a primary-secondary-arbiter set.
- CI jobs with a one-node replica set (5.0 and the latest) and a PSA set.
- README: the MongoDB section and the "restore from a backup" note.

## Capabilities

### New Capabilities

- `mongodb-adapter`: how doyen keeps the line, the places and the generation in MongoDB,
  its settings, the setups it accepts, refuses or warns about, and its warning code.

### Modified Capabilities

(none)

## Impact

- `crates/doyen/src/mongodb/` (feature `mongodb`), dependency `mongodb` (the official Rust
  driver) and `bson`.
- `.github/workflows/ci.yml`: MongoDB jobs.
- `README.md`: MongoDB section updated from "planned" to the real behaviour.
