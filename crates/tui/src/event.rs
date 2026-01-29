//! Event handling and key mappings.
//!
//! This module provides event polling and conversion from terminal events
//! to application messages.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use whip_protocol::Message;

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

/// Converts an event (keyboard or mouse) to an application message.
///
/// Returns `Some(Message)` if the event maps to an action,
/// or `None` if the event is not handled.
#[must_use]
pub fn event_to_message(event: &Event) -> Option<Message> {
    match event {
        Event::Key(key) => key_to_message(*key),
        Event::Mouse(mouse) => mouse_to_message(mouse),
        _ => None,
    }
}

/// Converts a mouse event to an application message.
///
/// Only left-click press events are handled, producing a `ClickAt` message
/// with the click coordinates.
#[must_use]
fn mouse_to_message(mouse: &crossterm::event::MouseEvent) -> Option<Message> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => Some(Message::ClickAt {
            column: mouse.column,
            row: mouse.row,
        }),
        _ => None,
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
    use crossterm::event::{KeyEventKind, MouseEvent, MouseEventKind};

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

    fn make_mouse_click(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
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

    #[test]
    fn mouse_left_click_generates_click_at() {
        let mouse = make_mouse_click(10, 5);
        assert_eq!(
            mouse_to_message(&mouse),
            Some(Message::ClickAt { column: 10, row: 5 })
        );
    }

    #[test]
    fn mouse_right_click_ignored() {
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(mouse_to_message(&mouse), None);
    }

    #[test]
    fn mouse_release_ignored() {
        let mouse = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(mouse_to_message(&mouse), None);
    }

    #[test]
    fn mouse_move_ignored() {
        let mouse = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(mouse_to_message(&mouse), None);
    }

    #[test]
    fn event_to_message_handles_key_events() {
        let key_event = Event::Key(make_key(KeyCode::Enter));
        assert_eq!(event_to_message(&key_event), Some(Message::Select));
    }

    #[test]
    fn event_to_message_handles_mouse_events() {
        let mouse_event = Event::Mouse(make_mouse_click(15, 8));
        assert_eq!(
            event_to_message(&mouse_event),
            Some(Message::ClickAt { column: 15, row: 8 })
        );
    }

    #[test]
    fn event_to_message_ignores_resize_events() {
        let resize_event = Event::Resize(80, 24);
        assert_eq!(event_to_message(&resize_event), None);
    }
}
