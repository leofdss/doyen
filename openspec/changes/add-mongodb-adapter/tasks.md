# Tasks

## 1. Driver and config

- [ ] 1.1 Add the `mongodb` and `bson` crates behind the feature `mongodb`; verify `cargo build --no-default-features --features mongodb` and the `msrv` job
- [ ] 1.2 Write the MongoDB config (`database`, `collection`, URL schemes, defaults); verify the "MongoDB type and settings" scenarios as unit tests
- [ ] 1.3 Commit as `build(mongodb): add the MongoDB driver behind the mongodb feature`

## 2. Operations

- [ ] 2.1 Write join (upsert pipeline, `gen` at creation, tag retry, `version`); verify the "Join in one write" scenarios against a one-node replica set (`DOYEN_TEST_MONGODB_URL`)
- [ ] 2.2 Write renew (the pipeline in design.md, "place gone" rules); verify the "Renew in one write" scenarios
- [ ] 2.3 Write leave and watch (change stream filter, reopen on error, "no push" when not allowed); verify the "Leave" and "Wake-ups through a change stream" scenarios
- [ ] 2.4 Set `w: "majority"`, `wtimeout` and `maxTimeMS` (90% of the interval) on every write; verify with a test that reads the command options from the driver's command monitoring
- [ ] 2.5 Run the conformance suite on the MongoDB adapter; verify every rule passes
- [ ] 2.6 Commit as `feat(mongodb): add the MongoDB adapter`

## 3. Connect checks

- [ ] 3.1 Write the version, replica set / `mongos` and arbiter checks; verify the "Supported deployments" and "Warning for an arbiter" scenarios (MongoDB 4.4 and standalone containers, a PSA set)
- [ ] 3.2 Commit as `feat(mongodb): check the MongoDB setup at connect`

## 4. CI

- [ ] 4.1 Add the CI jobs `mongodb-rs` (one-node replica set, matrix 5.0 / latest), `mongodb-psa` and a three-node set with a failover (`rs.stepDown()`), all with `DOYEN_TEST_REQUIRE_DB=1`; verify the jobs pass and the failover test shows the same generation and numbers
- [ ] 4.2 Add the test "a deleted group document → the token still grows" and the 5.0 test "a pipeline update reaches the change stream with `updatedFields.version`"; verify both pass on 5.0
- [ ] 4.3 Commit as `test(mongodb): run the adapter on replica sets in CI`

## 5. Documentation

- [ ] 5.1 Update the README MongoDB section (settings, 5.0+, replica set, PSA warning, `mongos` limit, one busy document, the role doyen needs) and the restore case in "Using the token"; verify the "Restore from a backup" scenario
- [ ] 5.2 Add `crates/doyen/examples/mongodb.rs`; verify `cargo run --example mongodb --features mongodb` against a local replica set
- [ ] 5.3 Commit as `docs(mongodb): document the MongoDB adapter`

## Workflow follow-up

- Archive the change with `/opsx:archive add-mongodb-adapter` after the pull request is merged.
