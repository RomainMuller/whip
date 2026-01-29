//! Centralized layout measurements for the TUI.
//!
//! This module defines shared constants for layout dimensions used across
//! multiple rendering components. Centralizing these values ensures consistency
//! and makes it easier to adjust the UI proportions.

/// Height of the header bar in rows.
///
/// The header displays the application title and help cue.
pub const HEADER_HEIGHT: u16 = 3;

/// Height of each task card in rows.
///
/// This includes the border (2 rows) and content (2 rows for title and description).
pub const TASK_CARD_HEIGHT: u16 = 4;

/// Minimum terminal height for useful rendering (content area).
///
/// Below this height, we display a "terminal too small" message.
/// This is set to support all app features, including the detail panel which
/// requires the most vertical space:
/// - Borders: 2 rows
/// - Metadata: 1-3 rows
/// - Separators: 2 rows
/// - Description: at least 3 rows
/// - Footer: 1 row
pub const MIN_HEIGHT: u16 = 10;

/// Minimum terminal height for rendering with header.
///
/// When terminal height is between `MIN_HEIGHT` and `MIN_HEIGHT_WITH_HEADER`,
/// we hide the header to reclaim 3 rows of content space.
pub const MIN_HEIGHT_WITH_HEADER: u16 = MIN_HEIGHT + HEADER_HEIGHT;

/// Minimum terminal width for useful rendering.
///
/// The board has 4 lanes; each lane needs at least 10 characters
/// for borders and truncated titles to be readable.
pub const MIN_WIDTH: u16 = 40;
