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
//! - [`FetchOptions`] and [`IssueState`]: Options for filtering issues
//! - [`issue_to_task`]: Convert GitHub issues to whip tasks
//! - [`IssueCache`] and [`CachedIssues`]: Persistent caching for issues
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
//! # Caching
//!
//! The [`IssueCache`] provides persistent storage for GitHub issues:
//!
//! ```no_run
//! use std::time::Duration;
//! use whip_github::{CachedIssues, IssueCache};
//!
//! # fn example() -> whip_github::Result<()> {
//! let cache = IssueCache::new()?;
//!
//! // Check if we need to refresh
//! if cache.is_stale("owner", "repo", Duration::from_secs(300)) {
//!     // Fetch from API and save
//!     let cached = CachedIssues::new(vec![], None);
//!     cache.save("owner", "repo", &cached)?;
//! }
//!
//! // Load cached data
//! if let Some(cached) = cache.load("owner", "repo")? {
//!     println!("Found {} cached tasks", cached.tasks.len());
//! }
//! # Ok(())
//! # }
//! ```
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
//!
//! Fetching issues and converting to tasks:
//!
//! ```no_run
//! use whip_github::{GitHubClient, FetchOptions, IssueState, issue_to_task};
//!
//! # async fn example() -> whip_github::Result<()> {
//! let client = GitHubClient::new(None).await?;
//!
//! let options = FetchOptions {
//!     state: IssueState::Open,
//!     labels: vec!["bug".to_string()],
//!     per_page: 10,
//! };
//!
//! let issues = client.fetch_issues("owner", "repo", &options).await?;
//! let tasks: Vec<_> = issues.iter()
//!     .map(|issue| issue_to_task(issue, "owner", "repo"))
//!     .collect();
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod client;
pub mod error;
pub mod issue;

pub use cache::{CachedIssues, IssueCache};
pub use client::GitHubClient;
pub use error::{Error, Result};
pub use issue::{FetchOptions, IssueState, issue_to_task};
