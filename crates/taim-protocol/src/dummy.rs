//! Dummy data generation for testing and demonstration.
//!
//! This module provides functions to generate sample Kanban boards
//! with realistic tasks for testing the TUI and demonstrating the
//! application's capabilities.
//!
//! # Examples
//!
//! ```
//! use taim_protocol::dummy::dummy_board;
//!
//! let board = dummy_board();
//! assert_eq!(board.total_tasks(), 8);
//! ```

use crate::board::{KanbanBoard, LaneKind};
use crate::task::{Task, TaskState};

/// A builder for creating tasks with specific states and lanes.
///
/// This is an internal helper to reduce boilerplate when creating
/// multiple tasks with non-default states and lanes.
struct TaskBuilder {
    title: String,
    description: String,
    state: TaskState,
    lane: LaneKind,
}

impl TaskBuilder {
    /// Creates a new task builder with the given title and description.
    fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            state: TaskState::Idle,
            lane: LaneKind::Backlog,
        }
    }

    /// Sets the task state.
    fn state(mut self, state: TaskState) -> Self {
        self.state = state;
        self
    }

    /// Sets the task lane.
    fn lane(mut self, lane: LaneKind) -> Self {
        self.lane = lane;
        self
    }

    /// Builds the task with the configured state and lane.
    fn build(self) -> Task {
        let mut task = Task::new(self.title, self.description);
        task.state = self.state;
        task.lane = self.lane;
        task
    }
}

/// Generates a sample Kanban board with realistic tasks.
///
/// Creates a board with tasks distributed across all four lanes,
/// demonstrating various task states:
///
/// - **Backlog**: 3 tasks in `Idle` state
/// - **In Progress**: 2 tasks (one `InFlight`, one `NeedsAttention`)
/// - **Under Review**: 1 task in `InFlight` state
/// - **Done**: 2 tasks (one `Success`, one `Failed`)
///
/// # Examples
///
/// ```
/// use taim_protocol::dummy::dummy_board;
/// use taim_protocol::LaneKind;
///
/// let board = dummy_board();
///
/// // Check task distribution
/// assert_eq!(board.lane(LaneKind::Backlog).len(), 3);
/// assert_eq!(board.lane(LaneKind::InProgress).len(), 2);
/// assert_eq!(board.lane(LaneKind::UnderReview).len(), 1);
/// assert_eq!(board.lane(LaneKind::Done).len(), 2);
/// ```
#[must_use]
pub fn dummy_board() -> KanbanBoard {
    let mut board = KanbanBoard::new();

    // Backlog tasks (Idle state)
    board.add_task(
        TaskBuilder::new(
            "Set up CI/CD pipeline",
            "Configure GitHub Actions for automated testing and deployment.\n\n\
             Include:\n\
             - Rust toolchain setup\n\
             - Cargo test\n\
             - Cargo clippy\n\
             - Cargo fmt check\n\
             - Release builds",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Write API documentation",
            "Document all public APIs with examples.\n\n\
             Use rustdoc conventions and ensure all public items have doc comments.",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Add configuration file support",
            "Implement taim-config crate for loading TOML/YAML configuration files.\n\n\
             Support:\n\
             - Default config locations\n\
             - Environment variable overrides\n\
             - Command-line argument overrides",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    // In Progress tasks (mix of InFlight and NeedsAttention)
    board.add_task(
        TaskBuilder::new(
            "Implement Kanban TUI",
            "Build the terminal user interface with Ratatui.\n\n\
             Features:\n\
             - Four-lane board with task cards\n\
             - Keyboard navigation\n\
             - Task detail panel\n\
             - Responsive layout",
        )
        .state(TaskState::InFlight)
        .lane(LaneKind::InProgress)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Fix memory leak in event loop",
            "Investigate growing memory usage during long sessions.\n\n\
             Profiler shows unbounded buffer growth in the message channel.\n\
             Need to add backpressure or bounded channels.",
        )
        .state(TaskState::NeedsAttention)
        .lane(LaneKind::InProgress)
        .build(),
    );

    // Under Review task (InFlight)
    board.add_task(
        TaskBuilder::new(
            "Add Claude Code integration",
            "Spawn and manage Claude Code subprocess.\n\n\
             Implementation:\n\
             - Parse JSON-RPC messages\n\
             - Handle process lifecycle\n\
             - Stream stdout/stderr\n\
             - Graceful shutdown",
        )
        .state(TaskState::InFlight)
        .lane(LaneKind::UnderReview)
        .build(),
    );

    // Done tasks (Success and Failed)
    board.add_task(
        TaskBuilder::new(
            "Project setup",
            "Initialize Rust workspace with Cargo.toml and crate structure.\n\n\
             Created:\n\
             - taim (root binary)\n\
             - taim-protocol\n\
             - taim-tui\n\
             - taim-session (planned)\n\
             - taim-config (planned)",
        )
        .state(TaskState::Success)
        .lane(LaneKind::Done)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Implement REST API",
            "Add HTTP endpoints for remote control.\n\n\
             Abandoned in favor of TUI-first approach.\n\
             May revisit later for headless operation.",
        )
        .state(TaskState::Failed)
        .lane(LaneKind::Done)
        .build(),
    );

    board
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_board_has_correct_task_count() {
        let board = dummy_board();
        assert_eq!(board.total_tasks(), 8);
    }

    #[test]
    fn dummy_board_has_tasks_in_all_lanes() {
        let board = dummy_board();

        assert_eq!(board.lane(LaneKind::Backlog).len(), 3);
        assert_eq!(board.lane(LaneKind::InProgress).len(), 2);
        assert_eq!(board.lane(LaneKind::UnderReview).len(), 1);
        assert_eq!(board.lane(LaneKind::Done).len(), 2);
    }

    #[test]
    fn dummy_board_tasks_have_correct_states() {
        let board = dummy_board();

        // Check Backlog tasks are Idle
        for task in &board.lane(LaneKind::Backlog).tasks {
            assert_eq!(task.state, TaskState::Idle);
            assert_eq!(task.lane, LaneKind::Backlog);
        }

        // Check In Progress has both InFlight and NeedsAttention
        let in_progress = &board.lane(LaneKind::InProgress).tasks;
        assert!(in_progress.iter().any(|t| t.state == TaskState::InFlight));
        assert!(
            in_progress
                .iter()
                .any(|t| t.state == TaskState::NeedsAttention)
        );

        // Check Under Review task is InFlight
        let under_review = &board.lane(LaneKind::UnderReview).tasks;
        assert!(under_review.iter().all(|t| t.state == TaskState::InFlight));

        // Check Done has both Success and Failed
        let done = &board.lane(LaneKind::Done).tasks;
        assert!(done.iter().any(|t| t.state == TaskState::Success));
        assert!(done.iter().any(|t| t.state == TaskState::Failed));
    }

    #[test]
    fn dummy_board_task_lanes_match_lane_kind() {
        let board = dummy_board();

        for lane_kind in LaneKind::all() {
            let lane = board.lane(lane_kind);
            for task in &lane.tasks {
                assert_eq!(
                    task.lane, lane_kind,
                    "Task '{}' has lane {:?} but is in lane {:?}",
                    task.title, task.lane, lane_kind
                );
            }
        }
    }
}
