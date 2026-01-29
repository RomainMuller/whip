//! Configuration management for the whip application.
//!
//! This crate handles loading, validating, and persisting configuration
//! from multiple sources (files, environment variables, defaults).
//!
//! # Overview
//!
//! The crate is organized into the following modules:
//!
//! - [`config`]: Core configuration struct and loading logic
//! - [`repository`]: Repository configuration with flexible parsing
//! - [`polling`]: Polling interval configuration with rate-limit awareness
//! - [`auth`]: GitHub token resolution and authentication
//! - [`persistence`]: Config file reading and writing
//! - [`error`]: Error types for configuration operations
//!
//! # Configuration Sources (Priority)
//!
//! Configuration is loaded from multiple sources with the following priority
//! (highest to lowest):
//!
//! 1. Environment variables (`WHIP_*`)
//! 2. Local config (`./whip.json5` or `./whip.json`)
//! 3. User config (`~/.config/whip/config.json5` or `~/.config/whip/config.json`)
//! 4. Built-in defaults
//!
//! # Repository Format
//!
//! Repositories can be specified in two formats:
//!
//! ```json5
//! {
//!   "repositories": [
//!     // Short format: "owner/repo"
//!     "rust-lang/rust",
//!     // Full format with optional token override
//!     { "owner": "private-org", "repo": "secret-repo", "token": "ghp_xxx" }
//!   ]
//! }
//! ```
//!
//! # Token Resolution
//!
//! GitHub tokens are resolved in the following order:
//!
//! 1. Repository-specific token (if configured)
//! 2. Global `github_token` from config
//! 3. `gh auth token` command (GitHub CLI)
//! 4. Unauthenticated (rate-limited)
//!
//! # Examples
//!
//! Loading configuration:
//!
//! ```no_run
//! use whip_config::Config;
//!
//! # async fn example() -> whip_config::Result<()> {
//! // Load from default locations
//! let config = Config::load().await?;
//!
//! // Access repositories
//! for repo in &config.repositories {
//!     println!("Repository: {}", repo.full_name());
//! }
//!
//! // Get polling interval
//! println!("Poll every {} seconds", config.polling.interval_secs);
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod config;
pub mod error;
pub mod persistence;
pub mod polling;
pub mod repository;

// Re-export primary types at crate root for convenience
pub use config::Config;
pub use error::{ConfigError, Result};
pub use polling::PollingConfig;
pub use repository::Repository;
