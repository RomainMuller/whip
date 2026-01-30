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

use crate::settings_state::{EditMode, SettingsSection, SettingsState};

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

    let mut items: Vec<ListItem> = config
        .repositories
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == selected { "> " } else { "  " };
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
            format!("> + {}_", &value[..*cursor])
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
        if let EditMode::Text { value, .. } = state.edit_mode() {
            format!("{}_ seconds", value)
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
        if let EditMode::Text { value, .. } = state.edit_mode() {
            if value.is_empty() {
                "(editing...)".to_string()
            } else {
                // Mask the token for security
                format!("{}...", &"*".repeat(value.len().min(20)))
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
        "Enter: confirm | Esc: cancel"
    } else {
        "←→: sections | ↑↓: navigate | Enter: edit | d: delete | Esc: close"
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

    #[test]
    fn render_settings_panel_creates_output() {
        let config = Config::default();
        let state = SettingsState::new(config);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_settings_panel(&state, area, &mut buf);

        // Verify the settings title is rendered
        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
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

        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
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
}
