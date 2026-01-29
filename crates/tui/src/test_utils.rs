//! Test utilities for the TUI crate.
//!
//! This module provides common helper functions used across test modules
//! for rendering verification and snapshot testing.

use ratatui::buffer::Buffer;

/// Converts a ratatui [`Buffer`] to a string representation.
///
/// Each row of the buffer becomes a line in the output string. Trailing
/// whitespace is trimmed from each line to produce cleaner output suitable
/// for snapshot testing.
///
/// # Arguments
///
/// * `buf` - The buffer to convert
///
/// # Returns
///
/// A string representation of the buffer contents with one line per row.
///
/// # Example
///
/// ```ignore
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
///
/// let area = Rect::new(0, 0, 10, 2);
/// let mut buf = Buffer::empty(area);
/// buf.set_string(0, 0, "Hello", ratatui::style::Style::default());
///
/// let output = buffer_to_string(&buf);
/// assert!(output.contains("Hello"));
/// ```
#[must_use]
pub(crate) fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        // Trim trailing whitespace from each line for cleaner snapshots
        let trimmed = result.trim_end_matches(' ');
        result.truncate(trimmed.len());
        result.push('\n');
    }
    result
}
