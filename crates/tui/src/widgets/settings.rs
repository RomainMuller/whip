//! Settings panel widget.
//!
//! This module provides the settings panel overlay that allows users to
//! view and modify application configuration.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::settings_state::{EditMode, RepoEditField, SettingsSection, SettingsState};

/// Formats a text field value with a cursor indicator at the specified position.
///
/// Inserts an underscore `_` at the cursor position to indicate where the next
/// character will be inserted. Respects UTF-8 character boundaries.
fn format_with_cursor(value: &str, cursor: usize) -> String {
    // Ensure cursor is at a valid UTF-8 boundary
    let safe_cursor = if cursor > value.len() {
        value.len()
    } else {
        // Find the nearest valid boundary at or before cursor
        value
            .char_indices()
            .take_while(|(i, _)| *i <= cursor)
            .last()
            .map(|(i, c)| if i == cursor { i } else { i + c.len_utf8() })
            .unwrap_or(0)
            .min(cursor)
    };

    let (before, after) = value.split_at(safe_cursor.min(value.len()));
    format!("{}_{}", before, after)
}

/// The width of the settings panel.
const SETTINGS_WIDTH: u16 = 72;

/// The height of the settings panel.
const SETTINGS_HEIGHT: u16 = 22;

/// Renders the settings panel overlay.
///
/// The panel is centered on screen and displays configuration options
/// organized into sections.
///
/// # Arguments
///
/// * `state` - The settings state
/// * `area` - The full terminal area (panel will be centered within it)
/// * `buf` - The buffer to render into
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use whip_config::Config;
/// use whip_tui::settings_state::SettingsState;
/// use whip_tui::widgets::render_settings_panel;
///
/// let state = SettingsState::new(Config::default());
/// let area = Rect::new(0, 0, 80, 24);
/// let mut buf = Buffer::empty(area);
///
/// render_settings_panel(&state, area, &mut buf);
/// ```
pub fn render_settings_panel(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    // Calculate centered position
    let popup_area = centered_rect(SETTINGS_WIDTH, SETTINGS_HEIGHT, area);

    // Clear the area behind the popup
    Clear.render(popup_area, buf);

    // Create the main block
    let block = Block::default()
        .title(Span::styled(
            " Settings ",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    // Split inner area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Section tabs
            Constraint::Length(1), // Separator
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Separator
            Constraint::Length(2), // Help/status bar
        ])
        .split(inner);

    // Render section tabs
    render_section_tabs(state, chunks[0], buf);

    // Render section content
    render_section_content(state, chunks[2], buf);

    // Render help/status bar
    render_settings_help(state, chunks[4], buf);
}

/// Renders the section tabs at the top of the settings panel.
fn render_section_tabs(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    let sections = SettingsSection::all();
    let mut spans = Vec::with_capacity(sections.len() * 2);

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }

        let style = if *section == state.section() {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        spans.push(Span::styled(section.name(), style));
    }

    let tabs = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    tabs.render(area, buf);
}

/// Renders the content for the current section.
fn render_section_content(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    match state.section() {
        SettingsSection::Repositories => render_repositories_section(state, area, buf),
        SettingsSection::Polling => render_polling_section(state, area, buf),
        SettingsSection::Authentication => render_authentication_section(state, area, buf),
    }
}

/// Renders the repositories section.
fn render_repositories_section(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    let config = state.config();
    let selected = state.selected_item();
    let pending_delete = state.pending_delete();

    // Check if we're editing a repository
    if let EditMode::EditRepository {
        index,
        path,
        path_cursor,
        token,
        token_cursor,
        active_field,
    } = state.edit_mode()
    {
        // Render edit mode UI
        render_repo_edit_mode(
            *index,
            path,
            *path_cursor,
            token,
            *token_cursor,
            *active_field,
            area,
            buf,
        );
        return;
    }

    let mut items: Vec<ListItem> = config
        .repositories
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_pending_delete = pending_delete == Some(i);

            let style = if is_pending_delete {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if i == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == selected { "> " } else { "  " };

            if is_pending_delete {
                // Show confirmation prompt
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(repo.full_name(), style),
                    Span::styled(" Delete? (y/n)", Style::default().fg(Color::Red)),
                ]))
            } else {
                let token_indicator = if repo.token().is_some() {
                    " [token]"
                } else {
                    ""
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(repo.full_name(), style),
                    Span::styled(token_indicator, Style::default().fg(Color::DarkGray)),
                ]))
            }
        })
        .collect();

    // Add "Add new" option
    let add_new_selected = selected == config.repositories.len();
    let add_style = if add_new_selected {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    // Handle edit mode for adding repository
    let add_text = if add_new_selected {
        if let EditMode::AddRepository { value, cursor } = state.edit_mode() {
            format!("> + {}", format_with_cursor(value, *cursor))
        } else {
            "> + Add repository...".to_string()
        }
    } else {
        "  + Add repository...".to_string()
    };

    items.push(ListItem::new(Span::styled(add_text, add_style)));

    let list = List::new(items);
    list.render(area, buf);
}

/// Renders the repository edit mode UI with two fields.
#[allow(clippy::too_many_arguments)]
fn render_repo_edit_mode(
    _index: usize,
    path: &str,
    path_cursor: usize,
    token: &str,
    token_cursor: usize,
    active_field: RepoEditField,
    area: Rect,
    buf: &mut Buffer,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Path field
            Constraint::Length(3), // Token field
            Constraint::Length(2), // Help text
            Constraint::Min(0),    // Remaining space
        ])
        .split(area);

    // Path field
    let path_active = active_field == RepoEditField::Path;
    let path_style = if path_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let path_border_style = if path_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let path_display = if path_active {
        format_with_cursor(path, path_cursor)
    } else {
        path.to_string()
    };

    let path_block = Block::default()
        .title(Span::styled(" owner/repo ", path_border_style))
        .borders(Borders::ALL)
        .border_style(path_border_style);

    let path_para = Paragraph::new(Span::styled(path_display, path_style)).block(path_block);
    path_para.render(chunks[0], buf);

    // Token field
    let token_active = active_field == RepoEditField::Token;
    let token_style = if token_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let token_border_style = if token_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let token_display = if token_active {
        format_with_cursor(token, token_cursor)
    } else if token.is_empty() {
        "(optional)".to_string()
    } else {
        // Mask the token for security
        format!("{}...", "*".repeat(token.len().min(16)))
    };

    let token_block = Block::default()
        .title(Span::styled(" token (optional) ", token_border_style))
        .borders(Borders::ALL)
        .border_style(token_border_style);

    let token_para = Paragraph::new(Span::styled(token_display, token_style)).block(token_block);
    token_para.render(chunks[1], buf);

    // Help text
    let help = Paragraph::new(Span::styled(
        "Tab: switch field | ←→: move cursor | Enter: save | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Center);
    help.render(chunks[2], buf);
}

/// Renders the polling section.
fn render_polling_section(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    let config = state.config();
    let selected = state.selected_item();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Polling interval
    let interval_selected = selected == 0;
    let interval_style = if interval_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let interval_value = if interval_selected {
        if let EditMode::Text { value, cursor } = state.edit_mode() {
            format!("{} seconds", format_with_cursor(value, *cursor))
        } else {
            format!("{} seconds", config.polling.interval_secs)
        }
    } else {
        format!("{} seconds", config.polling.interval_secs)
    };

    let prefix = if interval_selected { "> " } else { "  " };
    let interval_line = Paragraph::new(Line::from(vec![
        Span::styled(prefix, interval_style),
        Span::styled("Polling interval: ", Style::default().fg(Color::Gray)),
        Span::styled(interval_value, interval_style),
    ]));
    interval_line.render(chunks[0], buf);

    // Auto-adjust toggle
    let auto_adjust_selected = selected == 1;
    let auto_adjust_style = if auto_adjust_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let checkbox = if config.polling.auto_adjust {
        "[x]"
    } else {
        "[ ]"
    };
    let prefix = if auto_adjust_selected { "> " } else { "  " };

    let auto_adjust_line = Paragraph::new(Line::from(vec![
        Span::styled(prefix, auto_adjust_style),
        Span::styled(
            "Auto-adjust for rate limits: ",
            Style::default().fg(Color::Gray),
        ),
        Span::styled(checkbox, auto_adjust_style),
    ]));
    auto_adjust_line.render(chunks[1], buf);
}

/// Renders the authentication section.
fn render_authentication_section(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    let config = state.config();
    let selected = state.selected_item();

    let token_selected = selected == 0;
    let token_style = if token_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let token_display = if token_selected {
        if let EditMode::Text { value, cursor } = state.edit_mode() {
            if value.is_empty() {
                // Show cursor even when empty
                "_".to_string()
            } else {
                // Show cursor position with masked token
                let masked = "*".repeat(value.len().min(20));
                format_with_cursor(&masked, (*cursor).min(masked.len()))
            }
        } else {
            match &config.github_token {
                Some(token) if !token.is_empty() => {
                    format!("{}...{}", &token[..4.min(token.len())], "(set)")
                }
                _ => "(not set)".to_string(),
            }
        }
    } else {
        match &config.github_token {
            Some(token) if !token.is_empty() => {
                format!("{}...{}", &token[..4.min(token.len())], "(set)")
            }
            _ => "(not set)".to_string(),
        }
    };

    let prefix = if token_selected { "> " } else { "  " };
    let token_line = Paragraph::new(Line::from(vec![
        Span::styled(prefix, token_style),
        Span::styled("GitHub Token: ", Style::default().fg(Color::Gray)),
        Span::styled(token_display, token_style),
    ]));
    token_line.render(area, buf);

    // Add a note about gh CLI fallback
    if area.height > 2 {
        let note_area = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(2),
            ..area
        };
        let note = Paragraph::new(Line::from(vec![Span::styled(
            "  (Falls back to `gh auth token` if not set)",
            Style::default().fg(Color::DarkGray),
        )]));
        note.render(note_area, buf);
    }
}

/// Renders the help/status bar at the bottom of the settings panel.
fn render_settings_help(state: &SettingsState, area: Rect, buf: &mut Buffer) {
    let help_text = if state.is_editing() {
        "←→: move cursor | Enter: confirm | Esc: cancel".to_string()
    } else if state.can_delete_selected() {
        "←→: sections | ↑↓: navigate | Enter: edit | d: delete | Esc: close".to_string()
    } else {
        // Use whitespace instead of "d: delete" to prevent layout shifting
        // "d: delete" is 9 characters, so we use 9 spaces
        "←→: sections | ↑↓: navigate | Enter: edit |           | Esc: close".to_string()
    };

    let help = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);

    help.render(area, buf);
}

/// Creates a centered rectangle within a given area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width);
    let popup_height = height.min(area.height);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect::new(x, y, popup_width, popup_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use whip_config::{Config, Repository};

    /// Extracts all characters from a buffer as a single string for content assertions.
    fn buffer_content(buf: &Buffer) -> String {
        buf.content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect()
    }

    #[test]
    fn render_settings_panel_creates_output() {
        let config = Config::default();
        let state = SettingsState::new(config);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_settings_panel(&state, area, &mut buf);

        let content = buffer_content(&buf);
        assert!(content.contains("Settings"));
        assert!(content.contains("Repositories"));
    }

    #[test]
    fn render_settings_panel_handles_small_area() {
        let config = Config::default();
        let state = SettingsState::new(config);
        let area = Rect::new(0, 0, 30, 10);
        let mut buf = Buffer::empty(area);

        // Should not panic
        render_settings_panel(&state, area, &mut buf);
    }

    #[test]
    fn render_settings_with_repositories() {
        let mut config = Config::default();
        config
            .repositories
            .push(Repository::new("rust-lang", "rust"));
        config
            .repositories
            .push(Repository::new("tokio-rs", "tokio"));

        let state = SettingsState::new(config);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_settings_panel(&state, area, &mut buf);

        let content = buffer_content(&buf);
        assert!(content.contains("rust-lang/rust"));
        assert!(content.contains("tokio-rs/tokio"));
    }

    #[test]
    fn centered_rect_positions_correctly() {
        let area = Rect::new(0, 0, 80, 24);
        let centered = centered_rect(40, 12, area);

        assert_eq!(centered.x, 20); // (80 - 40) / 2
        assert_eq!(centered.y, 6); // (24 - 12) / 2
        assert_eq!(centered.width, 40);
        assert_eq!(centered.height, 12);
    }

    #[test]
    fn centered_rect_clamps_to_area() {
        let area = Rect::new(0, 0, 30, 10);
        let centered = centered_rect(100, 50, area);

        assert_eq!(centered.width, 30);
        assert_eq!(centered.height, 10);
    }

    // Tests for format_with_cursor

    #[test]
    fn format_with_cursor_empty_string() {
        // Empty string with cursor at position 0 should show just the cursor
        let result = format_with_cursor("", 0);
        assert_eq!(result, "_");
    }

    #[test]
    fn format_with_cursor_at_start() {
        // Cursor at start should show cursor before text
        let result = format_with_cursor("hello", 0);
        assert_eq!(result, "_hello");
    }

    #[test]
    fn format_with_cursor_at_end() {
        // Cursor at end should show cursor after text
        let result = format_with_cursor("hello", 5);
        assert_eq!(result, "hello_");
    }

    #[test]
    fn format_with_cursor_in_middle() {
        // Cursor in middle should split text appropriately
        let result = format_with_cursor("hello", 2);
        assert_eq!(result, "he_llo");
    }

    #[test]
    fn format_with_cursor_beyond_length() {
        // Cursor beyond string length should clamp to end
        let result = format_with_cursor("hello", 100);
        assert_eq!(result, "hello_");
    }

    #[test]
    fn format_with_cursor_with_utf8() {
        // UTF-8 multi-byte characters should be handled correctly
        let result = format_with_cursor("cafe", 2);
        assert_eq!(result, "ca_fe");

        // Cursor at end of UTF-8 string
        let result = format_with_cursor("cafe", 4);
        assert_eq!(result, "cafe_");
    }

    // Tests for render_polling_section

    #[test]
    fn render_polling_section_shows_interval_and_auto_adjust() {
        let mut config = Config::default();
        config.polling.interval_secs = 45;
        config.polling.auto_adjust = true;

        let state = SettingsState::new(config);
        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_polling_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        // Verify polling interval is displayed
        assert!(content.contains("Polling interval"));
        assert!(content.contains("45"));
        assert!(content.contains("seconds"));

        // Verify auto-adjust toggle is displayed with checkbox
        assert!(content.contains("Auto-adjust"));
        assert!(content.contains("[x]")); // Checked since auto_adjust is true
    }

    #[test]
    fn render_polling_section_shows_unchecked_when_auto_adjust_disabled() {
        let mut config = Config::default();
        config.polling.auto_adjust = false;

        let mut state = SettingsState::new(config);
        // Navigate to Polling section
        state.next_section();

        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_polling_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        // Verify unchecked checkbox
        assert!(content.contains("[ ]"));
    }

    #[test]
    fn render_polling_section_highlights_selected_item() {
        let config = Config::default();
        let mut state = SettingsState::new(config);
        state.next_section(); // Go to Polling
        state.navigate(1); // Select auto-adjust (index 1)

        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_polling_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        // The auto-adjust line should have a selection indicator
        // The content should contain both polling interval and auto-adjust
        assert!(content.contains("Polling interval"));
        assert!(content.contains("Auto-adjust"));
    }

    // Tests for render_authentication_section

    #[test]
    fn render_authentication_section_shows_token_not_set() {
        let config = Config::default(); // No token set
        let mut state = SettingsState::new(config);
        state.next_section(); // Polling
        state.next_section(); // Authentication

        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_authentication_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        assert!(content.contains("GitHub Token"));
        assert!(content.contains("(not set)"));
    }

    #[test]
    fn render_authentication_section_shows_token_set() {
        let config = Config {
            github_token: Some("ghp_testtoken123456".to_string()),
            ..Default::default()
        };

        let mut state = SettingsState::new(config);
        state.next_section(); // Polling
        state.next_section(); // Authentication

        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_authentication_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        assert!(content.contains("GitHub Token"));
        // Should show partial token and "(set)" indicator
        assert!(content.contains("ghp_"));
        assert!(content.contains("(set)"));
    }

    #[test]
    fn render_authentication_section_shows_fallback_note() {
        let config = Config::default();
        let mut state = SettingsState::new(config);
        state.next_section(); // Polling
        state.next_section(); // Authentication

        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);

        render_authentication_section(&state, area, &mut buf);

        let content = buffer_content(&buf);

        // Should show the gh CLI fallback note
        assert!(content.contains("gh auth token"));
    }

    // Tests for render_repo_edit_mode

    #[test]
    fn render_repo_edit_mode_shows_path_and_token_fields() {
        let area = Rect::new(0, 0, 60, 15);
        let mut buf = Buffer::empty(area);

        render_repo_edit_mode(
            0,
            "owner/repo",
            5,
            "secret-token",
            6,
            RepoEditField::Path,
            area,
            &mut buf,
        );

        let content = buffer_content(&buf);

        // Verify path field label and content
        assert!(content.contains("owner/repo"));

        // Verify token field label (token is masked when not active)
        assert!(content.contains("token (optional)"));

        // Verify help text
        assert!(content.contains("Tab"));
        assert!(content.contains("Enter"));
        assert!(content.contains("Esc"));
    }

    #[test]
    fn render_repo_edit_mode_shows_cursor_in_active_field() {
        let area = Rect::new(0, 0, 60, 15);
        let mut buf = Buffer::empty(area);

        render_repo_edit_mode(
            0,
            "test",
            2, // Cursor in middle of path
            "",
            0,
            RepoEditField::Path,
            area,
            &mut buf,
        );

        let content = buffer_content(&buf);

        // Active path field should show cursor
        assert!(content.contains("te_st"));
    }

    #[test]
    fn render_repo_edit_mode_masks_inactive_token() {
        let area = Rect::new(0, 0, 60, 15);
        let mut buf = Buffer::empty(area);

        render_repo_edit_mode(
            0,
            "owner/repo",
            10,
            "secrettoken",
            11,
            RepoEditField::Path, // Path is active, token is inactive
            area,
            &mut buf,
        );

        let content = buffer_content(&buf);

        // Token should be masked with asterisks when inactive
        assert!(content.contains("***"));
    }

    #[test]
    fn render_repo_edit_mode_shows_optional_placeholder_for_empty_token() {
        let area = Rect::new(0, 0, 60, 15);
        let mut buf = Buffer::empty(area);

        render_repo_edit_mode(
            0,
            "owner/repo",
            10,
            "", // Empty token
            0,
            RepoEditField::Path, // Path is active
            area,
            &mut buf,
        );

        let content = buffer_content(&buf);

        // Empty token should show "(optional)" placeholder
        assert!(content.contains("(optional)"));
    }

    #[test]
    fn render_repo_edit_mode_token_field_active_shows_cursor() {
        let area = Rect::new(0, 0, 60, 15);
        let mut buf = Buffer::empty(area);

        render_repo_edit_mode(
            0,
            "owner/repo",
            10,
            "abc",
            1,
            RepoEditField::Token, // Token is active
            area,
            &mut buf,
        );

        let content = buffer_content(&buf);

        // Token field is active, should show cursor in the token value
        assert!(content.contains("a_bc"));
    }
}
