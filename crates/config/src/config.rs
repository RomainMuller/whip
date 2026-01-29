//! Core configuration struct and loading logic.
//!
//! This module provides the main [`Config`] struct which aggregates all
//! configuration options for the whip application.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::persistence::{find_config_file, read_config_file, write_config_file};
use crate::polling::PollingConfig;
use crate::repository::Repository;

/// The main configuration struct for the whip application.
///
/// This struct is the central point for all application configuration,
/// including repository settings, polling behavior, and authentication.
///
/// # Examples
///
/// ```
/// use whip_config::{Config, Repository, PollingConfig};
///
/// // Create a default config
/// let config = Config::default();
/// assert!(config.repositories.is_empty());
///
/// // Create a custom config
/// let config = Config {
///     repositories: vec![Repository::new("rust-lang", "rust")],
///     polling: PollingConfig::with_interval(120),
///     github_token: Some("ghp_xxx".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// List of GitHub repositories to monitor.
    ///
    /// Repositories can be specified in short format (`"owner/repo"`) or
    /// full format with optional token override.
    #[serde(default)]
    pub repositories: Vec<Repository>,

    /// Polling configuration.
    ///
    /// Controls how frequently the application checks for updates.
    #[serde(default)]
    pub polling: PollingConfig,

    /// Global GitHub token.
    ///
    /// Used for all repositories that don't have a specific token configured.
    /// If not set, the application will try to get a token from the `gh` CLI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,
}

impl Config {
    /// Creates a new empty configuration.
    ///
    /// This is equivalent to `Config::default()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Config;
    ///
    /// let config = Config::new();
    /// assert!(config.repositories.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration from the default file locations.
    ///
    /// Searches for configuration files in the following order:
    ///
    /// 1. Local: `./whip.json5` or `./whip.json`
    /// 2. User: `~/.config/whip/config.json5` or `~/.config/whip/config.json`
    ///
    /// If no configuration file is found, returns a default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if a configuration file is found but cannot be
    /// read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_config::Config;
    ///
    /// # async fn example() -> whip_config::Result<()> {
    /// let config = Config::load().await?;
    /// println!("Loaded {} repositories", config.repositories.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load() -> Result<Self> {
        match find_config_file() {
            Some(path) => {
                let config: Config = read_config_file(&path)?;
                config.validate()?;
                Ok(config)
            }
            None => Ok(Self::default()),
        }
    }

    /// Loads configuration from a specific file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_config::Config;
    ///
    /// # fn example() -> whip_config::Result<()> {
    /// let config = Config::load_from("custom-config.json5")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let config: Config = read_config_file(path)?;
        config.validate()?;
        Ok(config)
    }

    /// Saves the configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to save to
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_config::Config;
    ///
    /// # fn example() -> whip_config::Result<()> {
    /// let config = Config::default();
    /// config.save_to("my-config.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_to(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        write_config_file(path, self)
    }

    /// Validates the configuration.
    ///
    /// Checks that all configuration values are within acceptable ranges
    /// and that required fields are properly set.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::{Config, PollingConfig};
    ///
    /// let mut config = Config::default();
    /// assert!(config.validate().is_ok());
    ///
    /// // Invalid polling interval
    /// config.polling = PollingConfig::fixed(5); // Below minimum
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<()> {
        self.polling.validate()?;
        Ok(())
    }

    /// Returns whether the configuration has any repositories.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::{Config, Repository};
    ///
    /// let config = Config::default();
    /// assert!(!config.has_repositories());
    ///
    /// let config = Config {
    ///     repositories: vec![Repository::new("owner", "repo")],
    ///     ..Default::default()
    /// };
    /// assert!(config.has_repositories());
    /// ```
    #[must_use]
    pub fn has_repositories(&self) -> bool {
        !self.repositories.is_empty()
    }

    /// Adds a repository to the configuration.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository to add
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::{Config, Repository};
    ///
    /// let mut config = Config::default();
    /// config.add_repository(Repository::new("rust-lang", "rust"));
    /// assert_eq!(config.repositories.len(), 1);
    /// ```
    pub fn add_repository(&mut self, repo: Repository) {
        self.repositories.push(repo);
    }

    /// Removes a repository by its full name (`"owner/repo"`).
    ///
    /// # Arguments
    ///
    /// * `full_name` - The full repository name to remove
    ///
    /// # Returns
    ///
    /// Returns `true` if a repository was removed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::{Config, Repository};
    ///
    /// let mut config = Config {
    ///     repositories: vec![Repository::new("rust-lang", "rust")],
    ///     ..Default::default()
    /// };
    ///
    /// assert!(config.remove_repository("rust-lang/rust"));
    /// assert!(!config.remove_repository("nonexistent/repo"));
    /// ```
    #[must_use]
    pub fn remove_repository(&mut self, full_name: &str) -> bool {
        let initial_len = self.repositories.len();
        self.repositories.retain(|r| r.full_name() != full_name);
        self.repositories.len() < initial_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert!(config.repositories.is_empty());
        assert!(config.github_token.is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn new_config() {
        let config = Config::new();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn has_repositories() {
        let mut config = Config::default();
        assert!(!config.has_repositories());

        config.repositories.push(Repository::new("owner", "repo"));
        assert!(config.has_repositories());
    }

    #[test]
    fn add_repository() {
        let mut config = Config::default();
        config.add_repository(Repository::new("owner", "repo"));
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(config.repositories[0].full_name(), "owner/repo");
    }

    #[test]
    fn remove_repository() {
        let mut config = Config {
            repositories: vec![
                Repository::new("owner1", "repo1"),
                Repository::new("owner2", "repo2"),
            ],
            ..Default::default()
        };

        assert!(config.remove_repository("owner1/repo1"));
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(config.repositories[0].full_name(), "owner2/repo2");

        assert!(!config.remove_repository("nonexistent/repo"));
        assert_eq!(config.repositories.len(), 1);
    }

    #[test]
    fn validate_valid_config() {
        let config = Config {
            repositories: vec![Repository::new("owner", "repo")],
            polling: PollingConfig::with_interval(60),
            github_token: Some("ghp_xxx".to_string()),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_invalid_polling() {
        let config = Config {
            polling: PollingConfig::fixed(5), // Below minimum
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let config = Config {
            repositories: vec![
                Repository::new("rust-lang", "rust"),
                Repository::with_token("private", "repo", "ghp_xxx"),
            ],
            polling: PollingConfig::with_interval(120),
            github_token: Some("ghp_global".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn deserialize_with_defaults() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.repositories.is_empty());
        assert!(config.github_token.is_none());
    }

    #[test]
    fn deserialize_partial() {
        let json = r#"{"repositories": ["owner/repo"]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(config.repositories[0].full_name(), "owner/repo");
    }

    #[test]
    fn load_from_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json5");
        std::fs::write(
            &path,
            r#"
            {
                repositories: [
                    "rust-lang/rust",
                    { owner: "tokio-rs", repo: "tokio" }
                ],
                polling: { interval_secs: 90 },
                github_token: "ghp_test"
            }
            "#,
        )
        .unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.repositories.len(), 2);
        assert_eq!(config.repositories[0].full_name(), "rust-lang/rust");
        assert_eq!(config.repositories[1].full_name(), "tokio-rs/tokio");
        assert_eq!(config.polling.interval_secs, 90);
        assert_eq!(config.github_token, Some("ghp_test".to_string()));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");

        let original = Config {
            repositories: vec![Repository::new("owner", "repo")],
            polling: PollingConfig::with_interval(120),
            github_token: Some("ghp_xxx".to_string()),
        };

        original.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn github_token_not_serialized_when_none() {
        let config = Config {
            github_token: None,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("github_token"));
    }
}
