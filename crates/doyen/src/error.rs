// SPDX-License-Identifier: MIT OR Apache-2.0
//! The error type and the error codes.

use std::fmt;

/// The code of an [`Error`]. Every language binding returns the same codes.
///
/// Network errors are never returned as errors: the core retries and the status shows the
/// problem.
///
/// ```
/// use doyen::ErrorCode;
///
/// assert_eq!(ErrorCode::InvalidConfig.to_string(), "DOYEN_INVALID_CONFIG");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// The config is wrong: fix the setting that the message names.
    InvalidConfig,
    /// The database refused a command because of its access rules.
    Auth,
    /// The feature, the platform or the runtime is not supported.
    Unsupported,
    /// `join` was called on an object that already joined.
    AlreadyJoined,
    /// The object called `leave` and cannot join again.
    Left,
}

impl ErrorCode {
    /// The code as text, for example `DOYEN_INVALID_CONFIG`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidConfig => "DOYEN_INVALID_CONFIG",
            ErrorCode::Auth => "DOYEN_AUTH",
            ErrorCode::Unsupported => "DOYEN_UNSUPPORTED",
            ErrorCode::AlreadyJoined => "DOYEN_ALREADY_JOINED",
            ErrorCode::Left => "DOYEN_LEFT",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The one error type of doyen.
///
/// Its `Display` starts with the code, then says what to fix. The message never holds the
/// password of a database URL.
///
/// ```
/// use doyen::{Error, ErrorCode};
///
/// fn should_fix_config(error: &Error) -> bool {
///     error.code() == ErrorCode::InvalidConfig
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    code: ErrorCode,
    message: String,
}

impl Error {
    /// Make an error. Crate only: users get errors, they do not make them.
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Error {
        Error {
            code,
            message: message.into(),
        }
    }

    /// The code of the error, to match on.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// What to fix, without the code.
    pub fn message(&self) -> &str {
        &self.message
    }
}
