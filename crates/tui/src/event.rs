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
/// | `Shift+S` | Open settings |
#[must_use]
pub fn key_to_message(key: KeyEvent) -> Option<Message> {
    // Check for Ctrl+C first
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    // Check for Shift+S to open settings
    if key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::Char('S') {
        return Some(Message::OpenSettings);
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

/// Converts a key event to a settings-specific message.
///
/// This function is used when the settings panel is open to handle
/// settings-specific key bindings.
///
/// # Key Bindings (Settings Mode)
///
/// | Key | Action |
/// |-----|--------|
/// | `Left` | Previous section |
/// | `Right` | Next section |
/// | `Up` | Navigate up |
/// | `Down` | Navigate down |
/// | `Enter` | Edit/confirm |
/// | `Esc` | Cancel/close |
/// | `d` | Delete |
/// | `Backspace` | Backspace (in edit mode) |
/// | Any char | Input (in edit mode) |
#[must_use]
pub fn key_to_settings_message(key: KeyEvent, is_editing: bool) -> Option<Message> {
    // Check for Ctrl+C first (always works)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    if is_editing {
        // In edit mode, capture text input
        match key.code {
            KeyCode::Enter => Some(Message::SettingsConfirm),
            KeyCode::Esc => Some(Message::SettingsCancel),
            KeyCode::Backspace => Some(Message::SettingsBackspace),
            KeyCode::Char(ch) => Some(Message::SettingsInput { ch }),
            _ => None,
        }
    } else {
        // Navigation mode
        match key.code {
            KeyCode::Esc => Some(Message::CloseSettings),
            KeyCode::Left => Some(Message::SettingsPrevSection),
            KeyCode::Right => Some(Message::SettingsNextSection),
            KeyCode::Up => Some(Message::SettingsNavigate { delta: -1 }),
            KeyCode::Down => Some(Message::SettingsNavigate { delta: 1 }),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Message::SettingsEdit),
            KeyCode::Char('d') => Some(Message::SettingsDelete),
            _ => None,
        }
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
    fn open_settings_key() {
        // Shift+S opens settings
        assert_eq!(
            key_to_message(make_key_with_modifiers(
                KeyCode::Char('S'),
                KeyModifiers::SHIFT
            )),
            Some(Message::OpenSettings)
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

    // Settings mode tests
    #[test]
    fn settings_navigation_mode() {
        // Right moves to next section
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Right), false),
            Some(Message::SettingsNextSection)
        );

        // Left moves to previous section
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Left), false),
            Some(Message::SettingsPrevSection)
        );

        // Up/Down arrow keys navigate within section
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Up), false),
            Some(Message::SettingsNavigate { delta: -1 })
        );
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Down), false),
            Some(Message::SettingsNavigate { delta: 1 })
        );

        // Enter starts edit
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Enter), false),
            Some(Message::SettingsEdit)
        );

        // d deletes
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('d')), false),
            Some(Message::SettingsDelete)
        );

        // Esc closes
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Esc), false),
            Some(Message::CloseSettings)
        );
    }

    #[test]
    fn settings_edit_mode() {
        // Character input
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('a')), true),
            Some(Message::SettingsInput { ch: 'a' })
        );

        // Backspace
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Backspace), true),
            Some(Message::SettingsBackspace)
        );

        // Enter confirms
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Enter), true),
            Some(Message::SettingsConfirm)
        );

        // Esc cancels
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Esc), true),
            Some(Message::SettingsCancel)
        );
    }

    #[test]
    fn settings_ctrl_c_always_quits() {
        // Ctrl+C works in both modes
        assert_eq!(
            key_to_settings_message(
                make_key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL),
                false
            ),
            Some(Message::Quit)
        );
        assert_eq!(
            key_to_settings_message(
                make_key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL),
                true
            ),
            Some(Message::Quit)
        );
    }
}
