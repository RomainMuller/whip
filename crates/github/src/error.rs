//! Error types for GitHub API operations.
//!
//! This module defines the error types that can occur during GitHub API
//! interactions, including authentication, rate limiting, and I/O operations.

use std::time::Duration;

/// Errors that can occur during GitHub API operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while calling the GitHub API.
    #[error("GitHub API error: {0}")]
    Api(#[from] octocrab::Error),

    /// Token validation failed.
    ///
    /// This occurs when a provided token is invalid, expired, or lacks
    /// the necessary permissions.
    #[error("token validation failed: {reason}")]
    TokenValidation {
        /// A description of why validation failed.
        reason: String,
    },

    /// Rate limit exceeded.
    ///
    /// GitHub API has rate limits: 60 requests/hour for unauthenticated
    /// requests, 5,000 requests/hour for authenticated requests.
    #[error("rate limit exceeded{}", format_reset_time(*.reset_after))]
    RateLimited {
        /// Time until the rate limit resets, if known.
        reset_after: Option<Duration>,
    },

    /// An I/O error occurred during cache operations.
    #[error("I/O error during cache operation: {0}")]
    Io(#[from] std::io::Error),
}

/// Formats the reset time for the rate limit error message.
fn format_reset_time(reset_after: Option<Duration>) -> String {
    match reset_after {
        Some(duration) => format!(", resets in {} seconds", duration.as_secs()),
        None => String::new(),
    }
}

/// A specialized Result type for GitHub API operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_rate_limited_with_reset() {
        let err = Error::RateLimited {
            reset_after: Some(Duration::from_secs(3600)),
        };
        assert_eq!(
            err.to_string(),
            "rate limit exceeded, resets in 3600 seconds"
        );
    }

    #[test]
    fn error_display_rate_limited_without_reset() {
        let err = Error::RateLimited { reset_after: None };
        assert_eq!(err.to_string(), "rate limit exceeded");
    }

    #[test]
    fn error_display_token_validation() {
        let err = Error::TokenValidation {
            reason: "token expired".to_string(),
        };
        assert_eq!(err.to_string(), "token validation failed: token expired");
    }

    #[test]
    fn error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::Io(io_err);
        assert!(err.to_string().contains("I/O error during cache operation"));
    }
}
