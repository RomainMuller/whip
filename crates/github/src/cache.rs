//! Caching layer for GitHub issues.
//!
//! This module provides persistent caching of GitHub issues to reduce API calls
//! and enable offline access. Issues are stored as JSON files in the XDG data
//! directory, with support for ETags for conditional requests.
//!
//! # Directory Structure
//!
//! ```text
//! ~/.local/share/whip/       # Linux
//! ~/Library/Application Support/whip/  # macOS
//! └── cache/
//!     └── github/
//!         └── issues/
//!             └── {owner}_{repo}.json
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use std::time::Duration;
//! use whip_github::IssueCache;
//!
//! # fn example() -> whip_github::Result<()> {
//! let cache = IssueCache::new()?;
//!
//! // Check if cache is stale (older than 5 minutes)
//! if cache.is_stale("rust-lang", "rust", Duration::from_secs(300)) {
//!     println!("Cache is stale, should refresh");
//! }
//!
//! // Load cached issues
//! if let Some(cached) = cache.load("rust-lang", "rust")? {
//!     println!("Found {} cached tasks", cached.tasks.len());
//! }
//! # Ok(())
//! # }
//! ```

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use crate::error::{Error, Result};

/// Cached issues with metadata.
///
/// Contains the cached task data along with metadata needed for cache
/// validation and conditional requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedIssues {
    /// The cached issues converted to Tasks.
    pub tasks: Vec<whip_protocol::Task>,
    /// When these issues were cached.
    pub cached_at: DateTime<Utc>,
    /// ETag from the last API response (for conditional requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

impl CachedIssues {
    /// Creates a new `CachedIssues` instance with the current timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_github::CachedIssues;
    ///
    /// let cached = CachedIssues::new(vec![], Some("W/\"abc123\"".to_string()));
    /// assert!(cached.tasks.is_empty());
    /// assert_eq!(cached.etag, Some("W/\"abc123\"".to_string()));
    /// ```
    #[must_use]
    pub fn new(tasks: Vec<whip_protocol::Task>, etag: Option<String>) -> Self {
        Self {
            tasks,
            cached_at: Utc::now(),
            etag,
        }
    }

    /// Returns the age of the cache.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_github::CachedIssues;
    ///
    /// let cached = CachedIssues::new(vec![], None);
    /// // Cache was just created, so age should be very small
    /// assert!(cached.age().as_secs() < 1);
    /// ```
    #[must_use]
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.cached_at);
        // If the cached_at is in the future (clock skew), return zero
        diff.to_std().unwrap_or(Duration::ZERO)
    }

    /// Returns whether the cache is older than the given maximum age.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use whip_github::CachedIssues;
    ///
    /// let cached = CachedIssues::new(vec![], None);
    /// // Cache was just created, so it's not stale
    /// assert!(!cached.is_older_than(Duration::from_secs(60)));
    /// ```
    #[must_use]
    pub fn is_older_than(&self, max_age: Duration) -> bool {
        self.age() > max_age
    }
}

/// Cache for GitHub issues.
///
/// Provides persistent storage for GitHub issues, reducing API calls and
/// enabling offline access. The cache stores issues per repository and
/// supports ETag-based conditional requests.
///
/// # Examples
///
/// ```no_run
/// use whip_github::IssueCache;
///
/// # fn example() -> whip_github::Result<()> {
/// let cache = IssueCache::new()?;
///
/// // Load cached issues (returns None if not cached)
/// let cached = cache.load("owner", "repo")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IssueCache {
    base_path: PathBuf,
}

impl IssueCache {
    /// Creates a new cache at the XDG data directory.
    ///
    /// Creates the directory structure if it doesn't exist:
    /// - Linux: `~/.local/share/whip/cache/github/issues/`
    /// - macOS: `~/Library/Application Support/whip/cache/github/issues/`
    /// - Windows: `C:\Users\<User>\AppData\Roaming\whip\cache\github\issues\`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data directory cannot be determined (e.g., `$HOME` not set)
    /// - The directory structure cannot be created
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::IssueCache;
    ///
    /// let cache = IssueCache::new().expect("Failed to create cache");
    /// ```
    #[instrument]
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not determine data directory",
            ))
        })?;

        let base_path = data_dir
            .join("whip")
            .join("cache")
            .join("github")
            .join("issues");
        Self::with_path(base_path)
    }

    /// Creates a cache at a custom path.
    ///
    /// Useful for testing or when a non-standard cache location is required.
    /// Creates the directory if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use whip_github::IssueCache;
    ///
    /// let cache = IssueCache::with_path(PathBuf::from("/tmp/my-cache"))
    ///     .expect("Failed to create cache");
    /// ```
    #[instrument]
    pub fn with_path(base_path: PathBuf) -> Result<Self> {
        debug!(?base_path, "creating issue cache");

        if !base_path.exists() {
            debug!(?base_path, "creating cache directory");
            fs::create_dir_all(&base_path)?;
        }

        Ok(Self { base_path })
    }

    /// Loads cached issues for a repository.
    ///
    /// Returns `None` if the cache file doesn't exist. Returns an error if
    /// the file exists but cannot be read or parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cache file exists but cannot be read (permissions, etc.)
    /// - The cache file contains invalid JSON
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::IssueCache;
    ///
    /// # fn example() -> whip_github::Result<()> {
    /// let cache = IssueCache::new()?;
    ///
    /// match cache.load("rust-lang", "rust")? {
    ///     Some(cached) => println!("Found {} tasks", cached.tasks.len()),
    ///     None => println!("No cache found"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub fn load(&self, owner: &str, repo: &str) -> Result<Option<CachedIssues>> {
        let path = self.cache_path(owner, repo);
        debug!(?path, "loading cached issues");

        match fs::read_to_string(&path) {
            Ok(content) => {
                let cached: CachedIssues = serde_json::from_str(&content).map_err(|e| {
                    warn!(?path, error = %e, "failed to parse cache file");
                    Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("failed to parse cache file: {e}"),
                    ))
                })?;
                debug!(
                    tasks = cached.tasks.len(),
                    cached_at = %cached.cached_at,
                    "loaded cached issues"
                );
                Ok(Some(cached))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!(?path, "cache file not found");
                Ok(None)
            }
            Err(e) => {
                warn!(?path, error = %e, "failed to read cache file");
                Err(Error::Io(e))
            }
        }
    }

    /// Saves issues to cache.
    ///
    /// Overwrites any existing cache file for the repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file cannot be written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::{CachedIssues, IssueCache};
    ///
    /// # fn example() -> whip_github::Result<()> {
    /// let cache = IssueCache::new()?;
    /// let cached = CachedIssues::new(vec![], Some("W/\"abc\"".to_string()));
    ///
    /// cache.save("owner", "repo", &cached)?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, cached), fields(tasks = cached.tasks.len()))]
    pub fn save(&self, owner: &str, repo: &str, cached: &CachedIssues) -> Result<()> {
        let path = self.cache_path(owner, repo);
        debug!(?path, "saving cached issues");

        let content = serde_json::to_string_pretty(cached).map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("failed to serialize cache: {e}"),
            ))
        })?;

        fs::write(&path, content)?;
        debug!(?path, "cache saved successfully");

        Ok(())
    }

    /// Gets stored ETag for conditional requests.
    ///
    /// Returns `None` if there's no cached data or if the cached data
    /// doesn't have an ETag.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use whip_github::IssueCache;
    ///
    /// # fn example() -> whip_github::Result<()> {
    /// let cache = IssueCache::new()?;
    ///
    /// if let Some(etag) = cache.get_etag("owner", "repo") {
    ///     println!("Can use If-None-Match: {}", etag);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get_etag(&self, owner: &str, repo: &str) -> Option<String> {
        self.load(owner, repo)
            .ok()
            .flatten()
            .and_then(|cached| cached.etag)
    }

    /// Checks if cache is stale (older than max_age).
    ///
    /// Returns `true` if:
    /// - The cache doesn't exist
    /// - The cache exists but is older than `max_age`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use whip_github::IssueCache;
    ///
    /// # fn example() -> whip_github::Result<()> {
    /// let cache = IssueCache::new()?;
    ///
    /// // Check if cache is older than 5 minutes
    /// if cache.is_stale("owner", "repo", Duration::from_secs(300)) {
    ///     println!("Should refresh the cache");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_stale(&self, owner: &str, repo: &str, max_age: Duration) -> bool {
        match self.load(owner, repo) {
            Ok(Some(cached)) => cached.is_older_than(max_age),
            Ok(None) | Err(_) => true,
        }
    }

    /// Returns the cache file path for a repository.
    ///
    /// The filename is `{owner}_{repo}.json`, with owner and repo sanitized
    /// to replace any path separators.
    fn cache_path(&self, owner: &str, repo: &str) -> PathBuf {
        // Sanitize owner and repo to prevent path traversal
        let safe_owner = owner.replace(['/', '\\', '.'], "_");
        let safe_repo = repo.replace(['/', '\\', '.'], "_");
        self.base_path
            .join(format!("{safe_owner}_{safe_repo}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use whip_protocol::{LaneKind, Task, TaskState};

    fn create_test_cache() -> (IssueCache, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache =
            IssueCache::with_path(temp_dir.path().to_path_buf()).expect("Failed to create cache");
        (cache, temp_dir)
    }

    fn create_test_task(title: &str) -> Task {
        let mut task = Task::new(title, "Test description");
        task.state = TaskState::Idle;
        task.lane = LaneKind::Backlog;
        task
    }

    #[test]
    fn cache_path_sanitizes_input() {
        let (cache, _temp) = create_test_cache();

        // Normal case
        let path = cache.cache_path("owner", "repo");
        assert!(path.ends_with("owner_repo.json"));

        // Path traversal attempt
        let path = cache.cache_path("../evil", "repo");
        assert!(path.ends_with("___evil_repo.json"));

        // Dots in names
        let path = cache.cache_path("owner.name", "repo.name");
        assert!(path.ends_with("owner_name_repo_name.json"));
    }

    #[test]
    fn load_returns_none_for_missing_cache() {
        let (cache, _temp) = create_test_cache();

        let result = cache
            .load("nonexistent", "repo")
            .expect("load should not fail");
        assert!(result.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (cache, _temp) = create_test_cache();

        let tasks = vec![create_test_task("Task 1"), create_test_task("Task 2")];
        let cached = CachedIssues::new(tasks.clone(), Some("W/\"etag123\"".to_string()));

        cache
            .save("owner", "repo", &cached)
            .expect("save should succeed");

        let loaded = cache
            .load("owner", "repo")
            .expect("load should succeed")
            .expect("cache should exist");

        assert_eq!(loaded.tasks.len(), 2);
        assert_eq!(loaded.tasks[0].title, "Task 1");
        assert_eq!(loaded.tasks[1].title, "Task 2");
        assert_eq!(loaded.etag, Some("W/\"etag123\"".to_string()));
    }

    #[test]
    fn save_overwrites_existing_cache() {
        let (cache, _temp) = create_test_cache();

        // Save first version
        let cached1 = CachedIssues::new(vec![create_test_task("Old Task")], None);
        cache.save("owner", "repo", &cached1).expect("first save");

        // Save second version
        let cached2 = CachedIssues::new(vec![create_test_task("New Task")], None);
        cache.save("owner", "repo", &cached2).expect("second save");

        let loaded = cache
            .load("owner", "repo")
            .expect("load")
            .expect("cache exists");
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].title, "New Task");
    }

    #[test]
    fn get_etag_returns_stored_etag() {
        let (cache, _temp) = create_test_cache();

        let cached = CachedIssues::new(vec![], Some("W/\"test-etag\"".to_string()));
        cache.save("owner", "repo", &cached).expect("save");

        let etag = cache.get_etag("owner", "repo");
        assert_eq!(etag, Some("W/\"test-etag\"".to_string()));
    }

    #[test]
    fn get_etag_returns_none_when_no_cache() {
        let (cache, _temp) = create_test_cache();

        let etag = cache.get_etag("nonexistent", "repo");
        assert!(etag.is_none());
    }

    #[test]
    fn get_etag_returns_none_when_no_etag_stored() {
        let (cache, _temp) = create_test_cache();

        let cached = CachedIssues::new(vec![], None);
        cache.save("owner", "repo", &cached).expect("save");

        let etag = cache.get_etag("owner", "repo");
        assert!(etag.is_none());
    }

    #[test]
    fn is_stale_returns_true_for_missing_cache() {
        let (cache, _temp) = create_test_cache();

        assert!(cache.is_stale("nonexistent", "repo", Duration::from_secs(300)));
    }

    #[test]
    fn is_stale_returns_false_for_fresh_cache() {
        let (cache, _temp) = create_test_cache();

        let cached = CachedIssues::new(vec![], None);
        cache.save("owner", "repo", &cached).expect("save");

        // Cache was just created, so 5 minutes should be plenty fresh
        assert!(!cache.is_stale("owner", "repo", Duration::from_secs(300)));
    }

    #[test]
    fn is_stale_returns_true_for_zero_max_age() {
        let (cache, _temp) = create_test_cache();

        let cached = CachedIssues::new(vec![], None);
        cache.save("owner", "repo", &cached).expect("save");

        // Zero max_age means always stale
        assert!(cache.is_stale("owner", "repo", Duration::ZERO));
    }

    #[test]
    fn cached_issues_age_is_small_when_just_created() {
        let cached = CachedIssues::new(vec![], None);
        assert!(cached.age().as_secs() < 1);
    }

    #[test]
    fn cached_issues_is_older_than_works() {
        let cached = CachedIssues::new(vec![], None);

        // Just created, so not older than 1 second
        assert!(!cached.is_older_than(Duration::from_secs(1)));

        // But older than 0 seconds (technically)
        // Note: This might occasionally fail due to timing, but is_older_than uses > not >=
        // so if age is exactly 0, it won't be considered older than 0
    }

    #[test]
    fn load_returns_error_for_invalid_json() {
        let (cache, temp) = create_test_cache();

        // Write invalid JSON
        let path = temp.path().join("owner_repo.json");
        fs::write(&path, "not valid json").expect("write invalid json");

        let result = cache.load("owner", "repo");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn multiple_repos_are_isolated() {
        let (cache, _temp) = create_test_cache();

        let cached1 = CachedIssues::new(vec![create_test_task("Repo 1 Task")], None);
        let cached2 = CachedIssues::new(vec![create_test_task("Repo 2 Task")], None);

        cache.save("owner", "repo1", &cached1).expect("save repo1");
        cache.save("owner", "repo2", &cached2).expect("save repo2");

        let loaded1 = cache.load("owner", "repo1").expect("load").expect("exists");
        let loaded2 = cache.load("owner", "repo2").expect("load").expect("exists");

        assert_eq!(loaded1.tasks[0].title, "Repo 1 Task");
        assert_eq!(loaded2.tasks[0].title, "Repo 2 Task");
    }

    #[test]
    fn different_owners_same_repo_are_isolated() {
        let (cache, _temp) = create_test_cache();

        let cached1 = CachedIssues::new(vec![create_test_task("Owner 1 Task")], None);
        let cached2 = CachedIssues::new(vec![create_test_task("Owner 2 Task")], None);

        cache.save("owner1", "repo", &cached1).expect("save owner1");
        cache.save("owner2", "repo", &cached2).expect("save owner2");

        let loaded1 = cache.load("owner1", "repo").expect("load").expect("exists");
        let loaded2 = cache.load("owner2", "repo").expect("load").expect("exists");

        assert_eq!(loaded1.tasks[0].title, "Owner 1 Task");
        assert_eq!(loaded2.tasks[0].title, "Owner 2 Task");
    }

    #[test]
    fn with_path_creates_directory() {
        let temp = TempDir::new().expect("temp dir");
        let cache_path = temp.path().join("nested").join("cache").join("path");

        // Directory doesn't exist yet
        assert!(!cache_path.exists());

        let cache = IssueCache::with_path(cache_path.clone()).expect("create cache");

        // Now it should exist
        assert!(cache_path.exists());

        // And we should be able to use it
        let cached = CachedIssues::new(vec![], None);
        cache.save("owner", "repo", &cached).expect("save");
    }
}
