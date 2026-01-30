//! whip - An AI Agent orchestrator using Claude Code.
//!
//! This is the main binary that launches the TUI application.

use whip_config::Config;
use whip_protocol::dummy::dummy_board;
use whip_tui::{App, terminal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load().await.unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        Config::default()
    });

    // Install panic hook to restore terminal on panic
    terminal::install_panic_hook();

    // Setup terminal
    let mut terminal = terminal::setup_terminal()?;

    // Create app with dummy board and loaded config
    let board = dummy_board();
    let mut app = App::with_config(board, config);

    // Run the main loop
    let result = app.run(&mut terminal).await;

    // Always restore terminal, even if app.run() failed
    terminal::restore_terminal(&mut terminal)?;

    result
}
