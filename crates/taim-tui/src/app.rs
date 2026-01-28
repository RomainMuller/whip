//! Main application struct and run loop.
//!
//! This module provides the `App` struct which orchestrates the TUI
//! application lifecycle including event handling, state updates, and rendering.

use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use taim_protocol::{KanbanBoard, Message};

// Note: render_board is used via self.render_board() wrapper, not directly

use crate::{
    AppState, Focus,
    event::{key_to_message, poll_event},
    terminal::AppTerminal,
    widgets::{render_board, render_detail_panel, render_help_overlay, render_status_bar},
};

/// The main application struct.
///
/// Manages the application state and provides the main event loop.
#[derive(Debug)]
pub struct App {
    state: AppState,
    should_quit: bool,
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
                }
            }
            Message::NavigateDown => {
                if self.state.focus == Focus::Board {
                    self.state.navigate_down();
                } else if self.state.focus == Focus::Detail {
                    self.state.scroll_detail(1);
                }
            }
            Message::Select => {
                self.state.toggle_detail();
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
        }
    }

    /// Renders the application UI to the given frame.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to render into.
    pub fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Board (and detail if visible)
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render board (and optionally detail panel)
        if self.state.detail_visible {
            // Split horizontally: 60% board, 40% detail
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);

            // Render board in left panel
            self.render_board_area(frame, content_chunks[0]);

            // Render detail panel in right panel
            self.render_detail(frame, content_chunks[1]);
        } else {
            // Full board view
            self.render_board_area(frame, chunks[1]);
        }

        // Render footer
        self.render_footer(frame, chunks[2]);

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

            // Poll for events
            if let Some(Event::Key(key)) = poll_event()?
                && let Some(msg) = key_to_message(key)
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

    /// Renders the header bar.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "taim",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled("Kanban Board", Style::default().fg(Color::White)),
        ]))
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(title, area);
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

    /// Renders the footer with key hints.
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let buf = frame.buffer_mut();
        render_status_bar(area, buf);
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
    fn app_select_toggles_detail() {
        let board = KanbanBoard::new();
        let mut app = App::new(board);

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
}
