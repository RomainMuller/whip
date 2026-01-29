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
use whip_config::Config;
use whip_protocol::{KanbanBoard, Message};

// Note: render_board is used via self.render_board() wrapper, not directly

use crate::{
    AppState, Focus,
    event::{event_to_message, key_to_settings_message, poll_event},
    layout::{HEADER_HEIGHT, MIN_HEIGHT, MIN_HEIGHT_WITH_HEADER, MIN_WIDTH, TASK_CARD_HEIGHT},
    settings_state::SettingsState,
    terminal::AppTerminal,
    widgets::{
        description_area_dimensions, max_scroll_offset, render_board, render_detail_panel,
        render_help_overlay, render_settings_panel,
    },
};

/// The main application struct.
///
/// Manages the application state and provides the main event loop.
#[derive(Debug)]
pub struct App {
    state: AppState,
    should_quit: bool,
    /// Last known terminal area, used for click hit-testing.
    last_area: Rect,
    /// Whether the header was shown in the last render (affects click hit-testing).
    header_visible: bool,
    /// Settings panel state, if open.
    settings_state: Option<SettingsState>,
    /// The application configuration.
    config: Config,
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
    /// use whip_protocol::KanbanBoard;
    /// use whip_tui::App;
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
            header_visible: true,
            settings_state: None,
            config: Config::default(),
        }
    }

    /// Creates a new application with the given Kanban board and configuration.
    ///
    /// # Arguments
    ///
    /// * `board` - The initial Kanban board to display.
    /// * `config` - The application configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Config;
    /// use whip_protocol::KanbanBoard;
    /// use whip_tui::App;
    ///
    /// let board = KanbanBoard::new();
    /// let config = Config::default();
    /// let app = App::with_config(board, config);
    /// ```
    #[must_use]
    pub fn with_config(board: KanbanBoard, config: Config) -> Self {
        Self {
            state: AppState::new(board),
            should_quit: false,
            last_area: Rect::default(),
            header_visible: true,
            settings_state: None,
            config,
        }
    }

    /// Returns a reference to the application state.
    #[must_use]
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Returns a reference to the application configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns whether the settings panel is open.
    #[must_use]
    pub fn is_settings_open(&self) -> bool {
        self.settings_state.is_some()
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
        // Handle settings-specific messages when settings panel is open
        if let Some(ref mut settings) = self.settings_state {
            match msg {
                Message::Quit => {
                    self.should_quit = true;
                }
                Message::CloseSettings | Message::Escape if !settings.is_editing() => {
                    // Close settings, potentially applying changes
                    if let Some(settings) = self.settings_state.take() {
                        self.config = settings.into_config();
                    }
                    self.state.focus = Focus::Board;
                }
                Message::SettingsNextSection => {
                    settings.next_section();
                }
                Message::SettingsPrevSection => {
                    settings.prev_section();
                }
                Message::SettingsNavigate { delta } => {
                    settings.navigate(delta);
                }
                Message::SettingsEdit => {
                    settings.start_edit();
                }
                Message::SettingsConfirm => {
                    settings.confirm_edit();
                }
                Message::SettingsCancel | Message::Escape => {
                    settings.cancel_edit();
                }
                Message::SettingsDelete => {
                    let _ = settings.delete_selected();
                }
                Message::SettingsSave => {
                    // Save config to file
                    if let Ok(path) = whip_config::persistence::default_user_config_path() {
                        let _ = settings.config().save_to(&path);
                        settings.mark_saved();
                    }
                }
                Message::SettingsInput { ch } => {
                    settings.input_char(ch);
                }
                Message::SettingsBackspace => {
                    settings.backspace();
                }
                // Toggle settings item (for checkboxes)
                Message::Select if !settings.is_editing() => {
                    settings.toggle_selected();
                }
                _ => {}
            }
            return;
        }

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
            Message::OpenSettings => {
                self.settings_state = Some(SettingsState::new(self.config.clone()));
                self.state.focus = Focus::Settings;
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
            // Settings messages are handled above when settings is open
            _ => {}
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

        // Compute the board area (content area below header, if visible)
        let header_offset = if self.header_visible {
            HEADER_HEIGHT
        } else {
            0
        };
        let board_area = Rect {
            x: self.last_area.x,
            y: self.last_area.y + header_offset,
            width: self.last_area.width,
            height: self.last_area.height.saturating_sub(header_offset),
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
        let Some(lane) = self.state.board.lanes.get(lane_idx) else {
            return;
        };
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

        // Compute the detail panel area (content area below header, if visible)
        let header_offset = if self.header_visible {
            HEADER_HEIGHT
        } else {
            0
        };
        let detail_area = Rect {
            x: self.last_area.x,
            y: self.last_area.y + header_offset,
            width: self.last_area.width,
            height: self.last_area.height.saturating_sub(header_offset),
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
    /// Implements graceful degradation for small terminal sizes:
    /// - If terminal is below minimum dimensions, shows a "terminal too small" message.
    /// - If terminal is tight (below `MIN_HEIGHT_WITH_HEADER`), hides the header to reclaim space.
    /// - Otherwise, renders normally with header.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to render into.
    pub fn view(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.last_area = area;

        // Check if terminal is too small for any useful rendering
        if area.height < MIN_HEIGHT || area.width < MIN_WIDTH {
            self.header_visible = false;
            self.render_terminal_too_small(frame, area);
            return;
        }

        // Determine if we should show header (compact mode hides it to reclaim space)
        let show_header = area.height >= MIN_HEIGHT_WITH_HEADER;
        self.header_visible = show_header;

        // Create layout based on header visibility
        let content_area = if show_header {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(HEADER_HEIGHT), // Header
                    Constraint::Min(0),                // Content area
                ])
                .split(area);

            // Render header
            self.render_header(frame, chunks[0]);
            chunks[1]
        } else {
            // No header - full area is content
            area
        };

        // Render either board OR detail screen (mutually exclusive views)
        if self.state.detail_visible {
            self.render_detail(frame, content_area);
        } else {
            self.render_board_area(frame, content_area);
        }

        // Render help overlay on top if visible
        if self.state.help_visible {
            let buf = frame.buffer_mut();
            render_help_overlay(area, buf);
        }

        // Render settings panel overlay on top if open
        if let Some(ref settings) = self.settings_state {
            let buf = frame.buffer_mut();
            render_settings_panel(settings, area, buf);
        }
    }

    /// Renders a message indicating the terminal is too small.
    fn render_terminal_too_small(&self, frame: &mut Frame, area: Rect) {
        let message = format!(
            "Terminal too small ({}×{})\nMinimum: {}×{} (w×h)",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT
        );

        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: false });

        // Center the message vertically
        let vertical_offset = area.height.saturating_sub(2) / 2;
        let centered_area = Rect {
            x: area.x,
            y: area.y + vertical_offset,
            width: area.width,
            height: area.height.saturating_sub(vertical_offset),
        };

        frame.render_widget(paragraph, centered_area);
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
    /// use whip_protocol::KanbanBoard;
    /// use whip_tui::{App, terminal};
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
        use crossterm::event::Event;

        loop {
            // Render
            terminal.draw(|frame| self.view(frame))?;

            // Poll for events (keyboard and mouse)
            if let Some(event) = poll_event()? {
                // Handle key events differently when settings is open
                let msg = if self.settings_state.is_some() {
                    if let Event::Key(key) = event {
                        let is_editing =
                            self.settings_state.as_ref().is_some_and(|s| s.is_editing());
                        key_to_settings_message(key, is_editing)
                    } else {
                        event_to_message(&event)
                    }
                } else {
                    event_to_message(&event)
                };

                if let Some(msg) = msg {
                    self.update(msg);
                }
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
                "whip",
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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));
        // Move task to lane 2 (Under Review)
        board.move_task(
            board.lanes[0].tasks[0].id,
            whip_protocol::LaneKind::UnderReview,
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
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        app.last_area = Rect::new(0, 0, 80, 24);

        // Click on header (row 0-2)
        app.update(Message::ClickAt { column: 5, row: 1 });

        assert!(!app.state.detail_visible);
    }

    #[test]
    fn app_click_ignored_when_detail_visible() {
        let mut board = KanbanBoard::new();
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));
        board.add_task(whip_protocol::Task::new("Task 2", "Description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", "Short description"));

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
        board.add_task(whip_protocol::Task::new("Task 1", &long_description));

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

    // --- Graceful degradation tests ---

    #[test]
    fn app_view_shows_too_small_message_when_height_below_minimum() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Create a terminal with height below MIN_HEIGHT (10)
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.view(frame)).unwrap();

        // Verify header is not visible in this mode
        assert!(!app.header_visible);

        // Check that the buffer contains the "too small" message
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect::<String>();
        assert!(
            content.contains("Terminal too small"),
            "Buffer should contain 'Terminal too small' message"
        );
    }

    #[test]
    fn app_view_shows_too_small_message_when_width_below_minimum() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Create a terminal with width below MIN_WIDTH (40)
        let backend = TestBackend::new(30, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.view(frame)).unwrap();

        assert!(!app.header_visible);

        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect::<String>();
        assert!(
            content.contains("Terminal too small"),
            "Buffer should contain 'Terminal too small' message"
        );
    }

    #[test]
    fn app_view_hides_header_in_compact_mode() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Create a terminal with height at MIN_HEIGHT but below MIN_HEIGHT_WITH_HEADER
        // MIN_HEIGHT = 10, MIN_HEIGHT_WITH_HEADER = 13
        let backend = TestBackend::new(80, 11);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.view(frame)).unwrap();

        // Header should be hidden in compact mode
        assert!(!app.header_visible);

        // But we should still see the board content (lane names)
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect::<String>();
        assert!(
            content.contains("Backlog"),
            "Buffer should contain board content"
        );
    }

    #[test]
    fn app_view_shows_header_when_terminal_large_enough() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let board = KanbanBoard::new();
        let mut app = App::new(board);

        // Create a terminal with height at or above MIN_HEIGHT_WITH_HEADER (13)
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.view(frame)).unwrap();

        // Header should be visible
        assert!(app.header_visible);

        // Check that the buffer contains the header title
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect::<String>();
        assert!(
            content.contains("whip"),
            "Buffer should contain header title"
        );
        assert!(
            content.contains("Backlog"),
            "Buffer should contain board content"
        );
    }

    #[test]
    fn app_click_works_in_compact_mode() {
        let mut board = KanbanBoard::new();
        board.add_task(whip_protocol::Task::new("Task 1", "Description"));

        let mut app = App::new(board);
        // Simulate compact mode (header not visible, height between MIN_HEIGHT and MIN_HEIGHT_WITH_HEADER)
        app.last_area = Rect::new(0, 0, 80, 11);
        app.header_visible = false;

        // In compact mode, board starts at row 0 (no header)
        // Lane border is row 0, task starts at row 1
        app.update(Message::ClickAt { column: 5, row: 1 });

        assert_eq!(app.state.selected_lane, 0);
        assert_eq!(app.state.selected_task, Some(0));
        assert!(app.state.detail_visible);
    }

    #[test]
    fn app_view_renders_detail_when_large_enough() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut board = KanbanBoard::new();
        board.add_task(whip_protocol::Task::new("My Test Task", "Task description"));

        let mut app = App::new(board);
        // Select task and open detail
        app.update(Message::NavigateDown);
        app.update(Message::Select);
        assert!(app.state.detail_visible);

        // Create a terminal large enough for detail panel with header
        // MIN_HEIGHT_WITH_HEADER = 13
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| app.view(frame)).unwrap();

        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect::<String>();

        // Should render the detail panel with task title
        assert!(
            content.contains("My Test Task"),
            "Buffer should contain the task title"
        );
        // Should NOT show the "too small" message
        assert!(
            !content.contains("too small"),
            "Buffer should not contain 'too small' message"
        );
    }

    // --- Settings panel tests ---

    #[test]
    fn app_open_settings() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        assert!(!app.is_settings_open());

        app.update(Message::OpenSettings);
        assert!(app.is_settings_open());
        assert_eq!(app.state.focus, Focus::Settings);
    }

    #[test]
    fn app_close_settings() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::OpenSettings);
        assert!(app.is_settings_open());

        app.update(Message::CloseSettings);
        assert!(!app.is_settings_open());
        assert_eq!(app.state.focus, Focus::Board);
    }

    #[test]
    fn app_settings_navigation() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::OpenSettings);

        // Navigate sections
        app.update(Message::SettingsNextSection);
        if let Some(ref state) = app.settings_state {
            assert_eq!(
                state.section(),
                crate::settings_state::SettingsSection::Polling
            );
        }

        app.update(Message::SettingsPrevSection);
        if let Some(ref state) = app.settings_state {
            assert_eq!(
                state.section(),
                crate::settings_state::SettingsSection::Repositories
            );
        }
    }

    #[test]
    fn app_settings_escape_closes() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

        app.update(Message::OpenSettings);
        assert!(app.is_settings_open());

        app.update(Message::Escape);
        assert!(!app.is_settings_open());
    }

    #[test]
    fn app_with_config() {
        let board = KanbanBoard::new();
        let config = Config {
            github_token: Some("test_token".to_string()),
            ..Default::default()
        };

        let app = App::with_config(board, config);
        assert_eq!(app.config().github_token, Some("test_token".to_string()));
    }

    #[test]
    fn app_settings_preserves_config_on_close() {
        use whip_config::Repository;

        let board = KanbanBoard::new();
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut app = App::with_config(board, config);

        // Open settings, make a change, close
        app.update(Message::OpenSettings);

        // Delete the repository
        app.update(Message::SettingsDelete);

        // Close settings
        app.update(Message::CloseSettings);

        // Config should be updated
        assert!(app.config().repositories.is_empty());
    }
}
