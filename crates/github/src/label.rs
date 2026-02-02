//! GitHub label management API.
//!
//! This module provides functionality for managing GitHub labels on repositories,
//! including listing, creating, updating, and syncing whip status labels.
//!
//! # Overview
//!
//! The module extends [`GitHubClient`] with label management capabilities:
//!
//! - [`GitHubClient::list_labels`]: List all labels on a repository
//! - [`GitHubClient::create_label`]: Create a new label
//! - [`GitHubClient::update_label`]: Update an existing label
//! - [`sync_labels`]: Sync whip status labels to a repository
//!
//! # Example
//!
//! ```no_run
//! use whip_github::GitHubClient;
//! use whip_protocol::standard_status_labels;
//!
//! # async fn example() -> whip_github::Result<()> {
//! let client = GitHubClient::new(None).await?;
//!
//! // List existing labels
//! let labels = client.list_labels("owner", "repo").await?;
//! println!("Found {} labels", labels.len());
//!
//! // Sync whip labels (requires write access)
//! // sync_labels(&client, "owner", "repo").await?;
//! # Ok(())
//! # }
//! ```

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};
use whip_protocol::{LabelDefinition, standard_status_labels};

use crate::client::GitHubClient;
use crate::error::{Error, Result};

/// A GitHub label as returned by the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubLabel {
    /// The label name.
    pub name: String,
    /// The hex color code (without `#`).
    pub color: String,
    /// The label description.
    #[serde(default)]
    pub description: Option<String>,
}

impl GitHubLabel {
    /// Returns `true` if this label is a whip status label.
    #[must_use]
    pub fn is_whip_label(&self) -> bool {
        self.name.starts_with(whip_protocol::LABEL_PREFIX)
    }
}

/// Request body for creating a label.
#[derive(Debug, Serialize)]
struct CreateLabelRequest<'a> {
    name: &'a str,
    color: &'a str,
    description: &'a str,
}

/// Request body for updating a label.
#[derive(Debug, Serialize)]
struct UpdateLabelRequest<'a> {
    new_name: &'a str,
    color: &'a str,
    description: &'a str,
}

impl GitHubClient {
    /// Lists all labels in a repository.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails (e.g., repository not found,
    /// rate limit exceeded).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::GitHubClient;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let client = GitHubClient::new(None).await?;
    /// let labels = client.list_labels("rust-lang", "rust").await?;
    ///
    /// for label in &labels {
    ///     println!("{}: #{}", label.name, label.color);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(owner = %owner, repo = %repo))]
    pub async fn list_labels(&self, owner: &str, repo: &str) -> Result<Vec<GitHubLabel>> {
        debug!("listing labels");

        let url = format!("/repos/{owner}/{repo}/labels?per_page=100");
        let response: Vec<GitHubLabel> = self
            .inner()
            .get(&url, None::<&()>)
            .await
            .map_err(Error::Api)?;

        debug!(count = response.len(), "listed labels");
        Ok(response)
    }

    /// Creates a new label in a repository.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `label` - The label definition to create
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not authenticated
    /// - The label already exists
    /// - The API call fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::GitHubClient;
    /// use whip_protocol::{LabelDefinition, LaneKind};
    /// use secrecy::SecretString;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let token = SecretString::from("ghp_xxx".to_string());
    /// let client = GitHubClient::new(Some(token)).await?;
    ///
    /// let label = LabelDefinition::new(
    ///     "whip/backlog",
    ///     "0052CC",
    ///     "Task is in the backlog",
    ///     LaneKind::Backlog,
    /// );
    ///
    /// client.create_label("owner", "repo", &label).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, label), fields(owner = %owner, repo = %repo, label = %label.name))]
    pub async fn create_label(
        &self,
        owner: &str,
        repo: &str,
        label: &LabelDefinition,
    ) -> Result<GitHubLabel> {
        debug!("creating label");

        let url = format!("/repos/{owner}/{repo}/labels");
        let body = CreateLabelRequest {
            name: &label.name,
            color: &label.color,
            description: &label.description,
        };

        let response: GitHubLabel = self
            .inner()
            .post(&url, Some(&body))
            .await
            .map_err(Error::Api)?;

        debug!("created label");
        Ok(response)
    }

    /// Updates an existing label in a repository.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `current_name` - The current name of the label to update
    /// * `label` - The new label definition
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The client is not authenticated
    /// - The label does not exist
    /// - The API call fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::GitHubClient;
    /// use whip_protocol::{LabelDefinition, LaneKind};
    /// use secrecy::SecretString;
    ///
    /// # async fn example() -> whip_github::Result<()> {
    /// let token = SecretString::from("ghp_xxx".to_string());
    /// let client = GitHubClient::new(Some(token)).await?;
    ///
    /// let label = LabelDefinition::new(
    ///     "whip/backlog",
    ///     "0052CC",
    ///     "Updated description",
    ///     LaneKind::Backlog,
    /// );
    ///
    /// client.update_label("owner", "repo", "whip/backlog", &label).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, label), fields(owner = %owner, repo = %repo, current_name = %current_name, new_name = %label.name))]
    pub async fn update_label(
        &self,
        owner: &str,
        repo: &str,
        current_name: &str,
        label: &LabelDefinition,
    ) -> Result<GitHubLabel> {
        debug!("updating label");

        // URL-encode the label name since it may contain special characters like '/'
        let encoded_name = utf8_percent_encode(current_name, NON_ALPHANUMERIC).to_string();
        let url = format!("/repos/{owner}/{repo}/labels/{encoded_name}");

        let body = UpdateLabelRequest {
            new_name: &label.name,
            color: &label.color,
            description: &label.description,
        };

        let response: GitHubLabel = self
            .inner()
            .patch(&url, Some(&body))
            .await
            .map_err(Error::Api)?;

        debug!("updated label");
        Ok(response)
    }
}

/// Synchronizes whip status labels to a repository.
///
/// This function ensures that all standard whip labels exist on the repository
/// with the correct colors and descriptions. It will:
///
/// 1. Create labels that don't exist
/// 2. Update labels that exist but have incorrect colors or descriptions
/// 3. Leave correctly configured labels unchanged
///
/// # Arguments
///
/// * `client` - An authenticated GitHub client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// Returns a [`SyncResult`] containing counts of created and updated labels.
///
/// # Errors
///
/// Returns an error if any API call fails. The sync is not atomic - some
/// labels may be created/updated before the error occurs.
///
/// # Examples
///
/// ```no_run
/// use whip_github::{GitHubClient, sync_labels};
/// use secrecy::SecretString;
///
/// # async fn example() -> whip_github::Result<()> {
/// let token = SecretString::from("ghp_xxx".to_string());
/// let client = GitHubClient::new(Some(token)).await?;
///
/// let result = sync_labels(&client, "owner", "repo").await?;
/// println!("Created: {}, Updated: {}", result.created, result.updated);
/// # Ok(())
/// # }
/// ```
#[instrument(skip(client), fields(owner = %owner, repo = %repo))]
pub async fn sync_labels(client: &GitHubClient, owner: &str, repo: &str) -> Result<SyncResult> {
    debug!("syncing whip labels");

    // Get existing labels
    let existing = client.list_labels(owner, repo).await?;
    let existing_by_name: std::collections::HashMap<&str, &GitHubLabel> =
        existing.iter().map(|l| (l.name.as_str(), l)).collect();

    let mut created = 0;
    let mut updated = 0;

    // Sync each standard label
    for label_def in standard_status_labels() {
        match existing_by_name.get(label_def.name.as_str()) {
            Some(existing_label) => {
                // Check if update needed
                let needs_update = existing_label.color != label_def.color
                    || existing_label.description.as_deref() != Some(&label_def.description);

                if needs_update {
                    debug!(label = %label_def.name, "updating label");
                    match client
                        .update_label(owner, repo, &label_def.name, &label_def)
                        .await
                    {
                        Ok(_) => updated += 1,
                        Err(e) => {
                            warn!(label = %label_def.name, error = %e, "failed to update label");
                            return Err(e);
                        }
                    }
                } else {
                    debug!(label = %label_def.name, "label already up to date");
                }
            }
            None => {
                // Create the label
                debug!(label = %label_def.name, "creating label");
                match client.create_label(owner, repo, &label_def).await {
                    Ok(_) => created += 1,
                    Err(e) => {
                        warn!(label = %label_def.name, error = %e, "failed to create label");
                        return Err(e);
                    }
                }
            }
        }
    }

    debug!(created = created, updated = updated, "sync complete");
    Ok(SyncResult { created, updated })
}

/// Result of a label sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncResult {
    /// Number of labels created.
    pub created: usize,
    /// Number of labels updated.
    pub updated: usize,
}

impl SyncResult {
    /// Returns `true` if no changes were made.
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        self.created == 0 && self.updated == 0
    }

    /// Returns the total number of changes made.
    #[must_use]
    pub fn total_changes(&self) -> usize {
        self.created + self.updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_label_is_whip_label() {
        let whip = GitHubLabel {
            name: "whip/backlog".to_string(),
            color: "0052CC".to_string(),
            description: Some("Backlog".to_string()),
        };
        assert!(whip.is_whip_label());

        let other = GitHubLabel {
            name: "bug".to_string(),
            color: "FF0000".to_string(),
            description: None,
        };
        assert!(!other.is_whip_label());
    }

    #[test]
    fn sync_result_is_unchanged() {
        let unchanged = SyncResult {
            created: 0,
            updated: 0,
        };
        assert!(unchanged.is_unchanged());
        assert_eq!(unchanged.total_changes(), 0);

        let changed = SyncResult {
            created: 1,
            updated: 2,
        };
        assert!(!changed.is_unchanged());
        assert_eq!(changed.total_changes(), 3);
    }

    #[test]
    fn github_label_serialization_roundtrip() {
        let label = GitHubLabel {
            name: "whip/in-progress".to_string(),
            color: "FBCA04".to_string(),
            description: Some("In progress".to_string()),
        };

        let json = serde_json::to_string(&label).expect("serialize");
        let parsed: GitHubLabel = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(label, parsed);
    }

    #[test]
    fn github_label_deserialize_without_description() {
        let json = r#"{"name": "bug", "color": "FF0000"}"#;
        let label: GitHubLabel = serde_json::from_str(json).expect("deserialize");

        assert_eq!(label.name, "bug");
        assert_eq!(label.color, "FF0000");
        assert!(label.description.is_none());
    }
}
