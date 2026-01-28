//! Kanban board types and structures.
//!
//! This module defines the board layout types including lanes and the
//! overall board structure that organizes tasks.

use serde::{Deserialize, Serialize};

use crate::task::{Task, TaskId};

/// The type of lane on a Kanban board.
///
/// Represents the workflow stages that tasks move through.
/// The order reflects the typical progression of work.
///
/// # Examples
///
/// ```
/// use taim_protocol::LaneKind;
///
/// let lane = LaneKind::InProgress;
/// assert_eq!(lane.display_name(), "In Progress");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LaneKind {
    /// Tasks waiting to be started.
    #[default]
    Backlog,
    /// Tasks currently being worked on.
    InProgress,
    /// Tasks awaiting review or approval.
    UnderReview,
    /// Completed tasks.
    Done,
}

impl LaneKind {
    /// Returns all lane kinds in workflow order.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// let lanes = LaneKind::all();
    /// assert_eq!(lanes.len(), 4);
    /// assert_eq!(lanes[0], LaneKind::Backlog);
    /// ```
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::Backlog,
            Self::InProgress,
            Self::UnderReview,
            Self::Done,
        ]
    }

    /// Returns a human-readable display name for the lane.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// assert_eq!(LaneKind::Backlog.display_name(), "Backlog");
    /// assert_eq!(LaneKind::UnderReview.display_name(), "Under Review");
    /// ```
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Backlog => "Backlog",
            Self::InProgress => "In Progress",
            Self::UnderReview => "Under Review",
            Self::Done => "Done",
        }
    }

    /// Returns the index of this lane in the workflow (0-3).
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// assert_eq!(LaneKind::Backlog.index(), 0);
    /// assert_eq!(LaneKind::Done.index(), 3);
    /// ```
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Backlog => 0,
            Self::InProgress => 1,
            Self::UnderReview => 2,
            Self::Done => 3,
        }
    }

    /// Creates a `LaneKind` from its index.
    ///
    /// Returns `None` if the index is out of range (>= 4).
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// assert_eq!(LaneKind::from_index(0), Some(LaneKind::Backlog));
    /// assert_eq!(LaneKind::from_index(4), None);
    /// ```
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Backlog),
            1 => Some(Self::InProgress),
            2 => Some(Self::UnderReview),
            3 => Some(Self::Done),
            _ => None,
        }
    }

    /// Returns the next lane in the workflow, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// assert_eq!(LaneKind::Backlog.next(), Some(LaneKind::InProgress));
    /// assert_eq!(LaneKind::Done.next(), None);
    /// ```
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        Self::from_index(self.index() + 1)
    }

    /// Returns the previous lane in the workflow, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::LaneKind;
    ///
    /// assert_eq!(LaneKind::InProgress.previous(), Some(LaneKind::Backlog));
    /// assert_eq!(LaneKind::Backlog.previous(), None);
    /// ```
    #[must_use]
    pub const fn previous(self) -> Option<Self> {
        match self.index().checked_sub(1) {
            Some(idx) => Self::from_index(idx),
            None => None,
        }
    }
}

/// A single lane (column) on the Kanban board.
///
/// Contains tasks that share the same workflow stage.
///
/// # Examples
///
/// ```
/// use taim_protocol::{Lane, LaneKind, Task};
///
/// let lane = Lane::new(LaneKind::Backlog);
/// assert!(lane.is_empty());
/// assert_eq!(lane.kind, LaneKind::Backlog);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lane {
    /// The type of this lane.
    pub kind: LaneKind,
    /// Tasks currently in this lane, ordered by position.
    pub tasks: Vec<Task>,
}

impl Lane {
    /// Creates a new empty lane of the specified kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind};
    ///
    /// let lane = Lane::new(LaneKind::InProgress);
    /// assert_eq!(lane.kind, LaneKind::InProgress);
    /// assert!(lane.tasks.is_empty());
    /// ```
    #[must_use]
    pub const fn new(kind: LaneKind) -> Self {
        Self {
            kind,
            tasks: Vec::new(),
        }
    }

    /// Returns the number of tasks in this lane.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind, Task};
    ///
    /// let mut lane = Lane::new(LaneKind::Backlog);
    /// assert_eq!(lane.len(), 0);
    ///
    /// lane.add_task(Task::new("Task", "Description"));
    /// assert_eq!(lane.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` if the lane has no tasks.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind};
    ///
    /// let lane = Lane::new(LaneKind::Done);
    /// assert!(lane.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Adds a task to the end of this lane.
    ///
    /// Note: This does not update the task's `lane` field. Use
    /// [`KanbanBoard::move_task`] for proper task movement.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind, Task};
    ///
    /// let mut lane = Lane::new(LaneKind::Backlog);
    /// lane.add_task(Task::new("New task", "Do something"));
    /// assert_eq!(lane.len(), 1);
    /// ```
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Removes and returns a task by ID, if found.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind, Task};
    ///
    /// let mut lane = Lane::new(LaneKind::Backlog);
    /// let task = Task::new("Task", "Description");
    /// let id = task.id;
    /// lane.add_task(task);
    ///
    /// let removed = lane.remove_task(id);
    /// assert!(removed.is_some());
    /// assert!(lane.is_empty());
    /// ```
    pub fn remove_task(&mut self, id: TaskId) -> Option<Task> {
        let pos = self.tasks.iter().position(|t| t.id == id)?;
        Some(self.tasks.remove(pos))
    }

    /// Returns a reference to a task by ID, if found.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{Lane, LaneKind, Task};
    ///
    /// let mut lane = Lane::new(LaneKind::Backlog);
    /// let task = Task::new("Task", "Description");
    /// let id = task.id;
    /// lane.add_task(task);
    ///
    /// assert!(lane.get_task(id).is_some());
    /// ```
    #[must_use]
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Returns a mutable reference to a task by ID, if found.
    #[must_use]
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}

/// A Kanban board with four fixed lanes.
///
/// The board organizes tasks across workflow stages: Backlog, In Progress,
/// Under Review, and Done.
///
/// # Examples
///
/// ```
/// use taim_protocol::{KanbanBoard, LaneKind, Task};
///
/// let mut board = KanbanBoard::new();
/// let task = Task::new("Implement feature", "Add the new widget");
/// let id = task.id;
///
/// board.add_task(task);
/// assert!(board.get_task(id).is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanbanBoard {
    /// The four lanes of the board, indexed by [`LaneKind::index`].
    pub lanes: [Lane; 4],
}

impl Default for KanbanBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl KanbanBoard {
    /// Creates a new empty Kanban board with four lanes.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::KanbanBoard;
    ///
    /// let board = KanbanBoard::new();
    /// assert_eq!(board.lanes.len(), 4);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            lanes: [
                Lane::new(LaneKind::Backlog),
                Lane::new(LaneKind::InProgress),
                Lane::new(LaneKind::UnderReview),
                Lane::new(LaneKind::Done),
            ],
        }
    }

    /// Returns a reference to the lane of the specified kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, LaneKind};
    ///
    /// let board = KanbanBoard::new();
    /// let backlog = board.lane(LaneKind::Backlog);
    /// assert_eq!(backlog.kind, LaneKind::Backlog);
    /// ```
    #[must_use]
    pub fn lane(&self, kind: LaneKind) -> &Lane {
        &self.lanes[kind.index()]
    }

    /// Returns a mutable reference to the lane of the specified kind.
    #[must_use]
    pub fn lane_mut(&mut self, kind: LaneKind) -> &mut Lane {
        &mut self.lanes[kind.index()]
    }

    /// Adds a task to its designated lane based on `task.lane`.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, LaneKind, Task};
    ///
    /// let mut board = KanbanBoard::new();
    /// let task = Task::new("Task", "Description");
    ///
    /// board.add_task(task);
    /// assert_eq!(board.lane(LaneKind::Backlog).len(), 1);
    /// ```
    pub fn add_task(&mut self, task: Task) {
        let lane_kind = task.lane;
        self.lane_mut(lane_kind).add_task(task);
    }

    /// Finds and returns a reference to a task by ID across all lanes.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, Task};
    ///
    /// let mut board = KanbanBoard::new();
    /// let task = Task::new("Task", "Description");
    /// let id = task.id;
    /// board.add_task(task);
    ///
    /// assert!(board.get_task(id).is_some());
    /// ```
    #[must_use]
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.lanes.iter().find_map(|lane| lane.get_task(id))
    }

    /// Finds and returns a mutable reference to a task by ID across all lanes.
    #[must_use]
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.lanes.iter_mut().find_map(|lane| lane.get_task_mut(id))
    }

    /// Moves a task from its current lane to a new lane.
    ///
    /// Returns `true` if the task was found and moved, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, LaneKind, Task};
    ///
    /// let mut board = KanbanBoard::new();
    /// let task = Task::new("Task", "Description");
    /// let id = task.id;
    /// board.add_task(task);
    ///
    /// assert!(board.move_task(id, LaneKind::InProgress));
    /// assert_eq!(board.get_task(id).unwrap().lane, LaneKind::InProgress);
    /// ```
    pub fn move_task(&mut self, id: TaskId, to_lane: LaneKind) -> bool {
        // Find which lane contains the task
        let from_lane = self
            .lanes
            .iter()
            .find(|lane| lane.get_task(id).is_some())
            .map(|lane| lane.kind);

        let Some(from_kind) = from_lane else {
            return false;
        };

        // Remove from current lane
        let Some(mut task) = self.lane_mut(from_kind).remove_task(id) else {
            return false;
        };

        // Update task's lane field and add to new lane
        task.move_to_lane(to_lane);
        self.lane_mut(to_lane).add_task(task);

        true
    }

    /// Returns the total number of tasks across all lanes.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, Task};
    ///
    /// let mut board = KanbanBoard::new();
    /// assert_eq!(board.total_tasks(), 0);
    ///
    /// board.add_task(Task::new("Task 1", "Description"));
    /// board.add_task(Task::new("Task 2", "Description"));
    /// assert_eq!(board.total_tasks(), 2);
    /// ```
    #[must_use]
    pub fn total_tasks(&self) -> usize {
        self.lanes.iter().map(Lane::len).sum()
    }

    /// Removes a task by ID from any lane.
    ///
    /// Returns the removed task if found, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::{KanbanBoard, Task};
    ///
    /// let mut board = KanbanBoard::new();
    /// let task = Task::new("Task", "Description");
    /// let id = task.id;
    /// board.add_task(task);
    ///
    /// let removed = board.remove_task(id);
    /// assert!(removed.is_some());
    /// assert_eq!(board.total_tasks(), 0);
    /// ```
    pub fn remove_task(&mut self, id: TaskId) -> Option<Task> {
        for lane in &mut self.lanes {
            if let Some(task) = lane.remove_task(id) {
                return Some(task);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_kind_all_returns_four_lanes() {
        let all = LaneKind::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], LaneKind::Backlog);
        assert_eq!(all[1], LaneKind::InProgress);
        assert_eq!(all[2], LaneKind::UnderReview);
        assert_eq!(all[3], LaneKind::Done);
    }

    #[test]
    fn lane_kind_index_roundtrip() {
        for kind in LaneKind::all() {
            let idx = kind.index();
            assert_eq!(LaneKind::from_index(idx), Some(kind));
        }
    }

    #[test]
    fn lane_kind_navigation() {
        assert_eq!(LaneKind::Backlog.next(), Some(LaneKind::InProgress));
        assert_eq!(LaneKind::InProgress.next(), Some(LaneKind::UnderReview));
        assert_eq!(LaneKind::UnderReview.next(), Some(LaneKind::Done));
        assert_eq!(LaneKind::Done.next(), None);

        assert_eq!(LaneKind::Done.previous(), Some(LaneKind::UnderReview));
        assert_eq!(LaneKind::UnderReview.previous(), Some(LaneKind::InProgress));
        assert_eq!(LaneKind::InProgress.previous(), Some(LaneKind::Backlog));
        assert_eq!(LaneKind::Backlog.previous(), None);
    }

    #[test]
    fn lane_kind_serialization_roundtrip() {
        for kind in LaneKind::all() {
            let json = serde_json::to_string(&kind).expect("serialize");
            let parsed: LaneKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn lane_kind_json_format() {
        let json = serde_json::to_string(&LaneKind::UnderReview).expect("serialize");
        assert_eq!(json, r#""under_review""#);

        let json = serde_json::to_string(&LaneKind::InProgress).expect("serialize");
        assert_eq!(json, r#""in_progress""#);
    }

    #[test]
    fn lane_operations() {
        let mut lane = Lane::new(LaneKind::Backlog);
        assert!(lane.is_empty());

        let task = Task::new("Test", "Desc");
        let id = task.id;
        lane.add_task(task);

        assert_eq!(lane.len(), 1);
        assert!(!lane.is_empty());
        assert!(lane.get_task(id).is_some());

        let removed = lane.remove_task(id);
        assert!(removed.is_some());
        assert!(lane.is_empty());
    }

    #[test]
    fn lane_serialization_roundtrip() {
        let mut lane = Lane::new(LaneKind::InProgress);
        lane.add_task(Task::new("Task 1", "Desc 1"));
        lane.add_task(Task::new("Task 2", "Desc 2"));

        let json = serde_json::to_string(&lane).expect("serialize");
        let parsed: Lane = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(lane.kind, parsed.kind);
        assert_eq!(lane.len(), parsed.len());
    }

    #[test]
    fn kanban_board_new_has_four_lanes() {
        let board = KanbanBoard::new();
        assert_eq!(board.lanes.len(), 4);

        for (i, lane) in board.lanes.iter().enumerate() {
            assert_eq!(lane.kind.index(), i);
            assert!(lane.is_empty());
        }
    }

    #[test]
    fn kanban_board_add_and_find_task() {
        let mut board = KanbanBoard::new();
        let task = Task::new("Test", "Description");
        let id = task.id;

        board.add_task(task);

        assert_eq!(board.total_tasks(), 1);
        assert!(board.get_task(id).is_some());
        assert_eq!(board.lane(LaneKind::Backlog).len(), 1);
    }

    #[test]
    fn kanban_board_move_task() {
        let mut board = KanbanBoard::new();
        let task = Task::new("Test", "Description");
        let id = task.id;
        board.add_task(task);

        assert!(board.move_task(id, LaneKind::InProgress));
        assert_eq!(board.lane(LaneKind::Backlog).len(), 0);
        assert_eq!(board.lane(LaneKind::InProgress).len(), 1);

        let task = board.get_task(id).expect("task should exist");
        assert_eq!(task.lane, LaneKind::InProgress);
    }

    #[test]
    fn kanban_board_move_nonexistent_task() {
        let mut board = KanbanBoard::new();
        let fake_id = TaskId::new_v4();

        assert!(!board.move_task(fake_id, LaneKind::Done));
    }

    #[test]
    fn kanban_board_remove_task() {
        let mut board = KanbanBoard::new();
        let task = Task::new("Test", "Description");
        let id = task.id;
        board.add_task(task);

        let removed = board.remove_task(id);
        assert!(removed.is_some());
        assert_eq!(board.total_tasks(), 0);
    }

    #[test]
    fn kanban_board_serialization_roundtrip() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "Desc 1"));
        board.add_task(Task::new("Task 2", "Desc 2"));

        let json = serde_json::to_string(&board).expect("serialize");
        let parsed: KanbanBoard = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(board.total_tasks(), parsed.total_tasks());
    }
}
