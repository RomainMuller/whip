//! Snapshot tests for widget rendering.
//!
//! These tests use insta to capture and verify the visual output of widgets.

use chrono::{TimeZone, Utc};
use ratatui::{buffer::Buffer, layout::Rect};
use whip_protocol::{KanbanBoard, LaneKind, Task, TaskState};

/// Creates a test task with fixed timestamps for reproducible snapshots.
fn test_task(title: &str, description: &str) -> Task {
    let mut task = Task::new(title, description);
    // Use a fixed timestamp for reproducible snapshots
    let fixed_time = Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 0).unwrap();
    task.created_at = fixed_time;
    task.updated_at = fixed_time;
    task
}

use super::{
    LanePosition, render_board, render_detail_panel, render_help_overlay, render_lane,
    render_status_bar, render_task_card,
};

/// Converts a buffer to a string representation for snapshot testing.
fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        // Trim trailing whitespace from each line for cleaner snapshots
        let trimmed = result.trim_end_matches(' ');
        result.truncate(trimmed.len());
        result.push('\n');
    }
    result
}

/// Creates a sample board with tasks in various states for testing.
fn create_sample_board() -> KanbanBoard {
    let mut board = KanbanBoard::new();

    // Backlog tasks
    let mut task1 = Task::new("Design UI mockups", "Create wireframes for the new feature");
    task1.state = TaskState::Idle;
    board.add_task(task1);

    let mut task2 = Task::new("Write specs", "Document requirements");
    task2.state = TaskState::NeedsAttention;
    board.add_task(task2);

    // In Progress tasks
    let mut task3 = Task::new("Implement parser", "Build JSON parser module");
    task3.state = TaskState::InFlight;
    task3.lane = LaneKind::InProgress;
    board.lane_mut(LaneKind::InProgress).add_task(task3);

    // Under Review tasks
    let mut task4 = Task::new("Code review: auth", "Review authentication PR");
    task4.state = TaskState::Idle;
    task4.lane = LaneKind::UnderReview;
    board.lane_mut(LaneKind::UnderReview).add_task(task4);

    // Done tasks
    let mut task5 = Task::new("Setup CI/CD", "Configure GitHub Actions");
    task5.state = TaskState::Success;
    task5.lane = LaneKind::Done;
    board.lane_mut(LaneKind::Done).add_task(task5);

    let mut task6 = Task::new("Fix login bug", "Users couldn't log in");
    task6.state = TaskState::Failed;
    task6.lane = LaneKind::Done;
    board.lane_mut(LaneKind::Done).add_task(task6);

    board
}

#[test]
fn snapshot_empty_board() {
    let board = KanbanBoard::new();
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);

    render_board(&board, 0, None, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_board_with_tasks() {
    let board = create_sample_board();
    let area = Rect::new(0, 0, 100, 24);
    let mut buf = Buffer::empty(area);

    render_board(&board, 0, Some(0), area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_board_different_lane_selected() {
    let board = create_sample_board();
    let area = Rect::new(0, 0, 100, 24);
    let mut buf = Buffer::empty(area);

    render_board(&board, 2, None, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_lane_empty() {
    let lane = whip_protocol::Lane::new(LaneKind::Backlog);
    let area = Rect::new(0, 0, 25, 15);
    let mut buf = Buffer::empty(area);

    render_lane(
        &lane,
        false,
        None,
        area,
        &mut buf,
        LanePosition::First,
        false,
    );

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_lane_with_tasks() {
    let mut lane = whip_protocol::Lane::new(LaneKind::InProgress);

    let mut task1 = Task::new("Active task", "Currently being worked on");
    task1.state = TaskState::InFlight;
    lane.add_task(task1);

    let mut task2 = Task::new("Waiting task", "In queue");
    task2.state = TaskState::Idle;
    lane.add_task(task2);

    let area = Rect::new(0, 0, 25, 15);
    let mut buf = Buffer::empty(area);

    render_lane(
        &lane,
        true,
        Some(0),
        area,
        &mut buf,
        LanePosition::Middle,
        false,
    );

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_idle() {
    let mut task = Task::new("Idle Task", "Waiting to be started");
    task.state = TaskState::Idle;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, false, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_inflight() {
    let mut task = Task::new("Active Task", "Work in progress");
    task.state = TaskState::InFlight;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, false, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_needs_attention() {
    let mut task = Task::new("Blocked Task", "Needs input from PM");
    task.state = TaskState::NeedsAttention;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, false, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_success() {
    let mut task = Task::new("Completed Task", "Successfully finished");
    task.state = TaskState::Success;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, false, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_failed() {
    let mut task = Task::new("Failed Task", "Error during execution");
    task.state = TaskState::Failed;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, false, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_task_card_selected() {
    let mut task = Task::new("Selected Task", "This task is selected");
    task.state = TaskState::InFlight;

    let area = Rect::new(0, 0, 22, 4);
    let mut buf = Buffer::empty(area);

    render_task_card(&task, true, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_status_bar() {
    let area = Rect::new(0, 0, 80, 3);
    let mut buf = Buffer::empty(area);

    render_status_bar(area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_board_narrow_terminal() {
    let board = create_sample_board();
    let area = Rect::new(0, 0, 60, 15);
    let mut buf = Buffer::empty(area);

    render_board(&board, 1, Some(0), area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_idle() {
    let mut task = test_task(
        "Idle Task",
        "This task is waiting to be started. It has a longer description to test text wrapping in the detail panel.",
    );
    task.state = TaskState::Idle;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_inflight() {
    let mut task = test_task("Active Task", "Work in progress on this feature.");
    task.state = TaskState::InFlight;
    task.lane = LaneKind::InProgress;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_needs_attention() {
    let mut task = test_task(
        "Blocked Task",
        "Waiting for input from the product manager.",
    );
    task.state = TaskState::NeedsAttention;
    task.lane = LaneKind::InProgress;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_success() {
    let mut task = test_task("Completed Task", "Successfully finished and deployed.");
    task.state = TaskState::Success;
    task.lane = LaneKind::Done;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_failed() {
    let mut task = test_task("Failed Task", "Error occurred during execution.");
    task.state = TaskState::Failed;
    task.lane = LaneKind::Done;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_with_scroll() {
    let mut task = test_task(
        "Long Description Task",
        "This is a very long description that should require scrolling when displayed. \
         It contains multiple sentences to ensure we have enough content. \
         The detail panel should handle this gracefully by allowing the user to scroll.",
    );
    task.state = TaskState::InFlight;
    task.lane = LaneKind::InProgress;

    let area = Rect::new(0, 0, 40, 15);
    let mut buf = Buffer::empty(area);

    // Render with scroll offset
    render_detail_panel(&task, 3, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_empty_description() {
    let mut task = test_task("No Description", "");
    task.state = TaskState::Idle;

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_medium_width() {
    // Test the 2-row metadata layout with aligned delimiters
    let mut task = test_task("Medium Width Task", "Testing the medium width layout.");
    task.state = TaskState::InFlight;
    task.lane = LaneKind::InProgress;

    // Width of 80 should trigger 2-row layout (status+lane on row 1, timestamps on row 2)
    // The aligned timestamps line needs ~75 chars (padding + separators + content)
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_detail_panel_wide() {
    // Test the 1-row metadata layout (everything on one line)
    let mut task = test_task("Wide Screen Task", "Testing the wide layout.");
    task.state = TaskState::Success;
    task.lane = LaneKind::Done;

    // Width of 100 should fit everything on one line
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);

    render_detail_panel(&task, 0, area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_help_overlay() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);

    render_help_overlay(area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}

#[test]
fn snapshot_help_overlay_small_terminal() {
    let area = Rect::new(0, 0, 40, 15);
    let mut buf = Buffer::empty(area);

    render_help_overlay(area, &mut buf);

    insta::assert_snapshot!(buffer_to_string(&buf));
}
