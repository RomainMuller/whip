//! Task-related types for the Kanban board.
//!
//! This module defines the core task types used throughout the whip application,
//! including task identifiers, states, and the task structure itself.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::board::LaneKind;

/// Unique identifier for a task.
///
/// Uses UUID v4 for globally unique identification.
pub type TaskId = uuid::Uuid;

/// The execution state of a task.
///
/// Represents the current status of work being performed on a task,
/// independent of which lane it resides in.
///
/// # Examples
///
/// ```
/// use whip_protocol::TaskState;
///
/// let state = TaskState::InFlight;
/// assert!(matches!(state, TaskState::InFlight));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task is waiting to be worked on.
    #[default]
    Idle,
    /// Task is actively being processed by an agent.
    InFlight,
    /// Task requires human intervention or review.
    NeedsAttention,
    /// Task completed successfully.
    Success,
    /// Task failed during execution.
    Failed,
}

impl TaskState {
    /// Returns `true` if the task is in a terminal state (Success or Failed).
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::TaskState;
    ///
    /// assert!(!TaskState::Idle.is_terminal());
    /// assert!(!TaskState::InFlight.is_terminal());
    /// assert!(TaskState::Success.is_terminal());
    /// assert!(TaskState::Failed.is_terminal());
    /// ```
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failed)
    }

    /// Returns `true` if the task requires attention.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::TaskState;
    ///
    /// assert!(!TaskState::Idle.needs_attention());
    /// assert!(TaskState::NeedsAttention.needs_attention());
    /// assert!(TaskState::Failed.needs_attention());
    /// ```
    #[must_use]
    pub const fn needs_attention(self) -> bool {
        matches!(self, Self::NeedsAttention | Self::Failed)
    }
}

/// A task on the Kanban board.
///
/// Represents a unit of work that can be tracked through the board's lanes.
/// Each task has a unique identifier, descriptive content, and metadata
/// about its current state and position.
///
/// # Examples
///
/// ```
/// use whip_protocol::{Task, TaskState, LaneKind};
///
/// let task = Task::new("Implement feature X", "Add the new feature to the codebase");
/// assert_eq!(task.state, TaskState::Idle);
/// assert_eq!(task.lane, LaneKind::Backlog);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task.
    pub id: TaskId,
    /// Short summary of the task.
    pub title: String,
    /// Detailed description of what needs to be done.
    pub description: String,
    /// Current execution state of the task.
    pub state: TaskState,
    /// Which lane this task currently resides in.
    pub lane: LaneKind,
    /// When this task was created.
    pub created_at: DateTime<Utc>,
    /// When this task was last modified.
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Creates a new task with the given title and description.
    ///
    /// The task is created in the `Backlog` lane with `Idle` state.
    /// Timestamps are set to the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::Task;
    ///
    /// let task = Task::new("Fix bug", "The login button doesn't work on mobile");
    /// assert_eq!(task.title, "Fix bug");
    /// ```
    #[must_use]
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new_v4(),
            title: title.into(),
            description: description.into(),
            state: TaskState::Idle,
            lane: LaneKind::Backlog,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new task with a specific ID.
    ///
    /// Useful for testing or when recreating tasks from persistent storage.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{Task, TaskId};
    ///
    /// let id = TaskId::new_v4();
    /// let task = Task::with_id(id, "Test task", "Description");
    /// assert_eq!(task.id, id);
    /// ```
    #[must_use]
    pub fn with_id(id: TaskId, title: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            description: description.into(),
            state: TaskState::Idle,
            lane: LaneKind::Backlog,
            created_at: now,
            updated_at: now,
        }
    }

    /// Updates the task's state and refreshes the `updated_at` timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{Task, TaskState};
    ///
    /// let mut task = Task::new("Work item", "Do the thing");
    /// task.set_state(TaskState::InFlight);
    /// assert_eq!(task.state, TaskState::InFlight);
    /// ```
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    /// Moves the task to a different lane and refreshes the `updated_at` timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{Task, LaneKind};
    ///
    /// let mut task = Task::new("Work item", "Do the thing");
    /// task.move_to_lane(LaneKind::InProgress);
    /// assert_eq!(task.lane, LaneKind::InProgress);
    /// ```
    pub fn move_to_lane(&mut self, lane: LaneKind) {
        self.lane = lane;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_state_default_is_idle() {
        assert_eq!(TaskState::default(), TaskState::Idle);
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    impl Arbitrary for TaskState {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(TaskState::Idle),
                Just(TaskState::InFlight),
                Just(TaskState::NeedsAttention),
                Just(TaskState::Success),
                Just(TaskState::Failed),
            ]
            .boxed()
        }
    }

    impl Arbitrary for LaneKind {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(LaneKind::Backlog),
                Just(LaneKind::InProgress),
                Just(LaneKind::UnderReview),
                Just(LaneKind::Done),
            ]
            .boxed()
        }
    }

    prop_compose! {
        fn arb_task()(
            title in "[a-zA-Z][a-zA-Z0-9 ]{0,50}",
            description in "[a-zA-Z0-9 .,!?]{0,200}",
            state in any::<TaskState>(),
            lane in any::<LaneKind>(),
        ) -> Task {
            let mut task = Task::new(title, description);
            task.state = state;
            task.lane = lane;
            task
        }
    }

    proptest! {
        /// Tests that TaskState serialization is deterministic and roundtrips correctly.
        #[test]
        fn task_state_roundtrip(state in any::<TaskState>()) {
            let json = serde_json::to_string(&state).expect("serialize");
            let parsed: TaskState = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(state, parsed);
        }

        /// Tests that TaskState serialization produces the same output on repeated calls.
        #[test]
        fn task_state_serialization_is_deterministic(state in any::<TaskState>()) {
            let json1 = serde_json::to_string(&state).expect("serialize 1");
            let json2 = serde_json::to_string(&state).expect("serialize 2");
            prop_assert_eq!(json1, json2);
        }

        /// Tests that LaneKind serialization is deterministic and roundtrips correctly.
        #[test]
        fn lane_kind_roundtrip(kind in any::<LaneKind>()) {
            let json = serde_json::to_string(&kind).expect("serialize");
            let parsed: LaneKind = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(kind, parsed);
        }

        /// Tests that LaneKind serialization produces the same output on repeated calls.
        #[test]
        fn lane_kind_serialization_is_deterministic(kind in any::<LaneKind>()) {
            let json1 = serde_json::to_string(&kind).expect("serialize 1");
            let json2 = serde_json::to_string(&kind).expect("serialize 2");
            prop_assert_eq!(json1, json2);
        }

        /// Tests that Task serialization roundtrips correctly, preserving all fields.
        #[test]
        fn task_roundtrip(task in arb_task()) {
            let json = serde_json::to_string(&task).expect("serialize");
            let parsed: Task = serde_json::from_str(&json).expect("deserialize");

            prop_assert_eq!(task.id, parsed.id);
            prop_assert_eq!(task.title, parsed.title);
            prop_assert_eq!(task.description, parsed.description);
            prop_assert_eq!(task.state, parsed.state);
            prop_assert_eq!(task.lane, parsed.lane);
            prop_assert_eq!(task.created_at, parsed.created_at);
            prop_assert_eq!(task.updated_at, parsed.updated_at);
        }

        /// Tests that Task serialization is deterministic.
        #[test]
        fn task_serialization_is_deterministic(task in arb_task()) {
            let json1 = serde_json::to_string(&task).expect("serialize 1");
            let json2 = serde_json::to_string(&task).expect("serialize 2");
            prop_assert_eq!(json1, json2);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn task_state_terminal_detection() {
        assert!(!TaskState::Idle.is_terminal());
        assert!(!TaskState::InFlight.is_terminal());
        assert!(!TaskState::NeedsAttention.is_terminal());
        assert!(TaskState::Success.is_terminal());
        assert!(TaskState::Failed.is_terminal());
    }

    #[test]
    fn task_state_attention_detection() {
        assert!(!TaskState::Idle.needs_attention());
        assert!(!TaskState::InFlight.needs_attention());
        assert!(TaskState::NeedsAttention.needs_attention());
        assert!(!TaskState::Success.needs_attention());
        assert!(TaskState::Failed.needs_attention());
    }

    #[test]
    fn task_new_creates_with_defaults() {
        let task = Task::new("Test", "Description");

        assert_eq!(task.title, "Test");
        assert_eq!(task.description, "Description");
        assert_eq!(task.state, TaskState::Idle);
        assert_eq!(task.lane, LaneKind::Backlog);
    }

    #[test]
    fn task_with_id_preserves_id() {
        let id = TaskId::new_v4();
        let task = Task::with_id(id, "Test", "Description");

        assert_eq!(task.id, id);
    }

    #[test]
    fn task_set_state_updates_timestamp() {
        let mut task = Task::new("Test", "Description");
        let original_updated = task.updated_at;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        task.set_state(TaskState::InFlight);

        assert_eq!(task.state, TaskState::InFlight);
        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn task_move_to_lane_updates_timestamp() {
        let mut task = Task::new("Test", "Description");
        let original_updated = task.updated_at;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        task.move_to_lane(LaneKind::InProgress);

        assert_eq!(task.lane, LaneKind::InProgress);
        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn task_state_serialization_roundtrip() {
        for state in [
            TaskState::Idle,
            TaskState::InFlight,
            TaskState::NeedsAttention,
            TaskState::Success,
            TaskState::Failed,
        ] {
            let json = serde_json::to_string(&state).expect("serialize");
            let parsed: TaskState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, parsed);
        }
    }

    #[test]
    fn task_serialization_roundtrip() {
        let task = Task::new("Test task", "A description");
        let json = serde_json::to_string(&task).expect("serialize");
        let parsed: Task = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(task.id, parsed.id);
        assert_eq!(task.title, parsed.title);
        assert_eq!(task.description, parsed.description);
        assert_eq!(task.state, parsed.state);
        assert_eq!(task.lane, parsed.lane);
    }

    #[test]
    fn task_state_json_format() {
        // Verify snake_case serialization
        let json = serde_json::to_string(&TaskState::NeedsAttention).expect("serialize");
        assert_eq!(json, r#""needs_attention""#);

        let json = serde_json::to_string(&TaskState::InFlight).expect("serialize");
        assert_eq!(json, r#""in_flight""#);
    }
}
