//! GitHub API client for whip.
//!
//! This crate provides GitHub Issues integration for the whip task manager,
//! including issue fetching, caching, and token management.
//!
//! # Overview
//!
//! The crate provides:
//!
//! - [`GitHubClient`]: The main API client with optional authentication
//! - [`Error`]: Error types for GitHub API operations
//!
//! # Authentication
//!
//! The client supports both authenticated and unauthenticated access:
//!
//! - **Authenticated**: 5,000 requests/hour, access to private repos
//! - **Unauthenticated**: 60 requests/hour, public repos only
//!
//! Tokens are handled securely using [`secrecy::SecretString`] to prevent
//! accidental logging of sensitive credentials.
//!
//! # Examples
//!
//! Creating an authenticated client:
//!
//! ```no_run
//! use secrecy::SecretString;
//! use whip_github::GitHubClient;
//!
//! # async fn example() -> whip_github::Result<()> {
//! // Create with a token
//! let token = SecretString::from("ghp_your_token_here".to_string());
//! let client = GitHubClient::new(Some(token)).await?;
//!
//! // Verify authentication
//! if client.is_authenticated() {
//!     println!("Authenticated with higher rate limits");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Creating an unauthenticated client:
//!
//! ```no_run
//! use whip_github::GitHubClient;
//!
//! # async fn example() -> whip_github::Result<()> {
//! let client = GitHubClient::new(None).await?;
//! assert!(!client.is_authenticated());
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;

pub use client::GitHubClient;
pub use error::{Error, Result};
