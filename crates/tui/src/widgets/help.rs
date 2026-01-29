//! Help overlay widget.
//!
//! This module provides the help overlay that displays all available keybindings
//! when the user presses `?`.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

/// The width of the help overlay panel.
const HELP_WIDTH: u16 = 35;

/// The height of the help overlay panel.
const HELP_HEIGHT: u16 = 19;

/// Renders a centered help overlay displaying all keybindings.
///
/// The overlay is rendered on top of the existing content, with a semi-transparent
/// background effect achieved by clearing the area first.
///
/// # Arguments
///
/// * `area` - The full terminal area (the overlay will be centered within it)
/// * `buf` - The buffer to render into
///
/// # Layout
///
/// ```text
/// +-- Help ------------------------+
/// |                                |
/// |  Navigation                    |
/// |  ←          Move left          |
/// |  →          Move right         |
/// |  ↑          Select previous    |
/// |  ↓          Select next        |
/// |                                |
/// |  Actions                       |
/// |  Enter      Open details       |
/// |  Esc        Close panel        |
/// |  Ctrl+C     Quit               |
/// |  ?          Toggle help        |
/// |                                |
/// |  Press any key to close        |
/// +---------------------------------+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use whip_tui::widgets::render_help_overlay;
///
/// let area = Rect::new(0, 0, 80, 24);
/// let mut buf = Buffer::empty(area);
///
/// render_help_overlay(area, &mut buf);
/// ```
pub fn render_help_overlay(area: Rect, buf: &mut Buffer) {
    // Calculate centered position
    let popup_area = centered_rect(HELP_WIDTH, HELP_HEIGHT, area);

    // Clear the area behind the popup for a clean look
    Clear.render(popup_area, buf);

    // Build the help content
    let lines = build_help_lines();

    let help_block = Block::default()
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::LightYellow));

    let help_text = Paragraph::new(lines)
        .block(help_block)
        .alignment(Alignment::Left);

    help_text.render(popup_area, buf);
}

/// Builds the lines of help content.
fn build_help_lines() -> Vec<Line<'static>> {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Green);
    let text_style = Style::default().fg(Color::White);
    let hint_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC);

    vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", header_style)),
        Line::from(vec![
            Span::styled("  ←          ", key_style),
            Span::styled("Move left", text_style),
        ]),
        Line::from(vec![
            Span::styled("  →          ", key_style),
            Span::styled("Move right", text_style),
        ]),
        Line::from(vec![
            Span::styled("  ↑          ", key_style),
            Span::styled("Select previous", text_style),
        ]),
        Line::from(vec![
            Span::styled("  ↓          ", key_style),
            Span::styled("Select next", text_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Actions", header_style)),
        Line::from(vec![
            Span::styled("  Enter      ", key_style),
            Span::styled("Open details", text_style),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key_style),
            Span::styled("Close panel", text_style),
        ]),
        Line::from(vec![
            Span::styled("  Shift+S    ", key_style),
            Span::styled("Open settings", text_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C     ", key_style),
            Span::styled("Quit", text_style),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", key_style),
            Span::styled("Toggle help", text_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", hint_style)),
    ]
}

/// Creates a centered rectangle within a given area.
///
/// If the requested dimensions exceed the available area, the rectangle
/// will be clamped to fit.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    // Clamp dimensions to available area
    let popup_width = width.min(area.width);
    let popup_height = height.min(area.height);

    // Calculate centered position
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect::new(x, y, popup_width, popup_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::buffer_to_string;

    #[test]
    fn centered_rect_positions_correctly() {
        let area = Rect::new(0, 0, 80, 24);
        let centered = centered_rect(20, 10, area);

        // Should be centered horizontally
        assert_eq!(centered.x, 30); // (80 - 20) / 2
        assert_eq!(centered.y, 7); // (24 - 10) / 2
        assert_eq!(centered.width, 20);
        assert_eq!(centered.height, 10);
    }

    #[test]
    fn centered_rect_clamps_to_area() {
        let area = Rect::new(0, 0, 40, 12);
        let centered = centered_rect(100, 50, area);

        // Should be clamped to area dimensions
        assert_eq!(centered.width, 40);
        assert_eq!(centered.height, 12);
        assert_eq!(centered.x, 0);
        assert_eq!(centered.y, 0);
    }

    #[test]
    fn render_help_overlay_creates_output() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_help_overlay(area, &mut buf);

        // Verify the help title is rendered
        let content = buffer_to_string(&buf);
        assert!(content.contains("Help"));
        assert!(content.contains("Navigation"));
        assert!(content.contains("Actions"));
    }

    #[test]
    fn render_help_overlay_handles_small_area() {
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);

        // Should not panic with small area
        render_help_overlay(area, &mut buf);
    }

    #[test]
    fn build_help_lines_contains_all_keybindings() {
        let lines = build_help_lines();

        let content: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Check navigation keys (arrow symbols)
        assert!(content.contains("←"));
        assert!(content.contains("→"));
        assert!(content.contains("↑"));
        assert!(content.contains("↓"));

        // Check action keys
        assert!(content.contains("Enter"));
        assert!(content.contains("Esc"));
        assert!(content.contains("Quit"));
        assert!(content.contains("?"));
    }
}
