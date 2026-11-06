// SPDX-License-Identifier: MIT OR Apache-2.0
//! The config shared by all databases, its defaults and its checks.

use std::fmt;
use std::time::Duration;

use url::Url;

use crate::error::{Error, ErrorCode};

/// How long a place lives without a renew, when the config does not say.
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
/// How often the place is renewed, when the config does not say.
const DEFAULT_RENEW_INTERVAL_MS: u64 = 2_000;

#[cfg(test)]
mod tests;

/// The config of an instance: one variant per database.
///
/// ```
/// use doyen::{DatabaseConfig, RedisConfig};
///
/// let config = DatabaseConfig::Redis(RedisConfig::new("redis://localhost", "billing-jobs"));
/// assert!(matches!(config, DatabaseConfig::Redis(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DatabaseConfig {
    /// Redis, `redis://` and `rediss://` URLs.
    Redis(RedisConfig),
    /// MongoDB, `mongodb://` and `mongodb+srv://` URLs.
    Mongodb(MongodbConfig),
}

/// The settings of the Redis type.
///
/// Only `url` and `group` are required; the other settings have defaults.
///
/// ```
/// use doyen::RedisConfig;
///
/// let mut config = RedisConfig::new("rediss://redis.example.com:6380", "billing-jobs");
/// config.instance_name = Some("billing-worker-1".to_owned());
/// assert_eq!(config.timeout_ms, None);
/// ```
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedisConfig {
    /// The database URL.
    pub url: String,
    /// The group: instances with the same group share one master.
    pub group: String,
    /// How long a place lives without a renew. Default 10000.
    pub timeout_ms: Option<u64>,
    /// How often the place is renewed. Default 2000.
    pub renew_interval_ms: Option<u64>,
    /// A label for logs and database records. Default: the host name.
    pub instance_name: Option<String>,
}

/// The settings of the MongoDB type.
///
/// Only `url` and `group` are required; the other settings have defaults.
///
/// ```
/// use doyen::MongodbConfig;
///
/// let mut config = MongodbConfig::new("mongodb://db.example.com/doyen", "billing-jobs");
/// config.timeout_ms = Some(15_000);
/// assert_eq!(config.renew_interval_ms, None);
/// ```
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MongodbConfig {
    /// The database URL.
    pub url: String,
    /// The group: instances with the same group share one master.
    pub group: String,
    /// How long a place lives without a renew. Default 10000.
    pub timeout_ms: Option<u64>,
    /// How often the place is renewed. Default 2000.
    pub renew_interval_ms: Option<u64>,
    /// A label for logs and database records. Default: the host name.
    pub instance_name: Option<String>,
}

impl RedisConfig {
    /// A Redis config with the required settings and the defaults for the rest.
    pub fn new(url: impl Into<String>, group: impl Into<String>) -> RedisConfig {
        RedisConfig {
            url: url.into(),
            group: group.into(),
            timeout_ms: None,
            renew_interval_ms: None,
            instance_name: None,
        }
    }
}

impl MongodbConfig {
    /// A MongoDB config with the required settings and the defaults for the rest.
    pub fn new(url: impl Into<String>, group: impl Into<String>) -> MongodbConfig {
        MongodbConfig {
            url: url.into(),
            group: group.into(),
            timeout_ms: None,
            renew_interval_ms: None,
            instance_name: None,
        }
    }
}

/// The settings every database shares, borrowed from a config struct.
struct SharedSettings<'config> {
    database: DatabaseKind,
    url: &'config str,
    group: &'config str,
    timeout_ms: Option<u64>,
    renew_interval_ms: Option<u64>,
    instance_name: Option<&'config str>,
}

impl DatabaseConfig {
    /// Check the config and fill the defaults. The engine calls it at `join`.
    ///
    /// The shared checks come first, so a wrong setting is reported the same way with any
    /// cargo features. The feature check comes last.
    // The engine (change task group 6) is the caller; until then only the tests call it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn validate(&self) -> Result<Validated, Error> {
        let settings = self.shared_settings();
        check_group(settings.group)?;
        let (timeout, renew_interval) =
            check_timing(settings.timeout_ms, settings.renew_interval_ms)?;
        check_url(settings.database, settings.url)?;
        check_feature(settings.database)?;
        Ok(Validated {
            database: settings.database,
            url: settings.url.to_owned(),
            group: settings.group.to_owned(),
            timeout,
            renew_interval,
            instance_name: instance_name_or_host(settings.instance_name),
        })
    }

    fn shared_settings(&self) -> SharedSettings<'_> {
        match self {
            DatabaseConfig::Redis(config) => config.shared_settings(),
            DatabaseConfig::Mongodb(config) => config.shared_settings(),
        }
    }
}

impl RedisConfig {
    fn shared_settings(&self) -> SharedSettings<'_> {
        SharedSettings {
            database: DatabaseKind::Redis,
            url: &self.url,
            group: &self.group,
            timeout_ms: self.timeout_ms,
            renew_interval_ms: self.renew_interval_ms,
            instance_name: self.instance_name.as_deref(),
        }
    }
}

impl MongodbConfig {
    fn shared_settings(&self) -> SharedSettings<'_> {
        SharedSettings {
            database: DatabaseKind::Mongodb,
            url: &self.url,
            group: &self.group,
            timeout_ms: self.timeout_ms,
            renew_interval_ms: self.renew_interval_ms,
            instance_name: self.instance_name.as_deref(),
        }
    }
}

/// Which database a config is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatabaseKind {
    Redis,
    Mongodb,
}

impl DatabaseKind {
    /// The name of the type and of its cargo feature.
    fn name(self) -> &'static str {
        match self {
            DatabaseKind::Redis => "redis",
            DatabaseKind::Mongodb => "mongodb",
        }
    }

    /// The URL schemes the adapter accepts.
    fn schemes(self) -> &'static [&'static str] {
        match self {
            DatabaseKind::Redis => &["redis", "rediss"],
            DatabaseKind::Mongodb => &["mongodb", "mongodb+srv"],
        }
    }

    fn is_built_in(self) -> bool {
        match self {
            DatabaseKind::Redis => cfg!(feature = "redis"),
            DatabaseKind::Mongodb => cfg!(feature = "mongodb"),
        }
    }
}

/// A config that passed every check, with the defaults filled in.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Validated {
    pub(crate) database: DatabaseKind,
    /// The real URL, with the password: the adapter needs it. `Debug` hides the password.
    pub(crate) url: String,
    pub(crate) group: String,
    pub(crate) timeout: Duration,
    pub(crate) renew_interval: Duration,
    pub(crate) instance_name: String,
}

fn invalid_config(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::InvalidConfig, message)
}

fn check_group(group: &str) -> Result<(), Error> {
    if group.is_empty() {
        return Err(invalid_config("group must not be empty"));
    }
    if group.contains(['{', '}']) {
        return Err(invalid_config("group must not contain { or }"));
    }
    Ok(())
}

/// Check the two times and give them with their defaults.
fn check_timing(
    timeout_ms: Option<u64>,
    renew_interval_ms: Option<u64>,
) -> Result<(Duration, Duration), Error> {
    let timeout_ms = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let renew_interval_ms = renew_interval_ms.unwrap_or(DEFAULT_RENEW_INTERVAL_MS);
    if timeout_ms == 0 {
        return Err(invalid_config("timeout_ms must be a positive number"));
    }
    if renew_interval_ms == 0 {
        return Err(invalid_config(
            "renew_interval_ms must be a positive number",
        ));
    }
    // A place must survive two lost renews, so the timeout is at least three intervals.
    if timeout_ms < renew_interval_ms.saturating_mul(3) {
        return Err(invalid_config(format!(
            "the timeout must be at least 3 times the renew interval \
             (timeout_ms is {timeout_ms}, renew_interval_ms is {renew_interval_ms})"
        )));
    }
    Ok((
        Duration::from_millis(timeout_ms),
        Duration::from_millis(renew_interval_ms),
    ))
}

fn check_url(database: DatabaseKind, url: &str) -> Result<(), Error> {
    let Ok(parsed) = Url::parse(url) else {
        // The text is not shown: it may hold a password that the parser did not understand.
        return Err(invalid_config("url is not a valid URL"));
    };
    if !database.schemes().contains(&parsed.scheme()) {
        return Err(invalid_config(format!(
            "url {} has the scheme {}, but the {} type accepts only {}",
            redact_url(url),
            parsed.scheme(),
            database.name(),
            database.schemes().join(" and "),
        )));
    }
    Ok(())
}

fn check_feature(database: DatabaseKind) -> Result<(), Error> {
    if database.is_built_in() {
        return Ok(());
    }
    let name = database.name();
    Err(Error::new(
        ErrorCode::Unsupported,
        format!("the {name} type needs the cargo feature \"{name}\", which is off in this build"),
    ))
}

fn instance_name_or_host(instance_name: Option<&str>) -> String {
    match instance_name {
        Some(name) => name.to_owned(),
        None => gethostname::gethostname().to_string_lossy().into_owned(),
    }
}

/// What a URL shows in place of a secret.
const HIDDEN: &str = "***";

/// The URL with `***` in place of each secret: the password of the user info and the
/// query parameters that hold a password. A text that is not a URL is not shown at all.
pub(crate) fn redact_url(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return "<invalid url>".to_owned();
    };
    if parsed.password().is_some() && parsed.set_password(Some(HIDDEN)).is_err() {
        return "<url hidden>".to_owned();
    }
    if parsed.query().is_some() {
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(name, value)| {
                let value = if holds_secret(&name) {
                    HIDDEN.into()
                } else {
                    value.into_owned()
                };
                (name.into_owned(), value)
            })
            .collect();
        parsed.query_pairs_mut().clear().extend_pairs(pairs);
    }
    parsed.to_string()
}

/// A query parameter holds a secret when its name says password or secret
/// (`password`, `sentinelPassword`, `tlsCertificateKeyFilePassword`).
fn holds_secret(parameter_name: &str) -> bool {
    let name = parameter_name.to_ascii_lowercase();
    name.contains("password") || name.contains("secret")
}

/// Show the URL with `***`; the other settings as they are.
fn debug_config(
    formatter: &mut fmt::Formatter<'_>,
    type_name: &str,
    shared: SharedSettings<'_>,
) -> fmt::Result {
    formatter
        .debug_struct(type_name)
        .field("url", &redact_url(shared.url))
        .field("group", &shared.group)
        .field("timeout_ms", &shared.timeout_ms)
        .field("renew_interval_ms", &shared.renew_interval_ms)
        .field("instance_name", &shared.instance_name)
        .finish()
}

impl fmt::Debug for RedisConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_config(formatter, "RedisConfig", self.shared_settings())
    }
}

impl fmt::Debug for MongodbConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_config(formatter, "MongodbConfig", self.shared_settings())
    }
}

impl fmt::Debug for Validated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Validated")
            .field("database", &self.database)
            .field("url", &redact_url(&self.url))
            .field("group", &self.group)
            .field("timeout", &self.timeout)
            .field("renew_interval", &self.renew_interval)
            .field("instance_name", &self.instance_name)
            .finish()
    }
}
