// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests of the config, its checks, the error codes and the secret rule.
//!
//! `join` does not exist yet, so the tests call the check that `join` will call.

use super::*;
use crate::error::ErrorCode;

fn redis(url: &str, group: &str) -> DatabaseConfig {
    DatabaseConfig::Redis(RedisConfig::new(url, group))
}

fn redis_with_timing(timeout_ms: u64, renew_interval_ms: u64) -> DatabaseConfig {
    let mut config = RedisConfig::new("redis://localhost", "billing");
    config.timeout_ms = Some(timeout_ms);
    config.renew_interval_ms = Some(renew_interval_ms);
    DatabaseConfig::Redis(config)
}

fn mongodb(url: &str, group: &str) -> DatabaseConfig {
    DatabaseConfig::Mongodb(MongodbConfig::new(url, group))
}

fn invalid_config_message(config: &DatabaseConfig) -> String {
    let error = config.validate().unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidConfig);
    error.to_string()
}

// spec: client-config / Only the required settings
#[cfg(feature = "redis")]
#[test]
fn only_the_required_settings_use_the_defaults() {
    use std::time::Duration;

    let validated = redis("redis://localhost", "billing").validate().unwrap();

    assert_eq!(validated.timeout, Duration::from_millis(10_000));
    assert_eq!(validated.renew_interval, Duration::from_millis(2_000));
    let host_name = gethostname::gethostname().to_string_lossy().into_owned();
    assert_eq!(validated.instance_name, host_name);
}

// spec: rust-api / Build a config
#[test]
fn a_new_config_leaves_every_optional_setting_empty() {
    let config = RedisConfig::new("redis://localhost", "billing-jobs");

    assert_eq!(config.url, "redis://localhost");
    assert_eq!(config.group, "billing-jobs");
    assert_eq!(config.timeout_ms, None);
    assert_eq!(config.renew_interval_ms, None);
    assert_eq!(config.instance_name, None);
}

// spec: client-config / Braces in the group
#[test]
fn a_group_with_braces_is_invalid() {
    let message = invalid_config_message(&redis("redis://localhost", "billing{jobs}"));

    assert!(message.contains("group"), "{message}");
}

#[test]
fn an_empty_group_is_invalid() {
    let message = invalid_config_message(&redis("redis://localhost", ""));

    assert!(message.contains("group"), "{message}");
}

// spec: client-config / Timeout too short
#[test]
fn a_timeout_below_three_intervals_is_invalid() {
    let message = invalid_config_message(&redis_with_timing(5_000, 2_000));

    assert!(message.contains("at least 3 times"), "{message}");
}

#[test]
fn a_timeout_of_exactly_three_intervals_is_valid() {
    let config = redis_with_timing(6_000, 2_000);

    #[cfg(feature = "redis")]
    assert!(config.validate().is_ok());
    #[cfg(not(feature = "redis"))]
    assert_ne!(
        config.validate().unwrap_err().code(),
        ErrorCode::InvalidConfig
    );
}

// spec: client-config / Zero interval
#[test]
fn a_zero_interval_is_invalid() {
    let message = invalid_config_message(&redis_with_timing(10_000, 0));

    assert!(message.contains("renew_interval_ms"), "{message}");
}

#[test]
fn a_zero_timeout_is_invalid() {
    let message = invalid_config_message(&redis_with_timing(0, 2_000));

    assert!(message.contains("timeout_ms"), "{message}");
}

#[test]
fn a_huge_interval_does_not_overflow() {
    let message = invalid_config_message(&redis_with_timing(10_000, u64::MAX));

    assert!(message.contains("at least 3 times"), "{message}");
}

// spec: client-config / Wrong scheme
#[test]
fn a_mongodb_config_with_a_redis_url_is_invalid() {
    let message = invalid_config_message(&mongodb("redis://localhost", "billing"));

    assert!(message.contains("scheme"), "{message}");
}

#[test]
fn a_url_that_is_not_a_url_is_invalid() {
    let message = invalid_config_message(&redis("not a url", "billing"));

    assert!(message.contains("url"), "{message}");
}

#[test]
fn every_scheme_of_a_database_is_accepted() {
    let urls = [
        redis("redis://localhost", "billing"),
        redis("rediss://localhost", "billing"),
        mongodb("mongodb://localhost", "billing"),
        mongodb("mongodb+srv://cluster.example.com", "billing"),
    ];

    for config in urls {
        match config.validate() {
            Ok(_) => {}
            Err(error) => assert_eq!(error.code(), ErrorCode::Unsupported, "{error}"),
        }
    }
}

// spec: rust-api / Match a code
#[test]
fn a_bad_config_gives_an_error_that_starts_with_its_code() {
    let error = redis("redis://localhost", "").validate().unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidConfig);
    assert!(
        error.to_string().starts_with("DOYEN_INVALID_CONFIG"),
        "{error}"
    );
}

#[test]
fn the_five_codes_display_their_names() {
    assert_eq!(ErrorCode::InvalidConfig.to_string(), "DOYEN_INVALID_CONFIG");
    assert_eq!(ErrorCode::Auth.to_string(), "DOYEN_AUTH");
    assert_eq!(ErrorCode::Unsupported.to_string(), "DOYEN_UNSUPPORTED");
    assert_eq!(ErrorCode::AlreadyJoined.to_string(), "DOYEN_ALREADY_JOINED");
    assert_eq!(ErrorCode::Left.to_string(), "DOYEN_LEFT");
}

// spec: rust-api / Feature off
#[cfg(not(feature = "mongodb"))]
#[test]
fn a_config_of_a_feature_that_is_off_is_unsupported() {
    let error = mongodb("mongodb://localhost", "billing")
        .validate()
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::Unsupported);
    assert!(error.to_string().contains("mongodb"), "{error}");
}

// spec: client-config / Password in an error
#[test]
fn the_password_of_a_url_is_hidden_in_an_error() {
    let error = mongodb("redis://app:s3cret@db:6379", "billing")
        .validate()
        .unwrap_err();
    let message = error.to_string();

    assert!(!message.contains("s3cret"), "{message}");
    assert!(message.contains("redis://app:***@db:6379"), "{message}");
}

#[test]
fn the_password_is_hidden_when_another_check_fails() {
    let error = redis("redis://app:s3cret@db:6379", "")
        .validate()
        .unwrap_err();

    assert!(!format!("{error} {error:?}").contains("s3cret"));
}

// spec: client-config / Debug of a config
#[test]
fn debug_of_a_config_hides_the_password() {
    let config = mongodb("mongodb://app:s3cret@db/doyen", "billing");

    let output = format!("{config:?}");

    assert!(output.contains("app:***@db"), "{output}");
    assert!(!output.contains("s3cret"), "{output}");
}

#[test]
fn debug_of_every_config_type_hides_the_password() {
    let redis_config = RedisConfig::new("redis://:s3cret@db", "billing");
    let mongodb_config = MongodbConfig::new("mongodb://app:s3cret@db", "billing");

    assert!(!format!("{redis_config:?}").contains("s3cret"));
    assert!(!format!("{mongodb_config:?}").contains("s3cret"));
    assert!(!format!("{:#?}", DatabaseConfig::Redis(redis_config)).contains("s3cret"));
}

#[test]
fn debug_of_a_checked_config_hides_the_password() {
    let config = redis("redis://app:s3cret@db", "billing");

    if let Ok(validated) = config.validate() {
        assert!(!format!("{validated:?}").contains("s3cret"));
    }
}

#[test]
fn query_parameters_that_hold_a_password_are_hidden() {
    let url = "redis://db:6379/?sentinelPassword=s3cret&tls=true&PASSWORD=other";

    let shown = redact_url(url);

    assert!(!shown.contains("s3cret"), "{shown}");
    assert!(!shown.contains("other"), "{shown}");
    assert!(shown.contains("tls=true"), "{shown}");
}

#[test]
fn a_url_without_secrets_is_shown_as_it_is() {
    assert_eq!(redact_url("redis://localhost"), "redis://localhost");
    assert_eq!(redact_url("redis://app@db:6379/0"), "redis://app@db:6379/0");
}

#[test]
fn a_url_that_does_not_parse_is_not_shown_at_all() {
    let shown = redact_url("not a url, password s3cret");

    assert!(!shown.contains("s3cret"), "{shown}");
}
