//! Terminal UI for the whip application.
//!
//! This crate provides a Ratatui-based terminal interface for managing
//! and visualizing the Kanban board.
//!
//! # Overview
//!
//! The crate is organized into the following modules:
//!
//! - [`app`]: Main application struct and run loop
//! - [`state`]: Application state management
//! - [`settings_state`]: Settings panel state management
//! - [`terminal`]: Terminal setup, teardown, and panic handling
//! - [`event`]: Event handling and key mappings
//!
//! # Example
//!
//! ```no_run
//! use whip_protocol::KanbanBoard;
//! use whip_tui::{App, terminal};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     terminal::install_panic_hook();
//!     let mut terminal = terminal::setup_terminal()?;
//!
//!     let board = KanbanBoard::new();
//!     let mut app = App::new(board);
//!     let result = app.run(&mut terminal).await;
//!
//!     terminal::restore_terminal(&mut terminal)?;
//!     result
//! }
//! ```

pub mod app;
pub mod event;
pub mod layout;
pub mod settings_state;
pub mod state;
pub mod terminal;
pub mod widgets;

#[cfg(test)]
pub(crate) mod test_utils;

// Re-export primary types at crate root for convenience
pub use app::App;
pub use state::{AppState, Focus};
