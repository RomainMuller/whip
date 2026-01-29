//! Dummy data generation for testing and demonstration.
//!
//! This module provides functions to generate sample Kanban boards
//! with realistic tasks for testing the TUI and demonstrating the
//! application's capabilities.
//!
//! # Examples
//!
//! ```
//! use whip_protocol::dummy::dummy_board;
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
/// use whip_protocol::dummy::dummy_board;
/// use whip_protocol::LaneKind;
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
            "## Overview\n\
             Configure **GitHub Actions** for automated testing and deployment.\n\n\
             ## Checklist\n\
             1. Set up Rust toolchain with `actions-rs/toolchain`\n\
             2. Run `cargo test --workspace`\n\
             3. Run `cargo clippy -- -D warnings`\n\
             4. Check formatting with `cargo fmt --check`\n\
             5. Build release binaries\n\n\
             ## Notes\n\
             *Priority*: High - blocks other CI-dependent work",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Write API documentation",
            "## Goal\n\
             Document all **public APIs** with comprehensive examples.\n\n\
             ## Standards\n\
             - Follow *rustdoc* conventions\n\
             - Include `# Examples` section for each public function\n\
             - Document **panics** and **errors** where applicable\n\n\
             ## Example Template\n\
             ```rust\n\
             /// Creates a new session.\n\
             ///\n\
             /// # Examples\n\
             /// ```\n\
             /// let session = Session::new();\n\
             /// ```\n\
             pub fn new() -> Self { ... }\n\
             ```",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Add configuration file support",
            "## Objective\n\
             Implement `whip-config` crate for loading configuration files.\n\n\
             ## Supported Formats\n\
             - **TOML** (primary)\n\
             - YAML (optional)\n\n\
             ## Configuration Options\n\n\
             | Option | Type | Default | Environment | Description |\n\
             |--------|------|---------|-------------|-------------|\n\
             | `session.timeout` | integer | 300 | `WHIP_TIMEOUT` | Session timeout in seconds |\n\
             | `session.max_retries` | integer | 3 | `WHIP_RETRIES` | Max retry attempts |\n\
             | `ui.theme` | string | \"dark\" | `WHIP_THEME` | Color theme (dark/light) |\n\
             | `ui.refresh_rate` | integer | 60 | `WHIP_REFRESH` | UI refresh rate in Hz |\n\
             | `log.level` | string | \"info\" | `WHIP_LOG` | Log level (debug/info/warn) |\n\
             | `log.file` | path | none | `WHIP_LOG_FILE` | Optional log file path |\n\n\
             ## Configuration Sources\n\
             Priority order (highest to lowest):\n\
             1. Command-line arguments\n\
             2. Environment variables (`WHIP_*`)\n\
             3. Local config (`./whip.toml`)\n\
             4. User config (`~/.config/whip/config.toml`)\n\
             5. Built-in defaults\n\n\
             ## Sample Config\n\
             ```toml\n\
             [session]\n\
             timeout = 300\n\
             max_retries = 3\n\
             ```",
        )
        .state(TaskState::Idle)
        .lane(LaneKind::Backlog)
        .build(),
    );

    // In Progress tasks (mix of InFlight and NeedsAttention)
    board.add_task(
        TaskBuilder::new(
            "Implement Kanban TUI",
            "## Status\n\
             Building terminal UI with **Ratatui** framework.\n\n\
             ## Features\n\
             - [x] Four-lane board layout\n\
             - [x] Task cards with state indicators\n\
             - [ ] Keyboard navigation (`h/j/k/l`)\n\
             - [ ] Task detail panel\n\
             - [ ] Responsive resize handling\n\n\
             ## Architecture\n\
             ```\n\
             App -> Board -> Lanes -> Tasks\n\
                       |         |\n\
                       v         v\n\
                   Header    Cards\n\
             ```\n\n\
             ## Key Bindings\n\
             | Key | Action |\n\
             |-----|--------|\n\
             | `q` | Quit |\n\
             | `j/k` | Navigate |\n\
             | `Enter` | Select |",
        )
        .state(TaskState::InFlight)
        .lane(LaneKind::InProgress)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Fix memory leak in event loop",
            "## Problem\n\
             Memory usage grows **unbounded** during long sessions.\n\n\
             ## Symptoms\n\
             - RSS increases ~1MB/minute\n\
             - Eventually causes `OOM` on constrained systems\n\
             - Profiler points to *message channel buffers*\n\n\
             ## Root Cause\n\
             Using unbounded channel in event loop:\n\
             ```rust\n\
             // Current (problematic)\n\
             let (tx, rx) = mpsc::unbounded_channel();\n\
             ```\n\n\
             ## Proposed Fix\n\
             Switch to bounded channel with backpressure:\n\
             ```rust\n\
             // Fixed\n\
             let (tx, rx) = mpsc::channel(100);\n\
             ```\n\n\
             **Warning**: This may require handling `SendError` when buffer is full.",
        )
        .state(TaskState::NeedsAttention)
        .lane(LaneKind::InProgress)
        .build(),
    );

    // Under Review task (InFlight)
    board.add_task(
        TaskBuilder::new(
            "Add Claude Code integration",
            "## Summary\n\
             Implement `whip-session` crate to spawn and manage **Claude Code** subprocesses.\n\n\
             ## Implementation Details\n\
             ### Message Protocol\n\
             Uses *JSON-RPC 2.0* over stdio:\n\
             ```json\n\
             {\"jsonrpc\": \"2.0\", \"method\": \"init\", \"id\": 1}\n\
             ```\n\n\
             ### Process Lifecycle\n\
             1. Spawn via `tokio::process::Command`\n\
             2. Pipe `stdin`/`stdout`/`stderr`\n\
             3. Monitor for unexpected termination\n\
             4. Graceful shutdown with `SIGTERM`\n\n\
             ## Review Checklist\n\
             - [ ] Error handling is comprehensive\n\
             - [ ] No resource leaks on panic\n\
             - [ ] Tests cover edge cases\n\
             - [ ] Documentation complete",
        )
        .state(TaskState::InFlight)
        .lane(LaneKind::UnderReview)
        .build(),
    );

    // Done tasks (Success and Failed)
    board.add_task(
        TaskBuilder::new(
            "Project setup",
            "## Completed\n\
             Initialized Rust workspace with **Cargo.toml** and crate structure.\n\n\
             ## Workspace Structure\n\
             ```\n\
             whip/\n\
             +-- Cargo.toml          # Workspace root\n\
             +-- src/main.rs         # CLI binary\n\
             +-- crates/\n\
                 +-- whip-protocol/  # Shared types\n\
                 +-- whip-tui/       # Terminal UI\n\
                 +-- whip-session/   # Process mgmt\n\
                 +-- whip-config/    # Configuration\n\
             ```\n\n\
             ## Key Decisions\n\
             - **Edition**: Rust 2024\n\
             - **Async runtime**: tokio\n\
             - **Error handling**: `thiserror` + `anyhow`",
        )
        .state(TaskState::Success)
        .lane(LaneKind::Done)
        .build(),
    );

    board.add_task(
        TaskBuilder::new(
            "Implement REST API",
            "## Status: *Abandoned*\n\n\
             Originally planned HTTP endpoints for remote control.\n\n\
             ## Reason\n\
             Decided to focus on **TUI-first** approach:\n\
             - Simpler architecture\n\
             - No network security concerns\n\
             - Faster iteration\n\n\
             ## Future Consideration\n\
             May revisit for **headless operation** mode:\n\
             ```\n\
             whip --headless --port 8080\n\
             ```\n\n\
             > Note: Would require authentication and TLS support.",
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
