//! GitHub API client implementation.
//!
//! This module provides the [`GitHubClient`] struct for interacting with
//! the GitHub API, supporting both authenticated and unauthenticated access.

use octocrab::Octocrab;
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, instrument, warn};

use crate::error::{Error, Result};
use crate::issue::FetchOptions;

/// GitHub API client with optional authentication.
///
/// The client supports both authenticated and unauthenticated access.
/// Authenticated clients have higher rate limits (5,000 req/hour vs 60 req/hour)
/// and can access private repositories.
///
/// # Security
///
/// Tokens are stored using [`SecretString`] to prevent accidental logging
/// or exposure in debug output.
///
/// # Examples
///
/// ```no_run
/// use secrecy::SecretString;
/// use whip_github::GitHubClient;
///
/// # async fn example() -> whip_github::Result<()> {
/// // Create an authenticated client
/// let token = SecretString::from("ghp_your_token".to_string());
/// let client = GitHubClient::new(Some(token)).await?;
///
/// // Validate the token works
/// let is_valid = client.validate_token().await?;
/// println!("Token valid: {}", is_valid);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct GitHubClient {
    /// The underlying octocrab client.
    inner: Octocrab,
    /// Whether this client is authenticated.
    authenticated: bool,
}

impl GitHubClient {
    /// Creates a new GitHub client.
    ///
    /// # Arguments
    ///
    /// * `token` - Optional GitHub personal access token. If `Some`, the client
    ///   will be authenticated with higher rate limits. If `None`, the client
    ///   will be unauthenticated with lower rate limits.
    ///
    /// # Errors
    ///
    /// Returns an error if the octocrab client fails to initialize.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use secrecy::SecretString;
    /// use whip_github::GitHubClient;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// // Authenticated client
    /// let token = SecretString::from("ghp_xxx".to_string());
    /// let auth_client = GitHubClient::new(Some(token)).await?;
    ///
    /// // Unauthenticated client
    /// let unauth_client = GitHubClient::new(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(token), fields(authenticated = token.is_some()))]
    pub async fn new(token: Option<SecretString>) -> Result<Self> {
        let (inner, authenticated) = match token {
            Some(token) => {
                debug!("creating authenticated GitHub client");
                let client = Octocrab::builder()
                    .personal_token(token.expose_secret())
                    .build()
                    .map_err(Error::Api)?;
                (client, true)
            }
            None => {
                debug!("creating unauthenticated GitHub client");
                let client = Octocrab::builder().build().map_err(Error::Api)?;
                (client, false)
            }
        };

        Ok(Self {
            inner,
            authenticated,
        })
    }

    /// Validates the current token by making a test API call.
    ///
    /// This method calls the `/user` endpoint to verify that the token
    /// is valid and has not expired.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if authenticated and token is valid
    /// - `Ok(false)` if not authenticated (no token provided)
    /// - `Err` if the API call fails (e.g., invalid token, network error)
    ///
    /// # Errors
    ///
    /// Returns [`Error::TokenValidation`] if the token is invalid or expired.
    /// Returns [`Error::Api`] for other API errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use secrecy::SecretString;
    /// use whip_github::GitHubClient;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let token = SecretString::from("ghp_xxx".to_string());
    /// let client = GitHubClient::new(Some(token)).await?;
    ///
    /// match client.validate_token().await {
    ///     Ok(true) => println!("Token is valid"),
    ///     Ok(false) => println!("No token provided"),
    ///     Err(e) => println!("Validation failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn validate_token(&self) -> Result<bool> {
        if !self.authenticated {
            debug!("client is not authenticated, skipping validation");
            return Ok(false);
        }

        debug!("validating token by calling /user endpoint");
        match self.inner.current().user().await {
            Ok(user) => {
                debug!(login = %user.login, "token validated successfully");
                Ok(true)
            }
            Err(octocrab::Error::GitHub { source, .. }) => {
                warn!(message = %source.message, "token validation failed");
                Err(Error::TokenValidation {
                    reason: source.message,
                })
            }
            Err(e) => {
                warn!(error = %e, "API error during token validation");
                Err(Error::Api(e))
            }
        }
    }

    /// Returns whether this client is authenticated.
    ///
    /// Authenticated clients have access to:
    /// - Higher rate limits (5,000 requests/hour vs 60 requests/hour)
    /// - Private repositories (if token has appropriate scopes)
    ///
    /// Note: This returns the authentication state at client creation time.
    /// It does not verify the token is still valid. Use [`validate_token`](Self::validate_token)
    /// to check if the token is currently valid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use secrecy::SecretString;
    /// use whip_github::GitHubClient;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let token = SecretString::from("ghp_xxx".to_string());
    /// let client = GitHubClient::new(Some(token)).await?;
    /// assert!(client.is_authenticated());
    ///
    /// let unauth = GitHubClient::new(None).await?;
    /// assert!(!unauth.is_authenticated());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Returns a reference to the underlying octocrab client.
    ///
    /// This allows direct access to the full octocrab API for operations
    /// not yet wrapped by this client.
    #[must_use]
    pub fn inner(&self) -> &Octocrab {
        &self.inner
    }

    /// Fetches issues from a GitHub repository.
    ///
    /// Retrieves issues matching the given filter options. By default, fetches
    /// open issues with no label filter.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner (e.g., "rust-lang")
    /// * `repo` - Repository name (e.g., "rust")
    /// * `options` - Filtering and pagination options
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails, such as:
    /// - Repository not found (404)
    /// - Rate limit exceeded
    /// - Network errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::{GitHubClient, FetchOptions, IssueState};
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let client = GitHubClient::new(None).await?;
    ///
    /// // Fetch open issues with "bug" label
    /// let options = FetchOptions {
    ///     state: IssueState::Open,
    ///     labels: vec!["bug".to_string()],
    ///     per_page: 10,
    /// };
    ///
    /// let issues = client.fetch_issues("rust-lang", "rust", &options).await?;
    /// println!("Found {} issues", issues.len());
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, options), fields(owner = %owner, repo = %repo))]
    pub async fn fetch_issues(
        &self,
        owner: &str,
        repo: &str,
        options: &FetchOptions,
    ) -> Result<Vec<octocrab::models::issues::Issue>> {
        debug!(
            state = ?options.state,
            labels = ?options.labels,
            per_page = options.effective_per_page(),
            "fetching issues"
        );

        let issues_handler = self.inner.issues(owner, repo);
        let page = if options.labels.is_empty() {
            issues_handler
                .list()
                .state(options.state.to_octocrab_state())
                .per_page(options.effective_per_page())
                .send()
                .await
                .map_err(Error::Api)?
        } else {
            issues_handler
                .list()
                .state(options.state.to_octocrab_state())
                .per_page(options.effective_per_page())
                .labels(&options.labels)
                .send()
                .await
                .map_err(Error::Api)?
        };

        // Filter out pull requests (GitHub API returns PRs in the issues endpoint)
        let issues: Vec<_> = page
            .items
            .into_iter()
            .filter(|issue| issue.pull_request.is_none())
            .collect();
        debug!(count = issues.len(), "fetched issues (excluding PRs)");

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_unauthenticated_client() {
        let client = GitHubClient::new(None).await.unwrap();
        assert!(!client.is_authenticated());
    }

    #[tokio::test]
    async fn new_authenticated_client() {
        // This test creates a client with a token but doesn't validate it
        // (validation would require a real token)
        let token = SecretString::from("fake_token_for_testing".to_string());
        let client = GitHubClient::new(Some(token)).await.unwrap();
        assert!(client.is_authenticated());
    }

    #[tokio::test]
    async fn validate_token_unauthenticated() {
        let client = GitHubClient::new(None).await.unwrap();
        let result = client.validate_token().await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn inner_returns_octocrab_reference() {
        let client = GitHubClient::new(None).await.unwrap();
        // Verify inner() returns a valid reference to the Octocrab client
        let _octocrab: &Octocrab = client.inner();
    }
}
