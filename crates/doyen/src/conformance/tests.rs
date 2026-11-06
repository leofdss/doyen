// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests of the conformance suite itself and of the database test switch.

use std::time::Duration;

use async_trait::async_trait;
use futures_util::stream::BoxStream;

use super::*;
use crate::backend::{Result, Warning};

/// An adapter that is broken on purpose: every call fails.
struct StubBackend;

fn stub_error() -> BackendError {
    BackendError::Network("the stub adapter does nothing".to_owned())
}

#[async_trait]
impl Backend for StubBackend {
    async fn connect(&self) -> Result<Vec<Warning>> {
        Err(stub_error())
    }

    async fn join(
        &self,
        _group: &str,
        _id: &str,
        _tag: u128,
        _ttl: Duration,
    ) -> Result<(u64, View)> {
        Err(stub_error())
    }

    async fn renew(&self, _group: &str, _me: &Membership, _ttl: Duration) -> Result<Option<View>> {
        Err(stub_error())
    }

    async fn leave(&self, _group: &str, _me: &Membership) -> Result<()> {
        Err(stub_error())
    }

    async fn watch(&self, _group: &str) -> Result<Option<BoxStream<'static, ()>>> {
        Err(stub_error())
    }
}

/// The database of the stub adapter: its hooks do nothing.
struct StubDatabase;

impl TestDatabase for StubDatabase {
    type Adapter = StubBackend;

    async fn new_adapter(&self) -> StubBackend {
        StubBackend
    }

    fn group_name(&self, name: &str) -> String {
        name.to_owned()
    }

    async fn lose_all_data(&self, _group: &str) {}

    async fn delete_generation_record(&self, _group: &str) {}

    async fn set_counter(&self, _group: &str, _value: u64) {}

    async fn write_format(&self, _group: &str, _format: u32) {}

    async fn adapter_on_old_server(&self) -> (StubBackend, String) {
        (StubBackend, "1.0".to_owned())
    }
}

async fn start_stub_database() -> Option<StubDatabase> {
    tokio::time::pause();
    Some(StubDatabase)
}

/// Every rule of the suite on the broken adapter: each generated test passes only when its
/// rule fails, so a new adapter that breaks a rule fails the run.
mod every_rule_fails_on_a_broken_adapter {
    use super::*;

    // spec: adapter-contract / New adapter
    crate::conformance::conformance_tests!(start_stub_database, every_rule_fails);
}

const REDIS_URL_VARIABLE: &str = "DOYEN_TEST_REDIS_URL";

// spec: adapter-contract / Core tests without a database
#[test]
fn without_a_database_url_the_database_tests_are_skipped() {
    let decision = choose_database_url(REDIS_URL_VARIABLE, None, None);

    assert_eq!(decision, Ok(None));
}

#[test]
fn a_required_database_without_a_url_is_a_failure() {
    let decision = choose_database_url(REDIS_URL_VARIABLE, None, Some("1".to_owned()));

    let message = decision.expect_err("a missing URL must fail");
    assert!(message.contains(REDIS_URL_VARIABLE), "{message}");
    assert!(message.contains("DOYEN_TEST_REQUIRE_DB"), "{message}");
}

#[test]
fn an_empty_url_counts_as_missing() {
    let skipped = choose_database_url(REDIS_URL_VARIABLE, Some(String::new()), None);
    let failed = choose_database_url(
        REDIS_URL_VARIABLE,
        Some(String::new()),
        Some("1".to_owned()),
    );

    assert_eq!(skipped, Ok(None));
    assert!(failed.is_err());
}

#[test]
fn a_url_runs_the_database_tests() {
    let url = "redis://localhost:6379".to_owned();

    let not_required = choose_database_url(REDIS_URL_VARIABLE, Some(url.clone()), None);
    let required = choose_database_url(REDIS_URL_VARIABLE, Some(url.clone()), Some("1".to_owned()));

    assert_eq!(not_required, Ok(Some(url.clone())));
    assert_eq!(required, Ok(Some(url)));
}

#[test]
fn only_the_value_1_requires_a_database() {
    for value in ["0", ""] {
        let decision = choose_database_url(REDIS_URL_VARIABLE, None, Some(value.to_owned()));

        assert_eq!(decision, Ok(None), "DOYEN_TEST_REQUIRE_DB={value}");
    }
}
