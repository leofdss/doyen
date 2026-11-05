# Tasks

Test first: in each group with behaviour, the first task writes the tests (each one names
its scenario: `// spec: postgresql-adapter / <scenario>`) and shows that they fail against a
real PostgreSQL; the next tasks write the code until they pass. Never change a test only to
make it pass.

## 0. Decision

- [ ] 0.1 (owner) Review the "to confirm (owner)" decisions in design.md (rows with a server-time expiry instead of advisory locks, the tables, the driver, PostgreSQL 14+) and approve them or ask for changes

## 1. Driver, config and connect checks

- [ ] 1.1 Add `tokio-postgres` with a TLS connector behind the feature `postgresql` (in the default features); verify `cargo build --no-default-features --features postgresql` and the `msrv` job
- [ ] 1.2 Write the tests: "Default schema", "Standby URL", "No CREATE privilege", "Tables made by hand", "Function not allowed", and a PostgreSQL 13 server refused (PostgreSQL containers: 13, 14, a standby, limited users); verify they fail (red)
- [ ] 1.3 Write the PostgreSQL config (`schema`, URL schemes), the connect checks (version, standby, timeline readable) and the table creation; verify the tests pass (green)
- [ ] 1.4 Commit the group in one commit (tests, code, spec changes, ticks): `feat(postgresql): add the config and the connect checks`

## 2. Operations

- [ ] 2.1 Write the operation tests (`DOYEN_TEST_POSTGRESQL_URL`): "Concurrent joins", "Retry with the same tag", "Process crash", "Network partition", "Master leaves", "Pooler in transaction mode" (PgBouncer), "Failover to a standby", "Dump and restore into a new cluster"; add the PostgreSQL adapter to the conformance suite; give the adapter stub bodies and verify every test fails (red)
- [ ] 2.2 Write join (transaction, row lock, tag retry, n rule, `NOTIFY`); verify the join tests pass
- [ ] 2.3 Write renew (one statement, `clock_timestamp()`, live line, timeline) and the generation rules; verify the expiry and generation tests pass
- [ ] 2.4 Write leave and watch (`LISTEN doyen`, payload filter); verify the remaining tests and the whole conformance suite pass (green)
- [ ] 2.5 Commit the group in one commit (tests, code, spec changes, ticks): `feat(postgresql): add the PostgreSQL adapter`

## 3. CI

- [ ] 3.1 Add the CI jobs `postgresql` (14 / latest), a primary + standby failover (promote the standby) and the dump/restore test, with `DOYEN_TEST_REQUIRE_DB=1`; verify they pass and the failover test shows a new, higher generation
- [ ] 3.2 Commit as `test(postgresql): run the adapter on PostgreSQL in CI`

## 4. Managed services and documentation

- [ ] 4.1 Check that the timeline function works for a normal user on Amazon RDS, Google Cloud SQL and Azure Database for PostgreSQL (docs or a test account); record the result in design.md
- [ ] 4.2 Write the README PostgreSQL section (settings, versions, the table SQL, privileges, poolers, the documented limits) and update *Using the token*; add the scenario "README" to `openspec/manual-checks.md` with how it was checked
- [ ] 4.3 Add `crates/doyen/examples/postgresql.rs`; verify it runs against a local PostgreSQL
- [ ] 4.4 Commit as `docs(postgresql): document the PostgreSQL adapter`

## 5. Bindings and release

- [ ] 5.1 Write the binding tests first: the Node.js runtime test and a Go test join a PostgreSQL service with type `postgresql`; verify they fail (red)
- [ ] 5.2 Add `postgresql` to the Node.js `DatabaseType` and its config shape, and `PostgresConfig` to the Go package; verify both tests pass (green)
- [ ] 5.3 Commit the group in one commit (tests, code, spec changes, ticks): `feat(node,go): add the postgresql database type`
- [ ] 5.4 Run `python3 scripts/check-scenario-tests.py --change add-postgresql-adapter`; verify every `postgresql-adapter` scenario is named by a test or a manual check
- [ ] 5.5 (owner) After the merge of this change, merge the release pull request (0.4.0 by the plan) and approve the environment `release` (`release-process`); verify the crate, the npm package and the Go module install in a clean project

## Workflow follow-up

- Archive the change with `/opsx:archive add-postgresql-adapter` after the release.
