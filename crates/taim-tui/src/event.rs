//! Event handling and key mappings.
//!
//! This module provides event polling and conversion from terminal events
//! to application messages.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use taim_protocol::Message;

/// Default poll timeout for events.
const POLL_TIMEOUT: Duration = Duration::from_millis(100);

/// Polls for a terminal event with the default timeout.
///
/// Returns `Some(Event)` if an event is available within the timeout,
/// or `None` if the timeout expires without an event.
///
/// # Errors
///
/// Returns an error if polling the terminal fails.
pub fn poll_event() -> std::io::Result<Option<Event>> {
    if event::poll(POLL_TIMEOUT)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Converts a terminal key event to an application message.
///
/// Returns `Some(Message)` if the key event maps to an action,
/// or `None` if the key is not bound.
///
/// # Key Bindings
///
/// | Key | Action |
/// |-----|--------|
/// | `Ctrl+C` | Quit |
/// | `Esc` | Escape (close panel or clear selection) |
/// | `Left` | Navigate left |
/// | `Right` | Navigate right |
/// | `Up` | Navigate up |
/// | `Down` | Navigate down |
/// | `Enter` or `Space` | Select |
/// | `Backspace` | Back |
/// | `r` | Refresh |
/// | `?` | Toggle help |
#[must_use]
pub fn key_to_message(key: KeyEvent) -> Option<Message> {
    // Check for Ctrl+C first
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    // Regular keys
    match key.code {
        // Escape (contextual: close panel or clear selection)
        KeyCode::Esc => Some(Message::Escape),

        // Navigation (arrow keys only)
        KeyCode::Left => Some(Message::NavigateLeft),
        KeyCode::Right => Some(Message::NavigateRight),
        KeyCode::Up => Some(Message::NavigateUp),
        KeyCode::Down => Some(Message::NavigateDown),

        // Selection
        KeyCode::Enter | KeyCode::Char(' ') => Some(Message::Select),
        KeyCode::Backspace => Some(Message::Back),

        // Other actions
        KeyCode::Char('r') => Some(Message::Refresh),
        KeyCode::Char('?') => Some(Message::ToggleHelp),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_key_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: event::KeyEventState::NONE,
        }
    }

    #[test]
    fn quit_keys() {
        // Only Ctrl+C quits
        assert_eq!(
            key_to_message(make_key_with_modifiers(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            )),
            Some(Message::Quit)
        );
        // 'q' is no longer a quit key
        assert_eq!(key_to_message(make_key(KeyCode::Char('q'))), None);
    }

    #[test]
    fn escape_key() {
        assert_eq!(
            key_to_message(make_key(KeyCode::Esc)),
            Some(Message::Escape)
        );
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(
            key_to_message(make_key(KeyCode::Left)),
            Some(Message::NavigateLeft)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Right)),
            Some(Message::NavigateRight)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Up)),
            Some(Message::NavigateUp)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Down)),
            Some(Message::NavigateDown)
        );
    }

    #[test]
    fn vim_keys_not_mapped() {
        // Vim-style hjkl should NOT be mapped
        assert_eq!(key_to_message(make_key(KeyCode::Char('h'))), None);
        assert_eq!(key_to_message(make_key(KeyCode::Char('j'))), None);
        assert_eq!(key_to_message(make_key(KeyCode::Char('k'))), None);
        assert_eq!(key_to_message(make_key(KeyCode::Char('l'))), None);
    }

    #[test]
    fn selection_keys() {
        assert_eq!(
            key_to_message(make_key(KeyCode::Enter)),
            Some(Message::Select)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Char(' '))),
            Some(Message::Select)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Backspace)),
            Some(Message::Back)
        );
    }

    #[test]
    fn other_action_keys() {
        assert_eq!(
            key_to_message(make_key(KeyCode::Char('r'))),
            Some(Message::Refresh)
        );
        assert_eq!(
            key_to_message(make_key(KeyCode::Char('?'))),
            Some(Message::ToggleHelp)
        );
    }

    #[test]
    fn unmapped_keys_return_none() {
        assert_eq!(key_to_message(make_key(KeyCode::Char('x'))), None);
        assert_eq!(key_to_message(make_key(KeyCode::F(1))), None);
    }
}
