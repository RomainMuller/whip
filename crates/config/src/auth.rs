//! GitHub token resolution and authentication.
//!
//! This module provides token resolution with fallback chain:
//!
//! 1. Repository-specific token (if configured)
//! 2. Global `github_token` from config
//! 3. `gh auth token` command (GitHub CLI)
//! 4. Unauthenticated (returns `None`)

use crate::Repository;
use crate::error::{ConfigError, Result};

/// Resolves the GitHub token for a specific repository.
///
/// Tries the following sources in order:
///
/// 1. Repository-specific token (from `Repository::token()`)
/// 2. Global token (from `global_token` parameter)
/// 3. `gh auth token` command
///
/// # Arguments
///
/// * `repo` - The repository configuration (may have a specific token)
/// * `global_token` - The global GitHub token from config, if any
///
/// # Returns
///
/// Returns `Some(token)` if a token is available, `None` otherwise.
///
/// # Errors
///
/// This function does not return errors for missing tokens; it simply
/// returns `None`. Errors are only returned if the `gh` CLI command
/// exists but fails unexpectedly.
///
/// # Examples
///
/// ```no_run
/// use whip_config::{Repository, auth::resolve_token};
///
/// # async fn example() {
/// let repo = Repository::new("rust-lang", "rust");
/// let global_token = Some("ghp_global".to_string());
///
/// let token = resolve_token(&repo, global_token.as_deref()).await;
/// # }
/// ```
pub async fn resolve_token(repo: &Repository, global_token: Option<&str>) -> Option<String> {
    // 1. Repository-specific token
    if let Some(token) = repo.token() {
        return Some(token.to_string());
    }

    // 2. Global token
    if let Some(token) = global_token {
        return Some(token.to_string());
    }

    // 3. Try gh CLI
    get_gh_token().await.ok().flatten()
}

/// Gets a GitHub token from the `gh` CLI.
///
/// Runs `gh auth token` and returns the token if successful.
///
/// # Returns
///
/// - `Ok(Some(token))` if the command succeeds and returns a token
/// - `Ok(None)` if the `gh` command is not found
/// - `Err(...)` if the command exists but fails
///
/// # Errors
///
/// Returns an error if:
/// - The `gh` command exists but returns an error
/// - The command output cannot be parsed
///
/// # Examples
///
/// ```no_run
/// use whip_config::auth::get_gh_token;
///
/// # async fn example() -> whip_config::Result<()> {
/// match get_gh_token().await? {
///     Some(token) => println!("Got token from gh CLI"),
///     None => println!("gh CLI not available"),
/// }
/// # Ok(())
/// # }
/// ```
pub async fn get_gh_token() -> Result<Option<String>> {
    use tokio::process::Command;

    let output = match Command::new("gh").args(["auth", "token"]).output().await {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // gh not installed, not an error
            return Ok(None);
        }
        Err(e) => {
            return Err(ConfigError::GhAuthFailed(e));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // If gh is not logged in, treat as no token available
        if stderr.contains("not logged in") || stderr.contains("no oauth token") {
            return Ok(None);
        }
        return Err(ConfigError::GhAuthError {
            code: output.status.code(),
            stderr,
        });
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Ok(None);
    }

    Ok(Some(token))
}

/// Checks if a token is available through any source.
///
/// This is a convenience function that checks all token sources
/// without actually retrieving the token value.
///
/// # Arguments
///
/// * `repo` - The repository configuration
/// * `global_token` - The global GitHub token from config, if any
///
/// # Examples
///
/// ```no_run
/// use whip_config::{Repository, auth::has_token};
///
/// # async fn example() {
/// let repo = Repository::new("rust-lang", "rust");
/// if has_token(&repo, None).await {
///     println!("Authenticated access available");
/// }
/// # }
/// ```
pub async fn has_token(repo: &Repository, global_token: Option<&str>) -> bool {
    resolve_token(repo, global_token).await.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_token_prefers_repo_token() {
        let repo = Repository::with_token("owner", "repo", "repo_token");
        let token = resolve_token(&repo, Some("global_token")).await;
        assert_eq!(token, Some("repo_token".to_string()));
    }

    #[tokio::test]
    async fn resolve_token_falls_back_to_global() {
        let repo = Repository::new("owner", "repo");
        let token = resolve_token(&repo, Some("global_token")).await;
        assert_eq!(token, Some("global_token".to_string()));
    }

    // Note: Testing gh CLI integration requires the tool to be installed,
    // so we don't include those tests here. Integration tests should cover that.

    #[tokio::test]
    async fn has_token_true_with_repo_token() {
        let repo = Repository::with_token("owner", "repo", "repo_token");
        assert!(has_token(&repo, None).await);
    }

    #[tokio::test]
    async fn has_token_true_with_global_token() {
        let repo = Repository::new("owner", "repo");
        assert!(has_token(&repo, Some("global_token")).await);
    }

    #[tokio::test]
    async fn resolve_token_gh_cli_fallback_does_not_panic() {
        // Without repo token or global token, resolve_token will try gh CLI.
        // This test verifies the fallback path doesn't panic regardless of
        // whether gh is installed or logged in on this machine.
        let repo = Repository::new("owner", "repo");
        let _result = resolve_token(&repo, None).await;
    }
}
