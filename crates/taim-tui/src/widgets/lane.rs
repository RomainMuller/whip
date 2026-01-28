//! Lane rendering widget.
//!
//! This module provides functions for rendering individual Kanban lanes
//! with their headers and task lists.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use taim_protocol::Lane;

use super::task_card::render_task_card;

/// Height of each task card in rows.
const TASK_CARD_HEIGHT: u16 = 4;

/// Renders a single lane to the buffer.
///
/// A lane displays its header (name and task count) followed by a vertical
/// list of task cards. Empty lanes show a "No tasks" placeholder message.
///
/// # Arguments
///
/// * `lane` - The lane to render
/// * `is_focused` - Whether this lane currently has focus
/// * `selected_idx` - Index of the selected task within this lane, if any
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Layout
///
/// ```text
/// +----------------+
/// | Backlog (3)    |  <- Header with name and count
/// +----------------+
/// | +------------+ |
/// | | Task 1     | |  <- Task cards
/// | | desc...    | |
/// | +------------+ |
/// | +------------+ |
/// | | Task 2     | |
/// | | desc...    | |
/// | +------------+ |
/// +----------------+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_protocol::{Lane, LaneKind, Task};
/// use taim_tui::widgets::render_lane;
///
/// let mut lane = Lane::new(LaneKind::Backlog);
/// lane.add_task(Task::new("Task 1", "Description"));
///
/// let area = Rect::new(0, 0, 20, 15);
/// let mut buf = Buffer::empty(area);
///
/// render_lane(&lane, true, Some(0), area, &mut buf);
/// ```
pub fn render_lane(
    lane: &Lane,
    is_focused: bool,
    selected_idx: Option<usize>,
    area: Rect,
    buf: &mut Buffer,
) {
    // Determine border style based on focus
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Create the lane header
    let title = format!("{} ({})", lane.kind.display_name(), lane.len());
    let title_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL)
        .border_style(border_style);

    // Render the outer block
    let inner_area = block.inner(area);
    block.render(area, buf);

    // Handle empty lanes
    if lane.is_empty() {
        render_empty_placeholder(inner_area, buf);
        return;
    }

    // Calculate how many tasks can fit in the visible area
    let visible_tasks = (inner_area.height / TASK_CARD_HEIGHT).max(1) as usize;

    // Determine scroll offset to keep selected task visible
    let scroll_offset = calculate_scroll_offset(selected_idx, lane.len(), visible_tasks);

    // Create constraints for visible task cards
    let task_count = lane.len().min(visible_tasks);
    let mut constraints: Vec<Constraint> = (0..task_count)
        .map(|_| Constraint::Length(TASK_CARD_HEIGHT))
        .collect();
    constraints.push(Constraint::Min(0)); // Fill remaining space

    let task_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    // Render visible task cards
    for (i, task_area) in task_areas.iter().take(task_count).enumerate() {
        let task_idx = scroll_offset + i;
        if task_idx >= lane.tasks.len() {
            break;
        }

        let task = &lane.tasks[task_idx];
        let is_selected = is_focused && selected_idx == Some(task_idx);

        render_task_card(task, is_selected, *task_area, buf);
    }
}

/// Renders a placeholder message for empty lanes.
fn render_empty_placeholder(area: Rect, buf: &mut Buffer) {
    let placeholder = Paragraph::new(Line::from(Span::styled(
        "No tasks",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));

    placeholder.render(area, buf);
}

/// Calculates the scroll offset to keep the selected task visible.
fn calculate_scroll_offset(
    selected_idx: Option<usize>,
    total_tasks: usize,
    visible_tasks: usize,
) -> usize {
    let Some(selected) = selected_idx else {
        return 0;
    };

    if total_tasks <= visible_tasks {
        return 0;
    }

    // Ensure selected task is visible
    let max_offset = total_tasks.saturating_sub(visible_tasks);

    if selected < visible_tasks / 2 {
        0
    } else {
        (selected.saturating_sub(visible_tasks / 2)).min(max_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taim_protocol::{LaneKind, Task};

    #[test]
    fn render_empty_lane() {
        let lane = Lane::new(LaneKind::Backlog);
        let area = Rect::new(0, 0, 20, 15);
        let mut buf = Buffer::empty(area);

        render_lane(&lane, false, None, area, &mut buf);

        // Convert buffer to string and check for placeholder
        let content = buffer_to_string(&buf);
        assert!(content.contains("No tasks"));
    }

    #[test]
    fn render_lane_with_tasks() {
        let mut lane = Lane::new(LaneKind::InProgress);
        lane.add_task(Task::new("Task 1", "Description 1"));
        lane.add_task(Task::new("Task 2", "Description 2"));

        let area = Rect::new(0, 0, 25, 15);
        let mut buf = Buffer::empty(area);

        render_lane(&lane, true, Some(0), area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("In Progress"));
        assert!(content.contains("(2)"));
    }

    #[test]
    fn scroll_offset_no_selection() {
        assert_eq!(calculate_scroll_offset(None, 10, 3), 0);
    }

    #[test]
    fn scroll_offset_all_visible() {
        assert_eq!(calculate_scroll_offset(Some(2), 3, 5), 0);
    }

    #[test]
    fn scroll_offset_selection_at_start() {
        assert_eq!(calculate_scroll_offset(Some(0), 10, 3), 0);
    }

    #[test]
    fn scroll_offset_selection_in_middle() {
        let offset = calculate_scroll_offset(Some(5), 10, 3);
        assert!(offset > 0);
        assert!(offset <= 7);
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
