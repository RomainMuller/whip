//! Configuration file reading and writing.
//!
//! This module handles loading configuration from files and saving
//! configuration back to files.
//!
//! # File Formats
//!
//! The module supports both JSON5 and JSON formats:
//!
//! - JSON5 (`.json5`): Preferred format with comments and trailing commas
//! - JSON (`.json`): Standard JSON format
//!
//! # File Locations
//!
//! Configuration is searched in the following order:
//!
//! 1. Local: `./whip.json5` or `./whip.json`
//! 2. User: `~/.config/whip/config.json5` or `~/.config/whip/config.json`

use std::path::{Path, PathBuf};

use crate::error::{ConfigError, Result};

/// Configuration file names to search for, in priority order.
const CONFIG_FILE_NAMES: &[&str] = &["whip.json5", "whip.json"];

/// User config directory name.
const USER_CONFIG_DIR: &str = "whip";

/// User config file names to search for, in priority order.
const USER_CONFIG_FILE_NAMES: &[&str] = &["config.json5", "config.json"];

/// Finds the configuration file path.
///
/// Searches in the following order:
///
/// 1. Local directory: `./whip.json5`, `./whip.json`
/// 2. User config directory: `~/.config/whip/config.json5`, `~/.config/whip/config.json`
///
/// # Returns
///
/// Returns `Some(path)` if a config file is found, `None` otherwise.
///
/// # Examples
///
/// ```no_run
/// use whip_config::persistence::find_config_file;
///
/// if let Some(path) = find_config_file() {
///     println!("Found config at: {}", path.display());
/// }
/// ```
#[must_use]
pub fn find_config_file() -> Option<PathBuf> {
    // Try local directory first
    for name in CONFIG_FILE_NAMES {
        let path = PathBuf::from(name);
        if path.exists() {
            return Some(path);
        }
    }

    // Try user config directory
    if let Some(config_dir) = dirs::config_dir() {
        let whip_config_dir = config_dir.join(USER_CONFIG_DIR);
        for name in USER_CONFIG_FILE_NAMES {
            let path = whip_config_dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Returns the default user configuration directory.
///
/// This is typically `~/.config/whip/` on Unix systems.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
///
/// # Examples
///
/// ```no_run
/// use whip_config::persistence::user_config_dir;
///
/// let dir = user_config_dir().unwrap();
/// println!("User config dir: {}", dir.display());
/// ```
pub fn user_config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join(USER_CONFIG_DIR))
        .ok_or(ConfigError::NoHomeDirectory)
}

/// Returns the default user configuration file path.
///
/// This is typically `~/.config/whip/config.json5`.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
///
/// # Examples
///
/// ```no_run
/// use whip_config::persistence::default_user_config_path;
///
/// let path = default_user_config_path().unwrap();
/// println!("Default config path: {}", path.display());
/// ```
pub fn default_user_config_path() -> Result<PathBuf> {
    Ok(user_config_dir()?.join("config.json5"))
}

/// Reads and parses a configuration file.
///
/// Supports both JSON5 and JSON formats.
///
/// # Arguments
///
/// * `path` - The path to the configuration file
///
/// # Type Parameters
///
/// * `T` - The type to deserialize into (must implement `serde::Deserialize`)
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The file content cannot be parsed
///
/// # Examples
///
/// ```no_run
/// use whip_config::persistence::read_config_file;
/// use whip_config::Config;
///
/// # fn main() -> whip_config::Result<()> {
/// let config: Config = read_config_file("whip.json5")?;
/// # Ok(())
/// # }
/// ```
pub fn read_config_file<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFile {
        path: path.to_path_buf(),
        source: e,
    })?;

    // JSON5 parser handles both JSON5 and JSON
    serde_json5::from_str(&content).map_err(ConfigError::from)
}

/// Writes a configuration to a file.
///
/// The configuration is written as pretty-printed JSON (not JSON5, as
/// serde_json5 doesn't support serialization to JSON5 format).
///
/// # Arguments
///
/// * `path` - The path to write to
/// * `config` - The configuration to write
///
/// # Errors
///
/// Returns an error if:
/// - The parent directory cannot be created
/// - The file cannot be written
/// - The configuration cannot be serialized
///
/// # Examples
///
/// ```no_run
/// use whip_config::persistence::write_config_file;
/// use whip_config::Config;
///
/// # fn main() -> whip_config::Result<()> {
/// let config = Config::default();
/// write_config_file("whip.json", &config)?;
/// # Ok(())
/// # }
/// ```
pub fn write_config_file<T: serde::Serialize>(path: impl AsRef<Path>, config: &T) -> Result<()> {
    let path = path.as_ref();

    // Create parent directories if needed
    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFile {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    // Serialize to pretty JSON
    let content = serde_json::to_string_pretty(config)?;

    std::fs::write(path, content).map_err(|e| ConfigError::WriteFile {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    #[test]
    fn read_json_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json");
        std::fs::write(&path, r#"{"name": "test", "value": 42}"#).unwrap();

        let config: TestConfig = read_config_file(&path).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.value, 42);
    }

    #[test]
    fn read_json5_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json5");
        std::fs::write(
            &path,
            r#"
            {
                // This is a comment
                name: "test",
                value: 42,  // trailing comma
            }
            "#,
        )
        .unwrap();

        let config: TestConfig = read_config_file(&path).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.value, 42);
    }

    #[test]
    fn read_nonexistent_file() {
        let result: Result<TestConfig> = read_config_file("/nonexistent/path.json");
        assert!(result.is_err());
    }

    #[test]
    fn read_invalid_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("invalid.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result: Result<TestConfig> = read_config_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("roundtrip.json");

        let original = TestConfig {
            name: "test".to_string(),
            value: 42,
        };

        write_config_file(&path, &original).unwrap();
        let loaded: TestConfig = read_config_file(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("dirs").join("config.json");

        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };

        write_config_file(&path, &config).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn user_config_dir_returns_path() {
        // This test may fail in environments without a home directory
        if dirs::config_dir().is_some() {
            let result = user_config_dir();
            assert!(result.is_ok());
            assert!(result.unwrap().ends_with(USER_CONFIG_DIR));
        }
    }
}
