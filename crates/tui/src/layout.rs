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
