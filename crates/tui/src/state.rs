//! Application state management.
//!
//! This module defines the core state structures for the TUI application,
//! including focus management and selection tracking.

use whip_protocol::{KanbanBoard, Lane, Task};

/// The current focus area in the UI.
///
/// Determines which UI component receives keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// Focus is on the Kanban board lanes.
    #[default]
    Board,
    /// Focus is on the task detail panel.
    Detail,
    /// Focus is on the settings panel.
    Settings,
}

/// The application state.
///
/// Contains all mutable state for the TUI application including
/// the board data, focus state, and selection tracking.
#[derive(Debug, Clone)]
pub struct AppState {
    /// The Kanban board being displayed.
    pub board: KanbanBoard,
    /// Current focus area.
    pub focus: Focus,
    /// Index of the currently selected lane (0-3).
    pub selected_lane: usize,
    /// Index of the selected task within the current lane, if any.
    pub selected_task: Option<usize>,
    /// Whether the detail panel is visible.
    pub detail_visible: bool,
    /// Scroll offset for the detail panel description.
    pub detail_scroll: u16,
    /// Whether the help overlay is visible.
    pub help_visible: bool,
}

impl AppState {
    /// Returns a reference to the currently selected lane.
    ///
    /// # Panics
    ///
    /// Panics if `selected_lane` is out of bounds. This should never occur
    /// if navigation methods are used correctly, as they maintain the invariant
    /// that `selected_lane` is always in the range `0..4`.
    fn selected_lane_ref(&self) -> &Lane {
        self.board
            .lanes
            .get(self.selected_lane)
            .expect("selected_lane should always be in bounds (0..4)")
    }

    /// Returns a mutable reference to the currently selected lane.
    ///
    /// # Panics
    ///
    /// Panics if `selected_lane` is out of bounds. This should never occur
    /// if navigation methods are used correctly.
    #[allow(dead_code)] // Provided for future use and API symmetry
    fn selected_lane_mut(&mut self) -> &mut Lane {
        self.board
            .lanes
            .get_mut(self.selected_lane)
            .expect("selected_lane should always be in bounds (0..4)")
    }

    /// Creates a new application state with the given board.
    ///
    /// Initializes with focus on the board, selecting the first lane.
    ///
    /// # Arguments
    ///
    /// * `board` - The Kanban board to display.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::KanbanBoard;
    /// use whip_tui::AppState;
    ///
    /// let board = KanbanBoard::new();
    /// let state = AppState::new(board);
    /// assert_eq!(state.selected_lane, 0);
    /// ```
    #[must_use]
    pub fn new(board: KanbanBoard) -> Self {
        Self {
            board,
            focus: Focus::default(),
            selected_lane: 0,
            selected_task: None,
            detail_visible: false,
            detail_scroll: 0,
            help_visible: false,
        }
    }

    /// Toggles the help overlay visibility.
    ///
    /// When help is shown, other interactions are blocked until
    /// help is dismissed.
    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    /// Dismisses the help overlay if it is visible.
    ///
    /// Returns `true` if help was visible and has been dismissed,
    /// `false` if help was not visible.
    #[must_use]
    pub fn dismiss_help(&mut self) -> bool {
        if self.help_visible {
            self.help_visible = false;
            true
        } else {
            false
        }
    }

    /// Moves the lane selection to the left, wrapping around if needed.
    pub fn navigate_left(&mut self) {
        if self.selected_lane > 0 {
            self.selected_lane -= 1;
        } else {
            self.selected_lane = 3; // Wrap to last lane
        }
        self.clamp_task_selection();
    }

    /// Moves the lane selection to the right, wrapping around if needed.
    pub fn navigate_right(&mut self) {
        if self.selected_lane < 3 {
            self.selected_lane += 1;
        } else {
            self.selected_lane = 0; // Wrap to first lane
        }
        self.clamp_task_selection();
    }

    /// Moves the task selection up within the current lane.
    pub fn navigate_up(&mut self) {
        let lane = self.selected_lane_ref();
        if lane.is_empty() {
            self.selected_task = None;
            return;
        }

        match self.selected_task {
            Some(idx) if idx > 0 => {
                self.selected_task = Some(idx - 1);
            }
            Some(_) => {
                // Wrap to bottom
                self.selected_task = Some(lane.len().saturating_sub(1));
            }
            None => {
                // Select first task
                self.selected_task = Some(0);
            }
        }
    }

    /// Moves the task selection down within the current lane.
    pub fn navigate_down(&mut self) {
        let lane = self.selected_lane_ref();
        if lane.is_empty() {
            self.selected_task = None;
            return;
        }

        let max_idx = lane.len().saturating_sub(1);
        match self.selected_task {
            Some(idx) if idx < max_idx => {
                self.selected_task = Some(idx + 1);
            }
            Some(_) => {
                // Wrap to top
                self.selected_task = Some(0);
            }
            None => {
                // Select first task
                self.selected_task = Some(0);
            }
        }
    }

    /// Toggles the detail panel visibility.
    pub fn toggle_detail(&mut self) {
        self.detail_visible = !self.detail_visible;
        self.focus = if self.detail_visible {
            Focus::Detail
        } else {
            Focus::Board
        };
        // Reset scroll when opening/closing
        self.detail_scroll = 0;
    }

    /// Scrolls the detail panel by the given delta.
    ///
    /// Positive delta scrolls down, negative scrolls up.
    /// The scroll offset is clamped to prevent underflow (scrolling above content).
    ///
    /// Note: To prevent scrolling past the end of content, call `clamp_detail_scroll()`
    /// after this method with the appropriate maximum value computed from
    /// `max_scroll_offset()`.
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to scroll (positive = down, negative = up)
    pub fn scroll_detail(&mut self, delta: i16) {
        if delta > 0 {
            self.detail_scroll = self.detail_scroll.saturating_add(delta as u16);
        } else {
            self.detail_scroll = self.detail_scroll.saturating_sub(delta.unsigned_abs());
        }
    }

    /// Clamps the detail scroll offset to a maximum value.
    ///
    /// Call this after scroll operations with the result of `max_scroll_offset()`
    /// to prevent scrolling past the end of the content.
    ///
    /// # Arguments
    ///
    /// * `max` - The maximum valid scroll offset
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::KanbanBoard;
    /// use whip_tui::AppState;
    ///
    /// let board = KanbanBoard::new();
    /// let mut state = AppState::new(board);
    ///
    /// state.scroll_detail(100); // Scroll way down
    /// state.clamp_detail_scroll(5); // Clamp to max of 5
    /// assert_eq!(state.detail_scroll, 5);
    /// ```
    pub fn clamp_detail_scroll(&mut self, max: u16) {
        self.detail_scroll = self.detail_scroll.min(max);
    }

    /// Returns a reference to the currently selected task, if any.
    ///
    /// Returns `None` if no task is selected or if the selection is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{KanbanBoard, Task};
    /// use whip_tui::AppState;
    ///
    /// let mut board = KanbanBoard::new();
    /// board.add_task(Task::new("Task 1", "Description"));
    ///
    /// let mut state = AppState::new(board);
    /// assert!(state.selected_task().is_none());
    ///
    /// state.navigate_down(); // Select first task
    /// assert!(state.selected_task().is_some());
    /// ```
    #[must_use]
    pub fn selected_task(&self) -> Option<&Task> {
        let task_idx = self.selected_task?;
        let lane = self.selected_lane_ref();
        lane.tasks.get(task_idx)
    }

    /// Clears the current task selection.
    ///
    /// After calling this, `selected_task` will be `None`.
    pub fn clear_selection(&mut self) {
        self.selected_task = None;
    }

    /// Ensures the task selection is valid for the current lane.
    fn clamp_task_selection(&mut self) {
        let lane = self.selected_lane_ref();
        if lane.is_empty() {
            self.selected_task = None;
        } else if let Some(idx) = self.selected_task
            && idx >= lane.len()
        {
            self.selected_task = Some(lane.len().saturating_sub(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use whip_protocol::Task;

    #[test]
    fn new_state_has_correct_defaults() {
        let board = KanbanBoard::new();
        let state = AppState::new(board);

        assert_eq!(state.focus, Focus::Board);
        assert_eq!(state.selected_lane, 0);
        assert_eq!(state.selected_task, None);
        assert!(!state.detail_visible);
        assert_eq!(state.detail_scroll, 0);
        assert!(!state.help_visible);
    }

    #[test]
    fn navigate_left_wraps_around() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.navigate_left();
        assert_eq!(state.selected_lane, 3);

        state.navigate_left();
        assert_eq!(state.selected_lane, 2);
    }

    #[test]
    fn navigate_right_wraps_around() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.selected_lane = 3;
        state.navigate_right();
        assert_eq!(state.selected_lane, 0);
    }

    #[test]
    fn navigate_up_down_in_empty_lane() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.navigate_up();
        assert_eq!(state.selected_task, None);

        state.navigate_down();
        assert_eq!(state.selected_task, None);
    }

    #[test]
    fn navigate_up_down_with_tasks() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "Desc 1"));
        board.add_task(Task::new("Task 2", "Desc 2"));
        board.add_task(Task::new("Task 3", "Desc 3"));

        let mut state = AppState::new(board);

        // Start navigating
        state.navigate_down();
        assert_eq!(state.selected_task, Some(0));

        state.navigate_down();
        assert_eq!(state.selected_task, Some(1));

        state.navigate_down();
        assert_eq!(state.selected_task, Some(2));

        // Wrap around
        state.navigate_down();
        assert_eq!(state.selected_task, Some(0));

        // Navigate up from top wraps to bottom
        state.navigate_up();
        assert_eq!(state.selected_task, Some(2));
    }

    #[test]
    fn toggle_detail_changes_focus() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        assert_eq!(state.focus, Focus::Board);
        assert!(!state.detail_visible);

        state.toggle_detail();
        assert_eq!(state.focus, Focus::Detail);
        assert!(state.detail_visible);

        state.toggle_detail();
        assert_eq!(state.focus, Focus::Board);
        assert!(!state.detail_visible);
    }

    #[test]
    fn toggle_detail_resets_scroll() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 10;
        state.toggle_detail();
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn scroll_detail_positive() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.scroll_detail(5);
        assert_eq!(state.detail_scroll, 5);

        state.scroll_detail(3);
        assert_eq!(state.detail_scroll, 8);
    }

    #[test]
    fn scroll_detail_negative() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 10;
        state.scroll_detail(-3);
        assert_eq!(state.detail_scroll, 7);
    }

    #[test]
    fn scroll_detail_does_not_underflow() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 5;
        state.scroll_detail(-10);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn clamp_detail_scroll_reduces_to_max() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 100;
        state.clamp_detail_scroll(10);
        assert_eq!(state.detail_scroll, 10);
    }

    #[test]
    fn clamp_detail_scroll_does_not_increase() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 5;
        state.clamp_detail_scroll(100);
        assert_eq!(state.detail_scroll, 5);
    }

    #[test]
    fn clamp_detail_scroll_with_zero_max() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.detail_scroll = 10;
        state.clamp_detail_scroll(0);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn selected_task_returns_none_when_no_selection() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "Description"));

        let state = AppState::new(board);
        assert!(state.selected_task().is_none());
    }

    #[test]
    fn selected_task_returns_task_when_selected() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "Description 1"));
        board.add_task(Task::new("Task 2", "Description 2"));

        let mut state = AppState::new(board);
        state.navigate_down(); // Select first task

        let task = state.selected_task().expect("should have selected task");
        assert_eq!(task.title, "Task 1");
    }

    #[test]
    fn selected_task_returns_none_for_empty_lane() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);
        state.selected_task = Some(0); // Manually set invalid selection

        assert!(state.selected_task().is_none());
    }

    #[test]
    fn toggle_help_visibility() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        assert!(!state.help_visible);

        state.toggle_help();
        assert!(state.help_visible);

        state.toggle_help();
        assert!(!state.help_visible);
    }

    #[test]
    fn dismiss_help_when_visible() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        state.help_visible = true;
        let dismissed = state.dismiss_help();

        assert!(dismissed);
        assert!(!state.help_visible);
    }

    #[test]
    fn dismiss_help_when_not_visible() {
        let board = KanbanBoard::new();
        let mut state = AppState::new(board);

        assert!(!state.help_visible);
        let dismissed = state.dismiss_help();

        assert!(!dismissed);
        assert!(!state.help_visible);
    }

    #[test]
    fn clear_selection_removes_task_selection() {
        let mut board = KanbanBoard::new();
        board.add_task(Task::new("Task 1", "Description"));

        let mut state = AppState::new(board);
        state.navigate_down(); // Select first task
        assert!(state.selected_task.is_some());

        state.clear_selection();
        assert!(state.selected_task.is_none());
    }
}
