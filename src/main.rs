//! taim - An AI Agent orchestrator using Claude Code.
//!
//! This is the main binary that launches the TUI application.

use taim_protocol::dummy::dummy_board;
use taim_tui::{App, terminal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install panic hook to restore terminal on panic
    terminal::install_panic_hook();

    // Setup terminal
    let mut terminal = terminal::setup_terminal()?;

    // Create app with dummy board
    let board = dummy_board();
    let mut app = App::new(board);

    // Run the main loop
    let result = app.run(&mut terminal).await;

    // Always restore terminal, even if app.run() failed
    terminal::restore_terminal(&mut terminal)?;

    result
}
