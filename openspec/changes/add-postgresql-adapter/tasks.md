# Tasks

## 0. Decision

- [ ] 0.1 (owner) Review the "to confirm (owner)" decisions in design.md (rows with a server-time expiry instead of advisory locks, the tables, the driver, PostgreSQL 14+) and approve them or ask for changes

## 1. Driver, config and tables

- [ ] 1.1 Add `tokio-postgres` with a TLS connector behind the feature `postgresql` (in the default features); verify `cargo build --no-default-features --features postgresql` and the `msrv` job
- [ ] 1.2 Write the PostgreSQL config (`schema`, URL schemes); verify the "PostgreSQL type and settings" scenario as a unit test
- [ ] 1.3 Write the connect checks (version, standby, timeline readable) and the table creation; verify the "Supported versions", "Tables created at connect" and "Timeline must be readable" scenarios against PostgreSQL containers (13, 14, a standby, a user without `CREATE`)
- [ ] 1.4 Commit as `feat(postgresql): add the config and the connect checks`

## 2. Operations

- [ ] 2.1 Write join (transaction, row lock, tag retry, n rule, `NOTIFY`); verify the "Join in one transaction" scenarios (`DOYEN_TEST_POSTGRESQL_URL`)
- [ ] 2.2 Write renew (one statement, `clock_timestamp()`, live line, timeline) and the generation rules; verify the "Places expire by the server clock" and "When the PostgreSQL generation changes" scenarios
- [ ] 2.3 Write leave and watch (`LISTEN doyen`, payload filter); verify the "Leave and wake-ups" scenarios (one with PgBouncer in transaction mode)
- [ ] 2.4 Run the conformance suite on the PostgreSQL adapter; verify every rule passes
- [ ] 2.5 Commit as `feat(postgresql): add the PostgreSQL adapter`

## 3. CI

- [ ] 3.1 Add the CI jobs `postgresql` (matrix 14 / latest) and a primary + standby failover (promote the standby) with `DOYEN_TEST_REQUIRE_DB=1`; verify the failover test shows a new, higher generation
- [ ] 3.2 Add the dump/restore test (restore into a new container); verify the token still grows
- [ ] 3.3 Commit as `test(postgresql): run the adapter on PostgreSQL in CI`

## 4. Managed services and documentation

- [ ] 4.1 Check that the timeline function works for a normal user on Amazon RDS, Google Cloud SQL and Azure Database for PostgreSQL (docs or a test account); record the result in design.md
- [ ] 4.2 Write the README PostgreSQL section (settings, versions, the table SQL, privileges, poolers, the documented limits) and update "Using the token"; verify the "Documented limits" scenario
- [ ] 4.3 Add `crates/doyen/examples/postgresql.rs`; verify it runs against a local PostgreSQL
- [ ] 4.4 Commit as `docs(postgresql): document the PostgreSQL adapter`

## 5. Bindings and release

- [ ] 5.1 Add `postgresql` to the Node.js `DatabaseType` and its config shape; verify the Node.js runtime test joins a PostgreSQL service
- [ ] 5.2 Add `PostgresConfig` to the Go package; verify `go test -race` joins a PostgreSQL service
- [ ] 5.3 Commit as `feat(node,go): add the postgresql database type`
- [ ] 5.4 Release the crate (next minor version), the npm package and the Go module with the release processes of `crate-release`, `node-binding` and `go-binding`; verify each one installs in a clean project
- [ ] 5.5 (owner) Publish the crate and the npm packages (2FA)

## Workflow follow-up

- Archive the change with `/opsx:archive add-postgresql-adapter` after the release.
