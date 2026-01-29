//! Repository configuration with flexible parsing.
//!
//! This module provides the [`Repository`] type which supports two formats:
//!
//! - Short format: `"owner/repo"` string
//! - Full format: `{ "owner": "...", "repo": "...", "token": "..." }` object
//!
//! # Examples
//!
//! ```
//! use whip_config::Repository;
//!
//! // Parse from short format
//! let repo: Repository = serde_json::from_str(r#""rust-lang/rust""#).unwrap();
//! assert_eq!(repo.owner(), "rust-lang");
//! assert_eq!(repo.repo(), "rust");
//!
//! // Parse from full format
//! let repo: Repository = serde_json::from_str(r#"{"owner": "org", "repo": "repo"}"#).unwrap();
//! assert_eq!(repo.full_name(), "org/repo");
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ConfigError, Result};

/// A GitHub repository configuration.
///
/// Supports two serialization formats:
/// - Short: `"owner/repo"` string
/// - Full: `{ "owner": "...", "repo": "...", "token": "..." }` object
///
/// # Examples
///
/// ```
/// use whip_config::Repository;
///
/// let repo = Repository::new("rust-lang", "rust");
/// assert_eq!(repo.full_name(), "rust-lang/rust");
/// assert!(repo.token().is_none());
///
/// let repo_with_token = Repository::with_token("org", "repo", "ghp_xxx");
/// assert!(repo_with_token.token().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    owner: String,
    repo: String,
    token: Option<String>,
}

impl Repository {
    /// Creates a new repository configuration without a token.
    ///
    /// # Arguments
    ///
    /// * `owner` - The repository owner (user or organization)
    /// * `repo` - The repository name
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Repository;
    ///
    /// let repo = Repository::new("rust-lang", "rust");
    /// assert_eq!(repo.owner(), "rust-lang");
    /// assert_eq!(repo.repo(), "rust");
    /// ```
    #[must_use]
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            token: None,
        }
    }

    /// Creates a new repository configuration with a token.
    ///
    /// # Arguments
    ///
    /// * `owner` - The repository owner (user or organization)
    /// * `repo` - The repository name
    /// * `token` - The GitHub token for this repository
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Repository;
    ///
    /// let repo = Repository::with_token("org", "repo", "ghp_xxx");
    /// assert_eq!(repo.token(), Some("ghp_xxx"));
    /// ```
    #[must_use]
    pub fn with_token(
        owner: impl Into<String>,
        repo: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            token: Some(token.into()),
        }
    }

    /// Parses a repository from the short format `"owner/repo"`.
    ///
    /// # Errors
    ///
    /// Returns an error if the string does not contain exactly one `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Repository;
    ///
    /// let repo = Repository::parse_short("rust-lang/rust").unwrap();
    /// assert_eq!(repo.owner(), "rust-lang");
    /// assert_eq!(repo.repo(), "rust");
    ///
    /// assert!(Repository::parse_short("invalid").is_err());
    /// assert!(Repository::parse_short("too/many/slashes").is_err());
    /// ```
    pub fn parse_short(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ConfigError::InvalidRepository(format!(
                "expected 'owner/repo' format, got '{s}'"
            )));
        }

        let owner = parts[0].trim();
        let repo = parts[1].trim();

        if owner.is_empty() || repo.is_empty() {
            return Err(ConfigError::InvalidRepository(format!(
                "owner and repo cannot be empty in '{s}'"
            )));
        }

        Ok(Self::new(owner, repo))
    }

    /// Returns the repository owner.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Returns the repository name.
    #[must_use]
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Returns the repository-specific token, if configured.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Returns the full repository name in `"owner/repo"` format.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Repository;
    ///
    /// let repo = Repository::new("rust-lang", "rust");
    /// assert_eq!(repo.full_name(), "rust-lang/rust");
    /// ```
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

impl Serialize for Repository {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // If there's no token, serialize as short format
        if self.token.is_none() {
            serializer.serialize_str(&self.full_name())
        } else {
            // Serialize as full format
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Repository", 3)?;
            state.serialize_field("owner", &self.owner)?;
            state.serialize_field("repo", &self.repo)?;
            state.serialize_field("token", &self.token)?;
            state.end()
        }
    }
}

impl<'de> Deserialize<'de> for Repository {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct RepositoryVisitor;

        impl<'de> Visitor<'de> for RepositoryVisitor {
            type Value = Repository;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string 'owner/repo' or an object with owner, repo, and optional token fields")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Repository::parse_short(v).map_err(de::Error::custom)
            }

            fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut owner: Option<String> = None;
                let mut repo: Option<String> = None;
                let mut token: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "owner" => {
                            if owner.is_some() {
                                return Err(de::Error::duplicate_field("owner"));
                            }
                            owner = Some(map.next_value()?);
                        }
                        "repo" => {
                            if repo.is_some() {
                                return Err(de::Error::duplicate_field("repo"));
                            }
                            repo = Some(map.next_value()?);
                        }
                        "token" => {
                            if token.is_some() {
                                return Err(de::Error::duplicate_field("token"));
                            }
                            token = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields for forward compatibility
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let owner = owner.ok_or_else(|| de::Error::missing_field("owner"))?;
                let repo = repo.ok_or_else(|| de::Error::missing_field("repo"))?;

                Ok(Repository { owner, repo, token })
            }
        }

        deserializer.deserialize_any(RepositoryVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_repository_without_token() {
        let repo = Repository::new("owner", "repo");
        assert_eq!(repo.owner(), "owner");
        assert_eq!(repo.repo(), "repo");
        assert!(repo.token().is_none());
    }

    #[test]
    fn with_token_creates_repository_with_token() {
        let repo = Repository::with_token("owner", "repo", "ghp_xxx");
        assert_eq!(repo.owner(), "owner");
        assert_eq!(repo.repo(), "repo");
        assert_eq!(repo.token(), Some("ghp_xxx"));
    }

    #[test]
    fn parse_short_valid() {
        let repo = Repository::parse_short("rust-lang/rust").unwrap();
        assert_eq!(repo.owner(), "rust-lang");
        assert_eq!(repo.repo(), "rust");
    }

    #[test]
    fn parse_short_with_whitespace() {
        let repo = Repository::parse_short("  owner / repo  ").unwrap();
        assert_eq!(repo.owner(), "owner");
        assert_eq!(repo.repo(), "repo");
    }

    #[test]
    fn parse_short_invalid_no_slash() {
        assert!(Repository::parse_short("invalid").is_err());
    }

    #[test]
    fn parse_short_invalid_too_many_slashes() {
        assert!(Repository::parse_short("too/many/slashes").is_err());
    }

    #[test]
    fn parse_short_invalid_empty_parts() {
        assert!(Repository::parse_short("/repo").is_err());
        assert!(Repository::parse_short("owner/").is_err());
        assert!(Repository::parse_short("/").is_err());
    }

    #[test]
    fn full_name_format() {
        let repo = Repository::new("owner", "repo");
        assert_eq!(repo.full_name(), "owner/repo");
    }

    #[test]
    fn deserialize_short_format() {
        let repo: Repository = serde_json::from_str(r#""rust-lang/rust""#).unwrap();
        assert_eq!(repo.owner(), "rust-lang");
        assert_eq!(repo.repo(), "rust");
        assert!(repo.token().is_none());
    }

    #[test]
    fn deserialize_full_format_without_token() {
        let repo: Repository =
            serde_json::from_str(r#"{"owner": "rust-lang", "repo": "rust"}"#).unwrap();
        assert_eq!(repo.owner(), "rust-lang");
        assert_eq!(repo.repo(), "rust");
        assert!(repo.token().is_none());
    }

    #[test]
    fn deserialize_full_format_with_token() {
        let repo: Repository =
            serde_json::from_str(r#"{"owner": "org", "repo": "repo", "token": "ghp_xxx"}"#)
                .unwrap();
        assert_eq!(repo.owner(), "org");
        assert_eq!(repo.repo(), "repo");
        assert_eq!(repo.token(), Some("ghp_xxx"));
    }

    #[test]
    fn deserialize_ignores_unknown_fields() {
        let repo: Repository = serde_json::from_str(
            r#"{"owner": "org", "repo": "repo", "unknown": "value", "another": 123}"#,
        )
        .unwrap();
        assert_eq!(repo.owner(), "org");
        assert_eq!(repo.repo(), "repo");
    }

    #[test]
    fn serialize_short_format() {
        let repo = Repository::new("rust-lang", "rust");
        let json = serde_json::to_string(&repo).unwrap();
        assert_eq!(json, r#""rust-lang/rust""#);
    }

    #[test]
    fn serialize_full_format_with_token() {
        let repo = Repository::with_token("org", "repo", "ghp_xxx");
        let json = serde_json::to_string(&repo).unwrap();
        // Should contain all fields
        assert!(json.contains("owner"));
        assert!(json.contains("repo"));
        assert!(json.contains("token"));
    }

    #[test]
    fn roundtrip_short_format() {
        let original = Repository::new("rust-lang", "rust");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Repository = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn roundtrip_full_format() {
        let original = Repository::with_token("org", "repo", "ghp_xxx");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Repository = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }
}
