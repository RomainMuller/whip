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
/// | `o` | Open in browser |
/// | `Ctrl+R` | Refresh |
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

    // Check for Ctrl+R to refresh
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
        return Some(Message::Refresh);
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
        KeyCode::Char('o') => Some(Message::OpenInBrowser),
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
/// | `Left` | Previous section (nav mode) / Move cursor left (edit mode) |
/// | `Right` | Next section (nav mode) / Move cursor right (edit mode) |
/// | `Up` | Navigate up |
/// | `Down` | Navigate down |
/// | `Enter` | Edit/confirm |
/// | `Esc` | Cancel/close |
/// | `d` | Delete |
/// | `y`/`n` | Confirm/cancel delete (when delete pending) |
/// | `Backspace` | Backspace (in edit mode) |
/// | Any char | Input (in edit mode) |
#[must_use]
pub fn key_to_settings_message(
    key: KeyEvent,
    is_editing: bool,
    is_delete_pending: bool,
) -> Option<Message> {
    // Check for Ctrl+C first (always works)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    if is_editing {
        // In edit mode, capture text input and cursor movement
        match key.code {
            KeyCode::Enter => Some(Message::SettingsConfirm),
            KeyCode::Esc => Some(Message::SettingsCancel),
            KeyCode::Backspace => Some(Message::SettingsBackspace),
            KeyCode::Tab => Some(Message::SettingsSwitchField),
            KeyCode::Left => Some(Message::SettingsCursorLeft),
            KeyCode::Right => Some(Message::SettingsCursorRight),
            KeyCode::Char(ch) => Some(Message::SettingsInput { ch }),
            _ => None,
        }
    } else if is_delete_pending {
        // Delete confirmation mode - route y/n keys
        match key.code {
            KeyCode::Char(ch @ ('y' | 'Y' | 'n' | 'N')) => Some(Message::SettingsInput { ch }),
            KeyCode::Esc => Some(Message::SettingsCancel),
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
            KeyCode::Enter => Some(Message::SettingsEdit),
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
            key_to_message(make_key(KeyCode::Char('?'))),
            Some(Message::ToggleHelp)
        );
    }

    #[test]
    fn ctrl_r_refreshes() {
        assert_eq!(
            key_to_message(make_key_with_modifiers(
                KeyCode::Char('r'),
                KeyModifiers::CONTROL
            )),
            Some(Message::Refresh)
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
            key_to_settings_message(make_key(KeyCode::Right), false, false),
            Some(Message::SettingsNextSection)
        );

        // Left moves to previous section
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Left), false, false),
            Some(Message::SettingsPrevSection)
        );

        // Up/Down arrow keys navigate within section
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Up), false, false),
            Some(Message::SettingsNavigate { delta: -1 })
        );
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Down), false, false),
            Some(Message::SettingsNavigate { delta: 1 })
        );

        // Enter starts edit
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Enter), false, false),
            Some(Message::SettingsEdit)
        );

        // d deletes
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('d')), false, false),
            Some(Message::SettingsDelete)
        );

        // Esc closes
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Esc), false, false),
            Some(Message::CloseSettings)
        );
    }

    #[test]
    fn settings_edit_mode() {
        // Character input
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('a')), true, false),
            Some(Message::SettingsInput { ch: 'a' })
        );

        // Backspace
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Backspace), true, false),
            Some(Message::SettingsBackspace)
        );

        // Enter confirms
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Enter), true, false),
            Some(Message::SettingsConfirm)
        );

        // Esc cancels
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Esc), true, false),
            Some(Message::SettingsCancel)
        );

        // Left arrow moves cursor left in edit mode
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Left), true, false),
            Some(Message::SettingsCursorLeft)
        );

        // Right arrow moves cursor right in edit mode
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Right), true, false),
            Some(Message::SettingsCursorRight)
        );
    }

    #[test]
    fn settings_delete_pending_mode() {
        // y confirms delete
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('y')), false, true),
            Some(Message::SettingsInput { ch: 'y' })
        );

        // Y also confirms delete
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('Y')), false, true),
            Some(Message::SettingsInput { ch: 'Y' })
        );

        // n cancels delete
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('n')), false, true),
            Some(Message::SettingsInput { ch: 'n' })
        );

        // N also cancels delete
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('N')), false, true),
            Some(Message::SettingsInput { ch: 'N' })
        );

        // Esc cancels delete
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Esc), false, true),
            Some(Message::SettingsCancel)
        );

        // Other keys are ignored in delete pending mode
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Char('x')), false, true),
            None
        );
        assert_eq!(
            key_to_settings_message(make_key(KeyCode::Enter), false, true),
            None
        );
    }

    #[test]
    fn settings_ctrl_c_always_quits() {
        // Ctrl+C works in all modes
        assert_eq!(
            key_to_settings_message(
                make_key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL),
                false,
                false
            ),
            Some(Message::Quit)
        );
        assert_eq!(
            key_to_settings_message(
                make_key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL),
                true,
                false
            ),
            Some(Message::Quit)
        );
        assert_eq!(
            key_to_settings_message(
                make_key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL),
                false,
                true
            ),
            Some(Message::Quit)
        );
    }
}
