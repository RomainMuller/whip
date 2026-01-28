//! Status bar rendering widget.
//!
//! This module provides functions for rendering the footer status bar
//! with keybinding hints.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Renders the status bar with keybinding hints.
///
/// The status bar displays available keyboard shortcuts to help users
/// navigate and interact with the application.
///
/// # Arguments
///
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Layout
///
/// ```text
/// +----------------------------------------------------+
/// | [Ctrl+C] Quit  [←→↑↓] Navigate  [Enter] Select     |
/// +----------------------------------------------------+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_tui::widgets::render_status_bar;
///
/// let area = Rect::new(0, 0, 80, 3);
/// let mut buf = Buffer::empty(area);
///
/// render_status_bar(area, &mut buf);
/// ```
pub fn render_status_bar(area: Rect, buf: &mut Buffer) {
    let key_style = Style::default().fg(Color::Yellow);
    let text_style = Style::default().fg(Color::White);

    let hints = Line::from(vec![
        Span::styled("Ctrl+C", key_style),
        Span::styled(" Quit  ", text_style),
        Span::styled("←→↑↓", key_style),
        Span::styled(" Navigate  ", text_style),
        Span::styled("Enter", key_style),
        Span::styled(" Select", text_style),
    ]);

    let status_bar = Paragraph::new(hints).block(Block::default().borders(Borders::ALL));

    status_bar.render(area, buf);
}

/// Renders a contextual status bar with custom message.
///
/// This variant allows displaying a custom status message alongside
/// the standard keybinding hints.
///
/// # Arguments
///
/// * `message` - Custom status message to display
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_tui::widgets::status_bar::render_status_bar_with_message;
///
/// let area = Rect::new(0, 0, 80, 3);
/// let mut buf = Buffer::empty(area);
///
/// render_status_bar_with_message("Task moved to In Progress", area, &mut buf);
/// ```
pub fn render_status_bar_with_message(message: &str, area: Rect, buf: &mut Buffer) {
    let key_style = Style::default().fg(Color::Yellow);
    let text_style = Style::default().fg(Color::White);
    let message_style = Style::default().fg(Color::Cyan);

    let hints = Line::from(vec![
        Span::styled(message, message_style),
        Span::styled("  |  ", text_style),
        Span::styled("Ctrl+C", key_style),
        Span::styled(" Quit  ", text_style),
        Span::styled("?", key_style),
        Span::styled(" Help", text_style),
    ]);

    let status_bar = Paragraph::new(hints).block(Block::default().borders(Borders::ALL));

    status_bar.render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_status_bar_contains_hints() {
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);

        render_status_bar(area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("Quit"));
        assert!(content.contains("Navigate"));
    }

    #[test]
    fn render_status_bar_with_message_shows_message() {
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);

        render_status_bar_with_message("Test message", area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("Test message"));
    }

    /// Helper to convert buffer to string for testing.
    fn buffer_to_string(buf: &Buffer) -> String {
        let mut result = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    result.push_str(cell.symbol());
                }
            }
            result.push('\n');
        }
        result
    }
}
