# Tasks

Test first: in each group with behaviour, the first task writes the tests (each one names
its scenario: `// spec: mongodb-adapter / <scenario>`) and shows that they fail against a
real replica set; the next tasks write the code until they pass. Never change a test only to
make it pass.

## 1. Driver and config

- [ ] 1.1 Add the `mongodb` and `bson` crates behind the feature `mongodb`; verify `cargo build --no-default-features --features mongodb` and the `msrv` job
- [ ] 1.2 Write the config tests for "URL without a database" and "Database in the URL path"; verify they fail (red)
- [ ] 1.3 Write the MongoDB config (`database`, `collection`, URL schemes, defaults); verify the tests pass (green)
- [ ] 1.4 Commit the tests (`test(mongodb): add the config tests`), then the code (`feat(mongodb): add the MongoDB config`)

## 2. Operations

- [ ] 2.1 Write the operation tests against a one-node replica set (`DOYEN_TEST_MONGODB_URL`): "Two groups", "Secondary down in a three-node set", "App clock is wrong", "First join of a group", "Retry with the same tag", "Number given twice", "Document deleted", "Leave", "Renew", "Master leaves", plus a test that reads the write options (`w: "majority"`, `wtimeout`, `maxTimeMS` = 90%) from the driver's command monitoring; add the MongoDB adapter to the conformance suite; give the adapter stub bodies and verify every test fails (red)
- [ ] 2.2 Write join (upsert pipeline, `gen` at creation, tag retry, `version`); verify the join tests pass
- [ ] 2.3 Write renew (the pipeline in design.md, "place gone" rules) and the write options; verify the renew and write-option tests pass
- [ ] 2.4 Write leave and watch (change stream filter, reopen on error, "no push" when not allowed); verify the remaining tests and the whole conformance suite pass (green)
- [ ] 2.5 Commit the tests (`test(mongodb): add the operation tests and the conformance run`), then the code (`feat(mongodb): add the MongoDB adapter`)

## 3. Connect checks

- [ ] 3.1 Write the connect tests: "Standalone server", "MongoDB 4.4", "PSA set", "Through mongos"; verify they fail (red)
- [ ] 3.2 Write the version, replica set / `mongos` and arbiter checks; verify the tests pass (green)
- [ ] 3.3 Commit the tests (`test(mongodb): add the connect check tests`), then the code (`feat(mongodb): check the MongoDB setup at connect`)

## 4. CI

- [ ] 4.1 Write the failover tests: "Failover" (`rs.stepDown()` on a three-node set: same generation and numbers), a deleted group document → the token still grows, and the 5.0 test "a pipeline update reaches the change stream with `updatedFields.version`"; verify the tests that need code not written yet fail
- [ ] 4.2 Add the CI jobs `mongodb-rs` (one-node replica set, 5.0 / latest), `mongodb-psa` and the three-node failover job, all with `DOYEN_TEST_REQUIRE_DB=1`; fix the code until the tests pass; verify the jobs pass
- [ ] 4.3 Commit the tests (`test(mongodb): run the adapter on replica sets in CI`), then any code fix (`fix(mongodb): …`)

## 5. Documentation

- [ ] 5.1 Update the README MongoDB section (settings, 5.0+, replica set, PSA warning, `mongos` limit, one busy document, the role doyen needs) and the restore case in *Using the token*; add the scenario "README" to `openspec/manual-checks.md` with how it was checked
- [ ] 5.2 Add `crates/doyen/examples/mongodb.rs`; verify `cargo run --example mongodb --features mongodb` against a local replica set
- [ ] 5.3 Commit as `docs(mongodb): document the MongoDB adapter`

## 6. Integration check

- [ ] 6.1 Run `python3 scripts/check-scenario-tests.py --change add-mongodb-adapter`; verify every scenario of `mongodb-adapter` is named by a test or by `openspec/manual-checks.md`

## Workflow follow-up

- Archive the change with `/opsx:archive add-mongodb-adapter` after the pull request is merged.
