//! Widget components for the whip TUI.
//!
//! This module provides reusable rendering functions for the Kanban board UI,
//! organized into focused submodules for each visual component.
//!
//! # Overview
//!
//! The widget system follows a functional rendering approach where each widget
//! is a pure function that renders state to a buffer. This enables easy testing
//! and composition.
//!
//! # Modules
//!
//! - [`board`]: Renders the complete Kanban board with four lanes
//! - [`lane`]: Renders individual lanes with task lists
//! - [`task_card`]: Renders task cards with color coding based on state
//! - [`status_bar`]: Renders the footer with keybinding hints
//!
//! # Color Coding
//!
//! Task cards are color-coded based on their [`TaskState`](whip_protocol::TaskState):
//!
//! | State | Color |
//! |-------|-------|
//! | `Idle` | Gray (`Color::DarkGray`) |
//! | `InFlight` | Blue (`Color::Blue`) |
//! | `NeedsAttention` | Yellow (`Color::Yellow`) |
//! | `Success` | Green (`Color::Green`) |
//! | `Failed` | Red (`Color::Red`) |
//!
//! # Example
//!
//! ```
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use whip_protocol::{KanbanBoard, Task};
//! use whip_tui::widgets;
//!
//! let mut board = KanbanBoard::new();
//! board.add_task(Task::new("Example", "A sample task"));
//!
//! let area = Rect::new(0, 0, 80, 24);
//! let mut buf = Buffer::empty(area);
//!
//! widgets::render_board(&board, 0, Some(0), area, &mut buf);
//! ```

pub mod board;
pub mod detail;
pub mod help;
pub mod lane;
pub mod markdown;
pub mod settings;
pub mod status_bar;
pub mod task_card;

// Re-export primary rendering functions for convenience
pub use board::render_board;
pub use detail::{
    calculate_metadata_height, description_area_dimensions, label_color, max_scroll_offset,
    render_detail_panel, state_indicator,
};
pub use help::render_help_overlay;
pub use lane::{LanePosition, render_lane};
pub use settings::render_settings_panel;
pub use status_bar::render_status_bar;
pub use task_card::{render_task_card, state_color};

#[cfg(test)]
mod tests;
