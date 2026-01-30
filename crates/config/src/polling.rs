//! Polling interval configuration with rate-limit awareness.
//!
//! This module provides the [`PollingConfig`] type which handles polling
//! intervals with automatic adjustment based on authentication status.
//!
//! # Rate Limits
//!
//! GitHub API has different rate limits depending on authentication:
//!
//! - Unauthenticated: 60 requests/hour (1 per minute)
//! - Authenticated: 5000 requests/hour (~83 per minute)
//!
//! The default polling intervals are set accordingly:
//!
//! - Unauthenticated: 300 seconds (5 minutes)
//! - Authenticated: 60 seconds (1 minute)

use serde::{Deserialize, Serialize};

/// Default polling interval for unauthenticated requests (5 minutes).
pub const DEFAULT_INTERVAL_UNAUTHENTICATED: u32 = 300;

/// Default polling interval for authenticated requests (1 minute).
pub const DEFAULT_INTERVAL_AUTHENTICATED: u32 = 60;

/// Minimum allowed polling interval (10 seconds).
pub const MIN_POLLING_INTERVAL: u32 = 10;

/// Maximum allowed polling interval (1 hour).
pub const MAX_POLLING_INTERVAL: u32 = 3600;

/// Configuration for polling behavior.
///
/// Controls how frequently the application polls for updates, with
/// automatic adjustment based on authentication status.
///
/// # Examples
///
/// ```
/// use whip_config::PollingConfig;
///
/// // Default configuration
/// let config = PollingConfig::default();
/// assert!(config.auto_adjust);
///
/// // Custom interval
/// let config = PollingConfig::with_interval(120);
/// assert_eq!(config.interval_secs, 120);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollingConfig {
    /// The polling interval in seconds.
    ///
    /// This is the base interval; if `auto_adjust` is true, the actual
    /// interval may be adjusted based on authentication status.
    #[serde(default = "default_interval")]
    pub interval_secs: u32,

    /// Whether to automatically adjust the interval based on authentication.
    ///
    /// When true:
    /// - If authenticated but interval is at the unauthenticated default,
    ///   use the authenticated default instead
    /// - If unauthenticated but interval is below the safe threshold,
    ///   increase it to avoid rate limiting
    #[serde(default = "default_auto_adjust")]
    pub auto_adjust: bool,
}

fn default_interval() -> u32 {
    DEFAULT_INTERVAL_UNAUTHENTICATED
}

fn default_auto_adjust() -> bool {
    true
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval_secs: DEFAULT_INTERVAL_UNAUTHENTICATED,
            auto_adjust: true,
        }
    }
}

impl PollingConfig {
    /// Creates a new polling configuration with the specified interval.
    ///
    /// Auto-adjust is enabled by default.
    ///
    /// # Arguments
    ///
    /// * `interval_secs` - The polling interval in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::PollingConfig;
    ///
    /// let config = PollingConfig::with_interval(120);
    /// assert_eq!(config.interval_secs, 120);
    /// assert!(config.auto_adjust);
    /// ```
    #[must_use]
    pub fn with_interval(interval_secs: u32) -> Self {
        Self {
            interval_secs,
            auto_adjust: true,
        }
    }

    /// Creates a new polling configuration with auto-adjust disabled.
    ///
    /// Use this when you want to enforce a specific interval regardless
    /// of authentication status.
    ///
    /// # Arguments
    ///
    /// * `interval_secs` - The polling interval in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::PollingConfig;
    ///
    /// let config = PollingConfig::fixed(30);
    /// assert_eq!(config.interval_secs, 30);
    /// assert!(!config.auto_adjust);
    /// ```
    #[must_use]
    pub fn fixed(interval_secs: u32) -> Self {
        Self {
            interval_secs,
            auto_adjust: false,
        }
    }

    /// Returns the effective polling interval based on authentication status.
    ///
    /// If `auto_adjust` is true, this may return a different interval than
    /// `interval_secs` based on the `is_authenticated` parameter.
    ///
    /// # Arguments
    ///
    /// * `is_authenticated` - Whether the user is authenticated with GitHub
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::PollingConfig;
    ///
    /// let config = PollingConfig::default();
    ///
    /// // Authenticated users get faster polling
    /// assert_eq!(config.effective_interval(true), 60);
    ///
    /// // Unauthenticated users get slower polling
    /// assert_eq!(config.effective_interval(false), 300);
    /// ```
    #[must_use]
    pub fn effective_interval(&self, is_authenticated: bool) -> u32 {
        if !self.auto_adjust {
            return self.interval_secs;
        }

        if is_authenticated {
            // If authenticated and using the unauthenticated default,
            // switch to the faster authenticated default
            if self.interval_secs == DEFAULT_INTERVAL_UNAUTHENTICATED {
                DEFAULT_INTERVAL_AUTHENTICATED
            } else {
                self.interval_secs
            }
        } else {
            // If unauthenticated, ensure we don't poll too fast
            self.interval_secs.max(DEFAULT_INTERVAL_UNAUTHENTICATED)
        }
    }

    /// Validates the polling configuration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the configuration is valid, or an error describing
    /// the validation failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is outside the allowed range.
    pub fn validate(&self) -> crate::Result<()> {
        if self.interval_secs < MIN_POLLING_INTERVAL {
            return Err(crate::ConfigError::InvalidPollingInterval {
                reason: format!(
                    "interval {} is below minimum of {} seconds",
                    self.interval_secs, MIN_POLLING_INTERVAL
                ),
            });
        }

        if self.interval_secs > MAX_POLLING_INTERVAL {
            return Err(crate::ConfigError::InvalidPollingInterval {
                reason: format!(
                    "interval {} exceeds maximum of {} seconds",
                    self.interval_secs, MAX_POLLING_INTERVAL
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = PollingConfig::default();
        assert_eq!(config.interval_secs, DEFAULT_INTERVAL_UNAUTHENTICATED);
        assert!(config.auto_adjust);
    }

    #[test]
    fn with_interval() {
        let config = PollingConfig::with_interval(120);
        assert_eq!(config.interval_secs, 120);
        assert!(config.auto_adjust);
    }

    #[test]
    fn fixed_interval() {
        let config = PollingConfig::fixed(30);
        assert_eq!(config.interval_secs, 30);
        assert!(!config.auto_adjust);
    }

    #[test]
    fn effective_interval_auto_adjust_authenticated() {
        let config = PollingConfig::default();
        assert_eq!(
            config.effective_interval(true),
            DEFAULT_INTERVAL_AUTHENTICATED
        );
    }

    #[test]
    fn effective_interval_auto_adjust_unauthenticated() {
        let config = PollingConfig::default();
        assert_eq!(
            config.effective_interval(false),
            DEFAULT_INTERVAL_UNAUTHENTICATED
        );
    }

    #[test]
    fn effective_interval_auto_adjust_custom_authenticated() {
        // Custom interval should be preserved for authenticated users
        let config = PollingConfig::with_interval(120);
        assert_eq!(config.effective_interval(true), 120);
    }

    #[test]
    fn effective_interval_auto_adjust_custom_unauthenticated() {
        // Custom interval below threshold should be raised for unauthenticated
        let config = PollingConfig::with_interval(30);
        assert_eq!(
            config.effective_interval(false),
            DEFAULT_INTERVAL_UNAUTHENTICATED
        );

        // Custom interval above threshold should be preserved
        let config = PollingConfig::with_interval(600);
        assert_eq!(config.effective_interval(false), 600);
    }

    #[test]
    fn effective_interval_fixed() {
        let config = PollingConfig::fixed(30);
        // Fixed interval should not change regardless of auth status
        assert_eq!(config.effective_interval(true), 30);
        assert_eq!(config.effective_interval(false), 30);
    }

    #[test]
    fn validate_valid() {
        let config = PollingConfig::with_interval(60);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_below_minimum() {
        let config = PollingConfig::fixed(5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_above_maximum() {
        let config = PollingConfig::fixed(7200);
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_at_boundaries() {
        let config = PollingConfig::fixed(MIN_POLLING_INTERVAL);
        assert!(config.validate().is_ok());

        let config = PollingConfig::fixed(MAX_POLLING_INTERVAL);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let config = PollingConfig::with_interval(120);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: PollingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn deserialize_with_defaults() {
        let json = "{}";
        let config: PollingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.interval_secs, DEFAULT_INTERVAL_UNAUTHENTICATED);
        assert!(config.auto_adjust);
    }

    #[test]
    fn deserialize_partial() {
        let json = r#"{"interval_secs": 90}"#;
        let config: PollingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.interval_secs, 90);
        assert!(config.auto_adjust); // default
    }
}
