//! TUI message types for event handling.
//!
//! This module defines the message enum used for communication between
//! the TUI input handler and the application state.

use serde::{Deserialize, Serialize};

/// Messages that represent user actions in the TUI.
///
/// These messages are produced by the input handler and consumed by
/// the application state to update the UI.
///
/// # Examples
///
/// ```
/// use whip_protocol::Message;
///
/// let msg = Message::NavigateRight;
/// assert!(matches!(msg, Message::NavigateRight));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// Move selection to the left lane.
    NavigateLeft,
    /// Move selection to the right lane.
    NavigateRight,
    /// Move selection up within the current lane.
    NavigateUp,
    /// Move selection down within the current lane.
    NavigateDown,
    /// Select the currently highlighted item.
    Select,
    /// Go back to the previous view or cancel current action.
    Back,
    /// Escape: close panel or clear selection (contextual).
    Escape,
    /// Quit the application.
    Quit,
    /// Refresh the board state.
    Refresh,
    /// Toggle help overlay.
    ToggleHelp,
    /// Mouse click at coordinates (column, row).
    ClickAt {
        /// Column (x coordinate) of the click.
        column: u16,
        /// Row (y coordinate) of the click.
        row: u16,
    },

    // --- Settings messages ---
    /// Open the settings panel.
    OpenSettings,
    /// Close the settings panel.
    CloseSettings,
    /// Move to the next settings section.
    SettingsNextSection,
    /// Move to the previous settings section.
    SettingsPrevSection,
    /// Navigate within the current settings section.
    SettingsNavigate {
        /// Direction to navigate (positive = down, negative = up).
        delta: i32,
    },
    /// Start editing the selected setting.
    SettingsEdit,
    /// Confirm the current edit.
    SettingsConfirm,
    /// Cancel the current edit.
    SettingsCancel,
    /// Delete the selected item (e.g., a repository).
    SettingsDelete,
    /// Save settings to file.
    SettingsSave,
    /// Input a character while editing.
    SettingsInput {
        /// The character that was input.
        ch: char,
    },
    /// Delete the last character while editing.
    SettingsBackspace,
    /// Switch to the next field in multi-field edit mode (e.g., Tab between path and token).
    SettingsSwitchField,
}

impl Message {
    /// Returns `true` if this message is a navigation action.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::Message;
    ///
    /// assert!(Message::NavigateLeft.is_navigation());
    /// assert!(Message::NavigateUp.is_navigation());
    /// assert!(!Message::Select.is_navigation());
    /// ```
    #[must_use]
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            Self::NavigateLeft
                | Self::NavigateRight
                | Self::NavigateUp
                | Self::NavigateDown
                | Self::SettingsNavigate { .. }
                | Self::SettingsNextSection
                | Self::SettingsPrevSection
        )
    }

    /// Returns `true` if this message should terminate the application.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::Message;
    ///
    /// assert!(Message::Quit.is_terminating());
    /// assert!(!Message::Back.is_terminating());
    /// ```
    #[must_use]
    pub fn is_terminating(&self) -> bool {
        matches!(self, Self::Quit)
    }

    /// Returns `true` if this message is a settings-related action.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::Message;
    ///
    /// assert!(Message::OpenSettings.is_settings());
    /// assert!(Message::SettingsSave.is_settings());
    /// assert!(!Message::NavigateLeft.is_settings());
    /// ```
    #[must_use]
    pub fn is_settings(&self) -> bool {
        matches!(
            self,
            Self::OpenSettings
                | Self::CloseSettings
                | Self::SettingsNextSection
                | Self::SettingsPrevSection
                | Self::SettingsNavigate { .. }
                | Self::SettingsEdit
                | Self::SettingsConfirm
                | Self::SettingsCancel
                | Self::SettingsDelete
                | Self::SettingsSave
                | Self::SettingsInput { .. }
                | Self::SettingsBackspace
                | Self::SettingsSwitchField
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_navigation_detection() {
        assert!(Message::NavigateLeft.is_navigation());
        assert!(Message::NavigateRight.is_navigation());
        assert!(Message::NavigateUp.is_navigation());
        assert!(Message::NavigateDown.is_navigation());
        assert!(Message::SettingsNavigate { delta: 1 }.is_navigation());
        assert!(Message::SettingsNextSection.is_navigation());
        assert!(Message::SettingsPrevSection.is_navigation());
        assert!(!Message::Select.is_navigation());
        assert!(!Message::Back.is_navigation());
        assert!(!Message::Quit.is_navigation());
    }

    #[test]
    fn message_terminating_detection() {
        assert!(Message::Quit.is_terminating());
        assert!(!Message::Back.is_terminating());
        assert!(!Message::Select.is_terminating());
    }

    #[test]
    fn message_settings_detection() {
        assert!(Message::OpenSettings.is_settings());
        assert!(Message::CloseSettings.is_settings());
        assert!(Message::SettingsNextSection.is_settings());
        assert!(Message::SettingsPrevSection.is_settings());
        assert!(Message::SettingsNavigate { delta: 1 }.is_settings());
        assert!(Message::SettingsEdit.is_settings());
        assert!(Message::SettingsConfirm.is_settings());
        assert!(Message::SettingsCancel.is_settings());
        assert!(Message::SettingsDelete.is_settings());
        assert!(Message::SettingsSave.is_settings());
        assert!(Message::SettingsInput { ch: 'a' }.is_settings());
        assert!(Message::SettingsBackspace.is_settings());
        assert!(Message::SettingsSwitchField.is_settings());
        assert!(!Message::NavigateLeft.is_settings());
        assert!(!Message::Quit.is_settings());
    }

    #[test]
    fn message_serialization_roundtrip() {
        let messages = vec![
            Message::NavigateLeft,
            Message::NavigateRight,
            Message::NavigateUp,
            Message::NavigateDown,
            Message::Select,
            Message::Back,
            Message::Escape,
            Message::Quit,
            Message::Refresh,
            Message::ToggleHelp,
            Message::ClickAt { column: 10, row: 5 },
            Message::OpenSettings,
            Message::CloseSettings,
            Message::SettingsNextSection,
            Message::SettingsPrevSection,
            Message::SettingsNavigate { delta: -1 },
            Message::SettingsEdit,
            Message::SettingsConfirm,
            Message::SettingsCancel,
            Message::SettingsDelete,
            Message::SettingsSave,
            Message::SettingsInput { ch: 'x' },
            Message::SettingsBackspace,
            Message::SettingsSwitchField,
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).expect("serialize");
            let parsed: Message = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(msg, parsed);
        }
    }

    #[test]
    fn message_json_format() {
        let json = serde_json::to_string(&Message::NavigateLeft).expect("serialize");
        assert_eq!(json, r#""navigate_left""#);

        let json = serde_json::to_string(&Message::Refresh).expect("serialize");
        assert_eq!(json, r#""refresh""#);

        let json = serde_json::to_string(&Message::OpenSettings).expect("serialize");
        assert_eq!(json, r#""open_settings""#);
    }
}
