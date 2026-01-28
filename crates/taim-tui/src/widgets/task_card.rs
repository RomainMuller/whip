//! Task card rendering widget.
//!
//! This module provides functions for rendering individual task cards with
//! color coding based on their execution state.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use taim_protocol::{Task, TaskState};

/// Returns the color associated with a task state.
///
/// This provides consistent color coding across the application:
///
/// - `Idle`: Gray - task is waiting, no activity
/// - `InFlight`: Blue - actively being worked on
/// - `NeedsAttention`: Yellow - blocked or needs input
/// - `Success`: Green - completed successfully
/// - `Failed`: Red - failed or errored
///
/// # Examples
///
/// ```
/// use taim_protocol::TaskState;
/// use taim_tui::widgets::state_color;
/// use ratatui::style::Color;
///
/// assert_eq!(state_color(TaskState::Idle), Color::DarkGray);
/// assert_eq!(state_color(TaskState::InFlight), Color::Blue);
/// assert_eq!(state_color(TaskState::NeedsAttention), Color::Yellow);
/// assert_eq!(state_color(TaskState::Success), Color::Green);
/// assert_eq!(state_color(TaskState::Failed), Color::Red);
/// ```
#[must_use]
pub const fn state_color(state: TaskState) -> Color {
    match state {
        TaskState::Idle => Color::DarkGray,
        TaskState::InFlight => Color::Blue,
        TaskState::NeedsAttention => Color::Yellow,
        TaskState::Success => Color::Green,
        TaskState::Failed => Color::Red,
    }
}

/// Returns a brighter version of the state color for selected items.
///
/// Used to highlight selected task cards with more vivid colors.
#[must_use]
const fn state_color_bright(state: TaskState) -> Color {
    match state {
        TaskState::Idle => Color::Gray,
        TaskState::InFlight => Color::LightBlue,
        TaskState::NeedsAttention => Color::LightYellow,
        TaskState::Success => Color::LightGreen,
        TaskState::Failed => Color::LightRed,
    }
}

/// Renders a task card to the buffer.
///
/// The card displays the task title and a truncated description within a bordered
/// box. The border color reflects the task's execution state, with brighter colors
/// used for selected cards.
///
/// # Arguments
///
/// * `task` - The task to render
/// * `is_selected` - Whether this card is currently selected
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Layout
///
/// The card uses a bordered paragraph with the following structure:
///
/// ```text
/// +----------------+
/// | Title          |
/// | description... |
/// +----------------+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_protocol::Task;
/// use taim_tui::widgets::render_task_card;
///
/// let task = Task::new("Implement feature", "Add new functionality");
/// let area = Rect::new(0, 0, 20, 5);
/// let mut buf = Buffer::empty(area);
///
/// render_task_card(&task, false, area, &mut buf);
/// ```
pub fn render_task_card(task: &Task, is_selected: bool, area: Rect, buf: &mut Buffer) {
    // Skip rendering if area is too small
    if area.width < 4 || area.height < 3 {
        return;
    }

    let base_color = state_color(task.state);
    let (border_color, title_style, desc_style) = if is_selected {
        (
            state_color_bright(task.state),
            Style::default()
                .fg(state_color_bright(task.state))
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::White),
        )
    } else {
        (
            base_color,
            Style::default().fg(Color::White),
            Style::default().fg(Color::DarkGray),
        )
    };

    // Truncate description to fit available space
    let inner_width = area.width.saturating_sub(2) as usize;
    let truncated_desc = truncate_string(&task.description, inner_width);

    let content = vec![
        Line::from(Span::styled(&task.title, title_style)),
        Line::from(Span::styled(truncated_desc, desc_style)),
    ];

    let card = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    card.render(area, buf);
}

/// Truncates a string to fit within a given width, adding ellipsis if needed.
fn truncate_string(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        let truncated: String = s.chars().take(max_width - 3).collect();
        format!("{truncated}...")
    } else {
        s.chars().take(max_width).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_color_mapping() {
        assert_eq!(state_color(TaskState::Idle), Color::DarkGray);
        assert_eq!(state_color(TaskState::InFlight), Color::Blue);
        assert_eq!(state_color(TaskState::NeedsAttention), Color::Yellow);
        assert_eq!(state_color(TaskState::Success), Color::Green);
        assert_eq!(state_color(TaskState::Failed), Color::Red);
    }

    #[test]
    fn truncate_string_short() {
        let result = truncate_string("Hello", 10);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn truncate_string_exact() {
        let result = truncate_string("Hello", 5);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn truncate_string_long() {
        let result = truncate_string("Hello, World!", 10);
        assert_eq!(result, "Hello, ...");
    }

    #[test]
    fn truncate_string_very_short_max() {
        let result = truncate_string("Hello", 3);
        assert_eq!(result, "Hel");
    }

    #[test]
    fn render_task_card_creates_output() {
        let task = Task::new("Test Task", "A description");
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);

        render_task_card(&task, false, area, &mut buf);

        // Verify something was rendered (borders at minimum)
        let cell = buf.cell((0, 0)).expect("cell should exist");
        assert_ne!(cell.symbol(), " ");
    }

    #[test]
    fn render_task_card_handles_small_area() {
        let task = Task::new("Test Task", "A description");
        let area = Rect::new(0, 0, 2, 2);
        let mut buf = Buffer::empty(area);

        // Should not panic with tiny area
        render_task_card(&task, false, area, &mut buf);
    }
}
