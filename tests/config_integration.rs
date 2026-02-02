//! Integration tests for the whip-config crate.

use std::fs;
use tempfile::TempDir;
use whip_config::{Config, PollingConfig, Repository};

#[tokio::test]
async fn config_load_from_json5_file() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("whip.json5");

    fs::write(
        &config_path,
        r#"
        {
            // Configuration for whip
            repositories: [
                "rust-lang/rust",
                { owner: "tokio-rs", repo: "tokio" },
            ],
            polling: {
                interval_secs: 120,
                auto_adjust: true,
            },
            github_token: "ghp_test_token",
        }
        "#,
    )
    .unwrap();

    let config = Config::load_from(&config_path).unwrap();

    assert_eq!(config.repositories.len(), 2);
    assert_eq!(config.repositories[0].full_name(), "rust-lang/rust");
    assert_eq!(config.repositories[1].full_name(), "tokio-rs/tokio");
    assert_eq!(config.polling.interval_secs, 120);
    assert!(config.polling.auto_adjust);
    assert_eq!(config.github_token, Some("ghp_test_token".to_string()));
}

#[tokio::test]
async fn config_save_and_reload() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");

    let original = Config {
        repositories: vec![
            Repository::new("owner1", "repo1"),
            Repository::with_token("owner2", "repo2", "ghp_secret"),
        ],
        polling: PollingConfig::with_interval(90),
        github_token: Some("ghp_global".to_string()),
        sync_labels: true,
    };

    original.save_to(&config_path).unwrap();
    let loaded = Config::load_from(&config_path).unwrap();

    assert_eq!(original.repositories.len(), loaded.repositories.len());
    assert_eq!(
        original.repositories[0].full_name(),
        loaded.repositories[0].full_name()
    );
    assert_eq!(
        original.repositories[1].full_name(),
        loaded.repositories[1].full_name()
    );
    assert_eq!(
        original.repositories[1].token(),
        loaded.repositories[1].token()
    );
    assert_eq!(original.polling.interval_secs, loaded.polling.interval_secs);
    assert_eq!(original.github_token, loaded.github_token);
}

#[tokio::test]
async fn config_load_nonexistent_returns_default() {
    // Config::load() returns default when no file exists
    // We can't easily test this without controlling the working directory,
    // but we can test that load_from fails for nonexistent files
    let result = Config::load_from("/nonexistent/path/config.json");
    assert!(result.is_err());
}

#[test]
fn repository_short_format_parsing() {
    let repo = Repository::parse_short("rust-lang/rust").unwrap();
    assert_eq!(repo.owner(), "rust-lang");
    assert_eq!(repo.repo(), "rust");
    assert!(repo.token().is_none());
}

#[test]
fn repository_short_format_invalid() {
    assert!(Repository::parse_short("invalid").is_err());
    assert!(Repository::parse_short("too/many/parts").is_err());
    assert!(Repository::parse_short("/missing-owner").is_err());
    assert!(Repository::parse_short("missing-repo/").is_err());
}

#[test]
fn repository_with_token() {
    let repo = Repository::with_token("owner", "repo", "ghp_xxx");
    assert_eq!(repo.owner(), "owner");
    assert_eq!(repo.repo(), "repo");
    assert_eq!(repo.token(), Some("ghp_xxx"));
}

#[test]
fn polling_config_effective_interval() {
    // Default config with auto-adjust
    let config = PollingConfig::default();

    // Authenticated gets faster polling
    assert_eq!(config.effective_interval(true), 60);
    // Unauthenticated gets default slower polling
    assert_eq!(config.effective_interval(false), 300);

    // Fixed config ignores auth status
    let fixed = PollingConfig::fixed(30);
    assert_eq!(fixed.effective_interval(true), 30);
    assert_eq!(fixed.effective_interval(false), 30);
}

#[test]
fn polling_config_validation() {
    // Valid interval
    let valid = PollingConfig::with_interval(60);
    assert!(valid.validate().is_ok());

    // Below minimum (10 seconds)
    let too_fast = PollingConfig::fixed(5);
    assert!(too_fast.validate().is_err());

    // Above maximum (1 hour)
    let too_slow = PollingConfig::fixed(7200);
    assert!(too_slow.validate().is_err());
}

#[test]
fn config_add_remove_repository() {
    let mut config = Config::default();
    assert!(!config.has_repositories());

    config.add_repository(Repository::new("owner1", "repo1"));
    config.add_repository(Repository::new("owner2", "repo2"));
    assert!(config.has_repositories());
    assert_eq!(config.repositories.len(), 2);

    assert!(config.remove_repository("owner1/repo1"));
    assert_eq!(config.repositories.len(), 1);
    assert_eq!(config.repositories[0].full_name(), "owner2/repo2");

    assert!(!config.remove_repository("nonexistent/repo"));
    assert_eq!(config.repositories.len(), 1);
}

#[test]
fn config_validation() {
    // Valid config
    let valid = Config {
        repositories: vec![Repository::new("owner", "repo")],
        polling: PollingConfig::with_interval(60),
        github_token: Some("ghp_xxx".to_string()),
        sync_labels: true,
    };
    assert!(valid.validate().is_ok());

    // Invalid polling interval
    let invalid = Config {
        polling: PollingConfig::fixed(5),
        ..Default::default()
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn repository_serialization_short_format() {
    let repo = Repository::new("owner", "repo");
    let json = serde_json::to_string(&repo).unwrap();
    assert_eq!(json, r#""owner/repo""#);

    let parsed: Repository = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.full_name(), "owner/repo");
}

#[test]
fn repository_serialization_full_format() {
    let repo = Repository::with_token("owner", "repo", "ghp_xxx");
    let json = serde_json::to_string(&repo).unwrap();

    // Should serialize as object with all fields
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["owner"], "owner");
    assert_eq!(parsed["repo"], "repo");
    assert_eq!(parsed["token"], "ghp_xxx");
}

#[test]
fn config_github_token_not_serialized_when_none() {
    let config = Config {
        github_token: None,
        ..Default::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("github_token"));
}
