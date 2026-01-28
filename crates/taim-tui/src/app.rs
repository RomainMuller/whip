//! Main application struct and run loop.
//!
//! This module provides the `App` struct which orchestrates the TUI
//! application lifecycle including event handling, state updates, and rendering.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use taim_protocol::{KanbanBoard, Message};

// Note: render_board is used via self.render_board() wrapper, not directly

use crate::{
    AppState, Focus,
    event::{event_to_message, poll_event},
    terminal::AppTerminal,
    widgets::{
        description_area_dimensions, max_scroll_offset, render_board, render_detail_panel,
        render_help_overlay,
    },
};

/// Height of the header bar in rows.
const HEADER_HEIGHT: u16 = 3;

/// Height of each task card in rows (must match lane.rs).
const TASK_CARD_HEIGHT: u16 = 4;

/// The main application struct.
///
/// Manages the application state and provides the main event loop.
#[derive(Debug)]
pub struct App {
    state: AppState,
    should_quit: bool,
    /// Last known terminal area, used for click hit-testing.
    last_area: Rect,
}

impl App {
    /// Creates a new application with the given Kanban board.
    ///
    /// # Arguments
    ///
    /// * `board` - The initial Kanban board to display.
    ///
    /// # Examples
    ///
    /// ```
    /// use taim_protocol::KanbanBoard;
    /// use taim_tui::App;
    ///
    /// let board = KanbanBoard::new();
    /// let app = App::new(board);
    /// ```
    #[must_use]
    pub fn new(board: KanbanBoard) -> Self {
        Self {
            state: AppState::new(board),
            should_quit: false,
            last_area: Rect::default(),
        }
    }

    /// Returns a reference to the application state.
    #[must_use]
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Updates the application state based on a message.
    ///
    /// When the help overlay is visible, most messages are intercepted to
    /// dismiss the help instead of their normal action. Only `Quit` and
    /// `ToggleHelp` work normally when help is shown.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to process.
    pub fn update(&mut self, msg: Message) {
        // When help is visible, most keys should dismiss it
        if self.state.help_visible {
            match msg {
                Message::Quit => {
                    self.should_quit = true;
                }
                Message::ToggleHelp | Message::Escape => {
                    self.state.toggle_help();
                }
                // Any other key dismisses help
                _ => {
                    let _ = self.state.dismiss_help();
                }
            }
            return;
        }

        match msg {
            Message::Quit => {
                self.should_quit = true;
            }
            Message::Escape => {
                // Contextual escape: close detail panel if open, or clear selection
                if self.state.detail_visible {
                    self.state.toggle_detail();
                } else {
                    self.state.clear_selection();
                }
            }
            Message::NavigateLeft => {
                if self.state.focus == Focus::Board {
                    self.state.navigate_left();
                }
            }
            Message::NavigateRight => {
                if self.state.focus == Focus::Board {
                    self.state.navigate_right();
                }
            }
            Message::NavigateUp => {
                if self.state.focus == Focus::Board {
                    self.state.navigate_up();
                } else if self.state.focus == Focus::Detail {
                    self.state.scroll_detail(-1);
                    self.clamp_scroll_to_content();
                }
            }
            Message::NavigateDown => {
                if self.state.focus == Focus::Board {
                    self.state.navigate_down();
                } else if self.state.focus == Focus::Detail {
                    self.state.scroll_detail(1);
                    self.clamp_scroll_to_content();
                }
            }
            Message::Select => {
                // Only open detail if a task is actually selected
                if self.state.selected_task.is_some() {
                    self.state.toggle_detail();
                }
                // Otherwise do nothing (could ring bell, but simpler to ignore)
            }
            Message::Back => {
                if self.state.detail_visible {
                    self.state.toggle_detail();
                }
            }
            Message::ToggleHelp => {
                self.state.toggle_help();
            }
            Message::Refresh => {
                // TODO: Implement refresh action
            }
            Message::ClickAt { column, row } => {
                self.handle_click(column, row);
            }
        }
    }

    /// Handles a mouse click at the given coordinates.
    ///
    /// If the click is on a task card, selects that task and opens the detail view.
    fn handle_click(&mut self, column: u16, row: u16) {
        // Only handle clicks when on the board view
        if self.state.focus != Focus::Board || self.state.detail_visible {
            return;
        }

        // Compute the board area (content area below header)
        let board_area = Rect {
            x: self.last_area.x,
            y: self.last_area.y + HEADER_HEIGHT,
            width: self.last_area.width,
            height: self.last_area.height.saturating_sub(HEADER_HEIGHT),
        };

        // Check if click is within board area
        if !board_area.contains((column, row).into()) {
            return;
        }

        // Compute which lane was clicked (4 equal columns)
        let lane_width = board_area.width / 4;
        if lane_width == 0 {
            return;
        }
        let relative_x = column.saturating_sub(board_area.x);
        let lane_idx = (relative_x / lane_width).min(3) as usize;

        // Compute which task was clicked within the lane
        // Account for the lane border (1 row for top border)
        let relative_y = row.saturating_sub(board_area.y + 1);
        let task_idx = (relative_y / TASK_CARD_HEIGHT) as usize;

        // Validate the task exists in the clicked lane
        let lane = &self.state.board.lanes[lane_idx];
        if task_idx < lane.len() {
            // Select the lane and task
            self.state.selected_lane = lane_idx;
            self.state.selected_task = Some(task_idx);
            // Open detail view
            self.state.toggle_detail();
        }
    }

    /// Clamps the detail scroll offset to prevent scrolling past content.
    ///
    /// Uses the last known terminal area to compute the maximum valid scroll offset.
    fn clamp_scroll_to_content(&mut self) {
        // Get the selected task; if none, nothing to clamp
        let Some(task) = self.state.selected_task() else {
            return;
        };

        // Compute the detail panel area (content area below header)
        let detail_area = Rect {
            x: self.last_area.x,
            y: self.last_area.y + HEADER_HEIGHT,
            width: self.last_area.width,
            height: self.last_area.height.saturating_sub(HEADER_HEIGHT),
        };

        // Get the description area dimensions
        let Some((visible_height, panel_width)) = description_area_dimensions(task, detail_area)
        else {
            // Area too small, clamp to 0
            self.state.clamp_detail_scroll(0);
            return;
        };

        // Compute and apply the maximum scroll offset
        let max = max_scroll_offset(task, visible_height, panel_width);
        self.state.clamp_detail_scroll(max);
    }

    /// Renders the application UI to the given frame.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to render into.
    pub fn view(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.last_area = area;

        // Create main layout (header + content, no footer)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content area
            ])
            .split(area);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render either board OR detail screen (mutually exclusive views)
        if self.state.detail_visible {
            // Full-screen detail view
            self.render_detail(frame, chunks[1]);
        } else {
            // Full board view
            self.render_board_area(frame, chunks[1]);
        }

        // Render help overlay on top if visible
        if self.state.help_visible {
            let buf = frame.buffer_mut();
            render_help_overlay(area, buf);
        }
    }

    /// Runs the main application loop.
    ///
    /// This function blocks until the user quits the application.
    /// It polls for events, updates state, and renders the UI.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use taim_protocol::KanbanBoard;
    /// use taim_tui::{App, terminal};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut terminal = terminal::setup_terminal()?;
    ///     let mut app = App::new(KanbanBoard::new());
    ///     app.run(&mut terminal).await?;
    ///     terminal::restore_terminal(&mut terminal)?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run(&mut self, terminal: &mut AppTerminal) -> anyhow::Result<()> {
        loop {
            // Render
            terminal.draw(|frame| self.view(frame))?;

            // Poll for events (keyboard and mouse)
            if let Some(event) = poll_event()?
                && let Some(msg) = event_to_message(&event)
            {
                self.update(msg);
            }

            // Check for quit
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Renders the header bar with title and help cue.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        // Create the block first to get inner area (with rounded borders)
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner area: title left, help cue right
        let [title_area, help_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(17), // "Press ? for help" = 16 chars + padding
        ])
        .areas(inner);

        // Render title on left
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "taim",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled("Kanban Board", Style::default().fg(Color::White)),
        ]));
        frame.render_widget(title, title_area);

        // Render help cue on right
        let help_cue = Paragraph::new(Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled(" for help", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Right);
        frame.render_widget(help_cue, help_area);
    }

    /// Renders the Kanban board with four lanes.
    fn render_board_area(&self, frame: &mut Frame, area: Rect) {
        let buf = frame.buffer_mut();
        render_board(
            &self.state.board,
            self.state.selected_lane,
            self.state.selected_task,
            area,
            buf,
        );
    }

    /// Renders the task detail panel.
    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        if let Some(task) = self.state.selected_task() {
            let buf = frame.buffer_mut();
            render_detail_panel(task, self.state.detail_scroll, area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_new_creates_with_board() {
        let board = KanbanBoard::new();
        let app = App::new(board);

        assert!(!app.should_quit);
        assert_eq!(app.state.selected_lane, 0);
    }

    #[test]
    fn app_quit_message_sets_should_quit() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        assert!(!app.should_quit);
        app.update(Message::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn app_navigation_updates_state() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::NavigateRight);
        assert_eq!(app.state.selected_lane, 1);

        app.update(Message::NavigateLeft);
        assert_eq!(app.state.selected_lane, 0);
    }

    #[test]
    fn app_select_does_nothing_without_task() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        assert!(!app.state.detail_visible);
        app.update(Message::Select);
        // Should NOT toggle detail when no task is selected
        assert!(!app.state.detail_visible);
    }

    #[test]
    fn app_select_toggles_detail_with_task() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        app.update(Message::NavigateDown); // Select the task

        assert!(!app.state.detail_visible);
        app.update(Message::Select);
        assert!(app.state.detail_visible);

        app.update(Message::Back);
        assert!(!app.state.detail_visible);
    }

    #[test]
    fn app_toggle_help_shows_and_hides() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        assert!(!app.state.help_visible);

        app.update(Message::ToggleHelp);
        assert!(app.state.help_visible);

        app.update(Message::ToggleHelp);
        assert!(!app.state.help_visible);
    }

    #[test]
    fn app_help_dismisses_on_any_key() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Show help
        app.update(Message::ToggleHelp);
        assert!(app.state.help_visible);

        // Any navigation key should dismiss help
        app.update(Message::NavigateLeft);
        assert!(!app.state.help_visible);
    }

    #[test]
    fn app_help_blocks_navigation() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Start at lane 0
        assert_eq!(app.state.selected_lane, 0);

        // Show help and try to navigate
        app.update(Message::ToggleHelp);
        app.update(Message::NavigateRight);

        // Navigation should be blocked (help dismissed instead)
        assert!(!app.state.help_visible);
        assert_eq!(app.state.selected_lane, 0); // Lane unchanged
    }

    #[test]
    fn app_quit_works_with_help_visible() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::ToggleHelp);
        assert!(app.state.help_visible);

        app.update(Message::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn app_escape_closes_detail_panel() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        app.update(Message::NavigateDown); // Select a task
        app.update(Message::Select); // Open detail panel
        assert!(app.state.detail_visible);

        app.update(Message::Escape);
        assert!(!app.state.detail_visible);
        assert!(!app.should_quit); // Should NOT quit
    }

    #[test]
    fn app_escape_clears_selection_when_no_detail() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        app.update(Message::NavigateDown); // Select a task
        assert!(app.state.selected_task.is_some());

        app.update(Message::Escape);
        assert!(app.state.selected_task.is_none());
        assert!(!app.should_quit); // Should NOT quit
    }

    #[test]
    fn app_escape_dismisses_help() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::ToggleHelp);
        assert!(app.state.help_visible);

        app.update(Message::Escape);
        assert!(!app.state.help_visible);
        assert!(!app.should_quit); // Should NOT quit
    }

    #[test]
    fn app_click_on_task_selects_and_opens_detail() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        // Simulate having rendered with a known area
        app.last_area = Rect::new(0, 0, 80, 24);

        // Click on the first task in the first lane
        // Board starts at row 3 (header height), lane border is row 3, task is row 4+
        // Lane 0 is columns 0-19 (80/4 = 20 width per lane)
        app.update(Message::ClickAt { column: 5, row: 4 });

        assert_eq!(app.state.selected_lane, 0);
        assert_eq!(app.state.selected_task, Some(0));
        assert!(app.state.detail_visible);
    }

    #[test]
    fn app_click_on_different_lane_selects_correct_lane() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));
        // Move task to lane 2 (Under Review)
        board.move_task(
            board.lanes[0].tasks[0].id,
            taim_protocol::LaneKind::UnderReview,
        );

        let mut app = App::new(board);
        app.last_area = Rect::new(0, 0, 80, 24);

        // Click on lane 2 (columns 40-59 for 80-wide terminal)
        app.update(Message::ClickAt { column: 45, row: 4 });

        assert_eq!(app.state.selected_lane, 2);
        assert_eq!(app.state.selected_task, Some(0));
        assert!(app.state.detail_visible);
    }

    #[test]
    fn app_click_on_empty_lane_does_nothing() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);
        app.last_area = Rect::new(0, 0, 80, 24);

        app.update(Message::ClickAt { column: 5, row: 4 });

        assert!(!app.state.detail_visible);
    }

    #[test]
    fn app_click_outside_board_does_nothing() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        app.last_area = Rect::new(0, 0, 80, 24);

        // Click on header (row 0-2)
        app.update(Message::ClickAt { column: 5, row: 1 });

        assert!(!app.state.detail_visible);
    }

    #[test]
    fn app_click_ignored_when_detail_visible() {
        let mut board = KanbanBoard::new();
        board.add_task(taim_protocol::Task::new("Task 1", "Description"));
        board.add_task(taim_protocol::Task::new("Task 2", "Description"));

        let mut app = App::new(board);
        app.last_area = Rect::new(0, 0, 80, 24);

        // Open detail on first task
        app.update(Message::NavigateDown);
        app.update(Message::Select);
        assert!(app.state.detail_visible);
        assert_eq!(app.state.selected_task, Some(0));

        // Try clicking on second task - should be ignored
        app.update(Message::ClickAt { column: 5, row: 8 });
        assert_eq!(app.state.selected_task, Some(0)); // Still first task
    }

    #[test]
    fn app_scroll_clamped_to_content_bounds() {
        let mut board = KanbanBoard::new();
        // Create a task with a short description that won't require scrolling
        board.add_task(taim_protocol::Task::new("Task 1", "Short description"));

        let mut app = App::new(board);
        // Simulate a reasonably sized terminal
        app.last_area = Rect::new(0, 0, 80, 24);

        // Open detail panel
        app.update(Message::NavigateDown);
        app.update(Message::Select);
        assert!(app.state.detail_visible);
        assert_eq!(app.state.focus, Focus::Detail);

        // Try to scroll down many times - should be clamped
        for _ in 0..100 {
            app.update(Message::NavigateDown);
        }

        // With a short description and a 24-line terminal, the max scroll should be 0
        // The scroll should be clamped to 0 (no scrolling needed)
        assert_eq!(
            app.state.detail_scroll, 0,
            "Scroll should be clamped to 0 for short content"
        );
    }

    #[test]
    fn app_scroll_allows_scrolling_long_content() {
        let mut board = KanbanBoard::new();
        // Create a task with a very long description that will need scrolling
        let long_description = "This is a very long description. ".repeat(50);
        board.add_task(taim_protocol::Task::new("Task 1", &long_description));

        let mut app = App::new(board);
        // Simulate a reasonably sized terminal
        app.last_area = Rect::new(0, 0, 80, 24);

        // Open detail panel
        app.update(Message::NavigateDown);
        app.update(Message::Select);
        assert!(app.state.detail_visible);

        // Scroll down a few times
        app.update(Message::NavigateDown);
        app.update(Message::NavigateDown);
        app.update(Message::NavigateDown);

        // With long content, we should be able to scroll
        assert!(
            app.state.detail_scroll > 0,
            "Should be able to scroll with long content"
        );

        // Try to scroll way past the content
        for _ in 0..1000 {
            app.update(Message::NavigateDown);
        }

        // The scroll should be clamped to the maximum valid value
        // For this test, we just verify it's not absurdly high
        assert!(
            app.state.detail_scroll < 1000,
            "Scroll should be clamped to a reasonable max, got {}",
            app.state.detail_scroll
        );
    }
}
