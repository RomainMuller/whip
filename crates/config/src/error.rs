//! Error types for configuration operations.
//!
//! This module defines the error types that can occur during configuration
//! loading, parsing, and validation.

use std::path::PathBuf;

/// Errors that can occur during configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read a configuration file.
    #[error("failed to read config file at {path}: {source}")]
    ReadFile {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write a configuration file.
    #[error("failed to write config file at {path}: {source}")]
    WriteFile {
        /// The path that could not be written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse JSON5 configuration.
    #[error("failed to parse config: {0}")]
    ParseJson5(#[from] serde_json5::Error),

    /// Failed to serialize configuration to JSON.
    #[error("failed to serialize config: {0}")]
    SerializeJson(#[from] serde_json::Error),

    /// Invalid repository format.
    #[error("invalid repository format: {0}")]
    InvalidRepository(String),

    /// Invalid polling interval.
    #[error("invalid polling interval: {reason}")]
    InvalidPollingInterval {
        /// The reason the interval is invalid.
        reason: String,
    },

    /// Failed to determine home directory.
    #[error("could not determine home directory")]
    NoHomeDirectory,

    /// Failed to execute `gh auth token` command.
    #[error("failed to get GitHub token from gh CLI: {0}")]
    GhAuthFailed(#[source] std::io::Error),

    /// The `gh auth token` command returned an error.
    #[error("gh auth token failed with exit code {code:?}: {stderr}")]
    GhAuthError {
        /// The exit code, if available.
        code: Option<i32>,
        /// The stderr output.
        stderr: String,
    },
}

/// A specialized Result type for configuration operations.
pub type Result<T> = std::result::Result<T, ConfigError>;
