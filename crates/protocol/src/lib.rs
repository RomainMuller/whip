//! Shared protocol types for the whip application.
//!
//! This crate defines the core types used across all whip components,
//! including tasks, the Kanban board structure, messages, and error types.
//!
//! # Overview
//!
//! The crate is organized into the following modules:
//!
//! - [`task`]: Task identifiers, states, and the `Task` struct
//! - [`board`]: Kanban board lanes and the `KanbanBoard` struct
//! - [`message`]: TUI event messages
//! - [`error`]: Error types for protocol operations
//!
//! # Examples
//!
//! Creating and managing tasks on a board:
//!
//! ```
//! use whip_protocol::{KanbanBoard, Task, LaneKind, TaskState};
//!
//! // Create a new board
//! let mut board = KanbanBoard::new();
//!
//! // Add a task (starts in Backlog)
//! let task = Task::new("Implement feature", "Add user authentication");
//! let task_id = task.id;
//! board.add_task(task);
//!
//! // Move the task through the workflow
//! board.move_task(task_id, LaneKind::InProgress);
//!
//! // Update task state
//! if let Some(task) = board.get_task_mut(task_id) {
//!     task.set_state(TaskState::InFlight);
//! }
//! ```

pub mod board;
pub mod dummy;
pub mod error;
pub mod label;
pub mod message;
pub mod task;

// Re-export primary types at crate root for convenience
pub use board::{KanbanBoard, Lane, LaneKind};
pub use error::{ProtocolError, Result};
pub use label::{
    LABEL_PREFIX, LabelDefinition, StatusFromLabel, determine_lane_from_labels,
    determine_status_from_labels, has_whip_status_label, label_to_lane, label_to_status,
    standard_status_labels,
};
pub use message::Message;
pub use task::{GitHubSource, Task, TaskId, TaskState};
