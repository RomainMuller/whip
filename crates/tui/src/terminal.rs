//! Terminal setup and teardown utilities.
//!
//! This module provides functions for initializing and restoring the terminal
//! state, as well as installing a panic hook that ensures the terminal is
//! properly restored on panic.

use std::io::{self, Stdout};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

/// The terminal type used by the application.
pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Error type for terminal operations.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    /// Failed to initialize the terminal.
    #[error("failed to setup terminal: {0}")]
    Setup(#[source] io::Error),

    /// Failed to restore the terminal.
    #[error("failed to restore terminal: {0}")]
    Restore(#[source] io::Error),
}

/// Sets up the terminal for TUI rendering.
///
/// This function:
/// - Enables raw mode (disables line buffering and echoing)
/// - Enters the alternate screen buffer
/// - Creates a Ratatui terminal instance
///
/// # Errors
///
/// Returns an error if any terminal operation fails.
///
/// # Examples
///
/// ```no_run
/// use whip_tui::terminal;
///
/// let mut terminal = terminal::setup_terminal().expect("failed to setup terminal");
/// // Use terminal...
/// terminal::restore_terminal(&mut terminal).expect("failed to restore terminal");
/// ```
pub fn setup_terminal() -> Result<AppTerminal, TerminalError> {
    enable_raw_mode().map_err(TerminalError::Setup)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(TerminalError::Setup)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(TerminalError::Setup)
}

/// Restores the terminal to its original state.
///
/// This function:
/// - Disables raw mode
/// - Leaves the alternate screen buffer
/// - Shows the cursor
///
/// # Errors
///
/// Returns an error if any terminal operation fails.
///
/// # Examples
///
/// ```no_run
/// use whip_tui::terminal;
///
/// let mut terminal = terminal::setup_terminal().expect("failed to setup terminal");
/// // Use terminal...
/// terminal::restore_terminal(&mut terminal).expect("failed to restore terminal");
/// ```
pub fn restore_terminal(terminal: &mut AppTerminal) -> Result<(), TerminalError> {
    disable_raw_mode().map_err(TerminalError::Restore)?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )
    .map_err(TerminalError::Restore)?;
    terminal.show_cursor().map_err(TerminalError::Restore)?;
    Ok(())
}

/// Installs a panic hook that restores the terminal before panicking.
///
/// This ensures that if the application panics, the terminal is left in a
/// usable state (not in raw mode, cursor visible, main screen buffer).
///
/// # Panic Hook Chaining
///
/// This function replaces the current panic hook but chains to it after
/// performing terminal restoration. The original hook (captured at the time
/// of calling this function) will still execute after terminal cleanup.
///
/// **Important**: Call this function once at application startup, before
/// setting up the terminal and before any other crates install their own
/// panic hooks. Calling multiple times or after other hooks are installed
/// may result in unexpected hook chain behavior.
///
/// # Order of Operations
///
/// When a panic occurs, the installed hook:
/// 1. Disables raw mode (restores line buffering and echo)
/// 2. Disables mouse capture
/// 3. Leaves the alternate screen buffer
/// 4. Calls the original panic hook (typically prints the panic message)
///
/// # Examples
///
/// ```no_run
/// use whip_tui::terminal;
///
/// fn main() {
///     // Install panic hook FIRST, before any terminal setup
///     terminal::install_panic_hook();
///
///     let mut terminal = terminal::setup_terminal()
///         .expect("failed to setup terminal");
///     // Application code...
/// }
/// ```
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort terminal restoration
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
