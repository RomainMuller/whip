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
/// use taim_protocol::Message;
///
/// let msg = Message::NavigateRight;
/// assert!(matches!(msg, Message::NavigateRight));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Quit the application.
    Quit,
    /// Refresh the board state.
    Refresh,
    /// Toggle help overlay.
    ToggleHelp,
}

impl Message {
    /// Returns `true` if this message is a navigation action.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::Message;
    ///
    /// assert!(Message::NavigateLeft.is_navigation());
    /// assert!(Message::NavigateUp.is_navigation());
    /// assert!(!Message::Select.is_navigation());
    /// ```
    #[must_use]
    pub const fn is_navigation(self) -> bool {
        matches!(
            self,
            Self::NavigateLeft | Self::NavigateRight | Self::NavigateUp | Self::NavigateDown
        )
    }


    /// Returns `true` if this message should terminate the application.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::Message;
    ///
    /// assert!(Message::Quit.is_terminating());
    /// assert!(!Message::Back.is_terminating());
    /// ```
    #[must_use]
    pub const fn is_terminating(self) -> bool {
        matches!(self, Self::Quit)
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
    fn message_serialization_roundtrip() {
        let messages = [
            Message::NavigateLeft,
            Message::NavigateRight,
            Message::NavigateUp,
            Message::NavigateDown,
            Message::Select,
            Message::Back,
            Message::Quit,
            Message::Refresh,
            Message::ToggleHelp,
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
    }
}
