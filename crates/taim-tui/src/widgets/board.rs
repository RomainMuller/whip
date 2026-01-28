//! Kanban board rendering widget.
//!
//! This module provides functions for rendering the complete Kanban board
//! with its four lanes arranged horizontally.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use taim_protocol::{KanbanBoard, LaneKind};

use super::lane::{render_lane, LanePosition};

/// Renders the complete Kanban board to the buffer.
///
/// The board displays four lanes (Backlog, In Progress, Under Review, Done)
/// arranged horizontally with equal widths. Each lane shows its tasks with
/// the selected lane and task highlighted.
///
/// # Arguments
///
/// * `board` - The Kanban board containing all tasks
/// * `selected_lane` - Index of the currently focused lane (0-3)
/// * `selected_task` - Index of the selected task within the focused lane, if any
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Layout
///
/// ```text
/// +------------+------------+------------+------------+
/// | Backlog    | In Progress| Under Review| Done      |
/// +------------+------------+------------+------------+
/// | Task 1     | Task 3     | Task 5     | Task 7     |
/// | Task 2     | Task 4     |            |            |
/// |            |            |            |            |
/// +------------+------------+------------+------------+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_protocol::{KanbanBoard, Task};
/// use taim_tui::widgets::render_board;
///
/// let mut board = KanbanBoard::new();
/// board.add_task(Task::new("Task 1", "Description"));
///
/// let area = Rect::new(0, 0, 80, 20);
/// let mut buf = Buffer::empty(area);
///
/// render_board(&board, 0, Some(0), area, &mut buf);
/// ```
pub fn render_board(
    board: &KanbanBoard,
    selected_lane: usize,
    selected_task: Option<usize>,
    area: Rect,
    buf: &mut Buffer,
) {
    // Split into 4 equal columns for the lanes
    let lane_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Render each lane
    let lane_count = LaneKind::all().len();
    for (i, kind) in LaneKind::all().iter().enumerate() {
        let lane = board.lane(*kind);
        let is_focused = selected_lane == i;

        // Only show task selection in focused lane
        let task_selection = if is_focused { selected_task } else { None };

        // Determine lane position for border rendering
        let position = if i == 0 {
            LanePosition::First
        } else if i == lane_count - 1 {
            LanePosition::Last
        } else {
            LanePosition::Middle
        };

        // Check if the previous lane is focused (for shared border coloring)
        let prev_focused = i > 0 && selected_lane == i - 1;

        render_lane(lane, is_focused, task_selection, lane_areas[i], buf, position, prev_focused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taim_protocol::Task;

    #[test]
    fn render_empty_board() {
        let board = KanbanBoard::new();
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        render_board(&board, 0, None, area, &mut buf);

        let content = buffer_to_string(&buf);
        // All four lanes should be rendered
        assert!(content.contains("Backlog"));
        assert!(content.contains("In Progress"));
        assert!(content.contains("Under Review"));
        assert!(content.contains("Done"));
    }

    #[test]
    fn render_board_with_tasks() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "First task"));
        board.add_task(Task::new("Task 2", "Second task"));

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);

        render_board(&board, 0, Some(0), area, &mut buf);

        let content = buffer_to_string(&buf);
        assert!(content.contains("Backlog (2)"));
    }

    #[test]
    fn render_board_narrow_terminal() {
        let board = KanbanBoard::new();
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);

        // Should not panic with narrow area
        render_board(&board, 0, None, area, &mut buf);
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
