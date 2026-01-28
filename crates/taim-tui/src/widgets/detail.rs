//! Task detail screen widget.
//!
//! This module provides rendering for the full-screen task detail view, which shows
//! comprehensive information about a selected task including its title,
//! status, description, and timestamps.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
};
use taim_protocol::{Task, TaskState};

use super::task_card::state_color;

/// Returns the status indicator symbol and color for a task state.
///
/// # Status Indicators
///
/// | State | Symbol | Meaning |
/// |-------|--------|---------|
/// | `Idle` | `○` | Empty circle - waiting |
/// | `InFlight` | `●` | Filled circle - active |
/// | `NeedsAttention` | `◆` | Diamond - blocked |
/// | `Success` | `✓` | Checkmark - complete |
/// | `Failed` | `✗` | X mark - error |
///
/// # Examples
///
/// ```
/// use taim_protocol::TaskState;
/// use taim_tui::widgets::state_indicator;
///
/// let (symbol, color) = state_indicator(TaskState::InFlight);
/// assert_eq!(symbol, '●');
/// ```
#[must_use]
pub const fn state_indicator(state: TaskState) -> (char, Color) {
    match state {
        TaskState::Idle => ('\u{25CB}', Color::DarkGray), // ○
        TaskState::InFlight => ('\u{25CF}', Color::Blue), // ●
        TaskState::NeedsAttention => ('\u{25C6}', Color::Yellow), // ◆
        TaskState::Success => ('\u{2713}', Color::Green), // ✓
        TaskState::Failed => ('\u{2717}', Color::Red),    // ✗
    }
}

/// Returns the display name for a task state.
#[must_use]
pub const fn state_display_name(state: TaskState) -> &'static str {
    match state {
        TaskState::Idle => "Idle",
        TaskState::InFlight => "In Progress",
        TaskState::NeedsAttention => "Needs Attention",
        TaskState::Success => "Success",
        TaskState::Failed => "Failed",
    }
}

/// Renders the full-screen task detail view to the buffer.
///
/// The detail view shows comprehensive task information using the full screen:
///
/// - Header with title (large, prominent)
/// - Metadata row: Status, Lane, Timestamps (horizontal layout for wide screens)
/// - Description section (scrollable, centered with readable width)
/// - Keybinding hint at bottom
///
/// # Arguments
///
/// * `task` - The task to display details for
/// * `scroll_offset` - Vertical scroll offset for the description
/// * `area` - The rectangular area to render into
/// * `buf` - The buffer to render into
///
/// # Layout (Full Screen)
///
/// ```text
/// +═══════════════════════════════════════════════════════════════════════════+
/// │                                                                           │
/// │   Implement feature                                                       │
/// │                                                                           │
/// │   ● In Progress  │  Lane: In Progress  │  Created: 2025-01-28 10:30      │
/// │                                                                           │
/// │   ─────────────────────────────────────────────────────────────────────── │
/// │                                                                           │
/// │   Create a login form with validation. The form should include email     │
/// │   and password fields, with appropriate error handling...                 │
/// │                                                                           │
/// │   ─────────────────────────────────────────────────────────────────────── │
/// │                                                                           │
/// │   [Esc] Back to board  [↑↓] Scroll                                       │
/// │                                                                           │
/// +═══════════════════════════════════════════════════════════════════════════+
/// ```
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use taim_protocol::Task;
/// use taim_tui::widgets::render_detail_panel;
///
/// let task = Task::new("Implement feature", "Add user authentication");
/// let area = Rect::new(0, 0, 80, 24);
/// let mut buf = Buffer::empty(area);
///
/// render_detail_panel(&task, 0, area, &mut buf);
/// ```
pub fn render_detail_panel(task: &Task, scroll_offset: u16, area: Rect, buf: &mut Buffer) {
    // Skip rendering if area is too small
    if area.width < 20 || area.height < 10 {
        return;
    }

    // Create the outer block with rounded corners and task title
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", task.title),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    block.render(area, buf);

    // Calculate metadata height based on available width
    let metadata_height = calculate_metadata_height(task, inner_area.width);

    // Layout: Metadata (dynamic) + Separator (1) + Description (flex) + Separator (1) + Footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(metadata_height), // Metadata rows (responsive)
            Constraint::Length(1),               // Separator
            Constraint::Min(3),                  // Description (flexible)
            Constraint::Length(1),               // Separator
            Constraint::Length(1),               // Footer
        ])
        .split(inner_area);

    // Render metadata (status, lane, timestamps)
    render_metadata(task, chunks[0], buf);

    // Render separator
    render_separator(chunks[1], buf);

    // Render description (scrollable)
    render_description(task, scroll_offset, chunks[2], buf);

    // Render separator
    render_separator(chunks[3], buf);

    // Render footer with keybindings
    render_footer(chunks[4], buf);
}

/// Calculates the height needed for metadata based on available width.
///
/// Returns 1 if all metadata fits on one line, 2 if timestamps need a second line,
/// or 3 if each group needs its own line.
fn calculate_metadata_height(task: &Task, width: u16) -> u16 {
    let state_name = state_display_name(task.state);
    let lane_name = task.lane.display_name();
    let created_fmt = task.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_fmt = task.updated_at.format("%Y-%m-%d %H:%M").to_string();

    // Calculate lengths of each segment (including separators)
    // Status: "● In Progress" = indicator(1) + space(1) + state_name
    let status_len = 2 + state_name.len();
    // Lane: "  │  Lane: Backlog" ~= 5 + 6 + lane_name.len()
    let lane_len = 11 + lane_name.len();
    // Created: "  │  Created: 2025-01-28 10:30" ~= 5 + 9 + 16
    let created_len = 5 + 9 + created_fmt.len();
    // Updated: "  │  Updated: 2025-01-28 10:30" ~= 5 + 9 + 16
    let updated_len = 5 + 9 + updated_fmt.len();

    let full_line = status_len + lane_len + created_len + updated_len;
    let first_line = status_len + lane_len;
    // Second line in 2-row mode: "Created: timestamp" padded to status_len+2, then "│  Updated: timestamp"
    // status_len + 2 (for "  ") = column where │ appears
    // Then: "│  " (3) + "Updated: " (9) + timestamp
    let status_section_len = status_len + 2; // includes the "  " before │
    let created_with_label = 9 + created_fmt.len(); // "Created: " + timestamp
    let left_col = status_section_len.max(created_with_label);
    let timestamps_line = left_col + 3 + 9 + updated_fmt.len(); // left_col + "│  " + "Updated: " + timestamp

    let w = width as usize;

    if full_line <= w {
        1 // Everything fits on one line
    } else if first_line <= w && timestamps_line <= w {
        2 // Status+Lane on first line, timestamps on second (aligned)
    } else {
        3 // Each major group on its own line
    }
}

/// Renders the metadata (status, lane, timestamps) with responsive layout.
fn render_metadata(task: &Task, area: Rect, buf: &mut Buffer) {
    let (indicator, indicator_color) = state_indicator(task.state);
    let state_name = state_display_name(task.state);
    let created_fmt = task.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_fmt = task.updated_at.format("%Y-%m-%d %H:%M").to_string();

    let height = calculate_metadata_height(task, area.width);

    let lines: Vec<Line<'static>> = if height == 1 {
        // Everything on one line
        vec![Line::from(vec![
            Span::styled(format!("{indicator} "), Style::default().fg(indicator_color)),
            Span::styled(state_name, Style::default().fg(state_color(task.state))),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                task.lane.display_name().to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
            Span::styled(created_fmt, Style::default().fg(Color::White)),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
            Span::styled(updated_fmt, Style::default().fg(Color::White)),
        ])]
    } else if height == 2 {
        // Status+Lane on first line, timestamps on second
        // Align the │ delimiter: both lines have same column width before │
        // First line:  "● In Progress     │  Lane: In Progress"
        // Second line: "Created: 10:30    │  Updated: 10:30"

        // Calculate column width: max of status section and created section
        let status_section = format!("{indicator} {state_name}");
        let created_section = format!("Created: {created_fmt}");
        let col_width = status_section.len().max(created_section.len());

        vec![
            Line::from(vec![
                Span::styled(format!("{indicator} "), Style::default().fg(indicator_color)),
                Span::styled(
                    format!("{state_name:width$}", width = col_width - 2), // -2 for "● "
                    Style::default().fg(state_color(task.state)),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    task.lane.display_name().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{created_fmt:width$}", width = col_width - 9), // -9 for "Created: "
                    Style::default().fg(Color::White),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::styled(updated_fmt, Style::default().fg(Color::White)),
            ]),
        ]
    } else {
        // Each group on its own line
        vec![
            Line::from(vec![
                Span::styled(format!("{indicator} "), Style::default().fg(indicator_color)),
                Span::styled(state_name, Style::default().fg(state_color(task.state))),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    task.lane.display_name().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::styled(created_fmt, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::styled(updated_fmt, Style::default().fg(Color::White)),
            ]),
        ]
    };

    let metadata = Paragraph::new(lines);
    metadata.render(area, buf);
}

/// Renders a horizontal separator line.
fn render_separator(area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let sep = Paragraph::new(Line::from(Span::styled(
        "\u{2500}".repeat(width),
        Style::default().fg(Color::DarkGray),
    )));
    sep.render(area, buf);
}

/// Renders the description section with scrolling support.
fn render_description(task: &Task, scroll_offset: u16, area: Rect, buf: &mut Buffer) {
    // Calculate readable width (max 80 chars for readability, or full width if smaller)
    let content_width = area.width.min(100) as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    if task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Wrap description text to readable width
        for line in wrap_text(&task.description, content_width) {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }
    }

    // Apply scroll offset
    let scroll = scroll_offset as usize;
    if scroll < lines.len() {
        lines = lines.into_iter().skip(scroll).collect();
    } else {
        lines.clear();
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    content.render(area, buf);
}

/// Renders the footer with keybinding hints.
fn render_footer(area: Rect, buf: &mut Buffer) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::styled(" Back to board  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::styled(" Scroll", Style::default().fg(Color::DarkGray)),
    ]));
    footer.render(area, buf);
}

/// Builds the description lines for scroll calculation.
///
/// This returns just the description lines (for scroll offset calculation).
fn build_description_lines(task: &Task, width: u16) -> Vec<Line<'static>> {
    let content_width = width.min(100) as usize;
    let mut lines = Vec::new();

    if task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        for line in wrap_text(&task.description, content_width) {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }
    }

    lines
}

/// Builds the content lines for the detail panel (used by tests).
#[cfg(test)]
fn build_detail_lines(task: &Task, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let inner_width = width.saturating_sub(2) as usize;

    // Title section
    lines.push(Line::from(vec![
        Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            task.title.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Status with indicator
    let (indicator, indicator_color) = state_indicator(task.state);
    let state_name = state_display_name(task.state);
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{indicator} "),
            Style::default().fg(indicator_color),
        ),
        Span::styled(state_name, Style::default().fg(state_color(task.state))),
    ]));

    // Lane
    lines.push(Line::from(vec![
        Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            task.lane.display_name().to_string(),
            Style::default().fg(Color::White),
        ),
    ]));

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(inner_width.min(40)),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Description header
    lines.push(Line::from(Span::styled(
        "Description:",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));

    // Description content (or placeholder if empty)
    if task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Wrap description text
        for line in wrap_text(&task.description, inner_width) {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::White),
            )));
        }
    }

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(inner_width.min(40)),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Timestamps
    let created_fmt = task.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_fmt = task.updated_at.format("%Y-%m-%d %H:%M").to_string();

    lines.push(Line::from(vec![
        Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
        Span::styled(created_fmt, Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
        Span::styled(updated_fmt, Style::default().fg(Color::White)),
    ]));

    // Keybinding hint
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Esc] Close",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

/// Wraps text to fit within a given width.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_width == 0 {
            // Start of a new line
            if word_len > max_width {
                // Word is too long, force split it
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if !chunk.is_empty() {
                        lines.push(chunk);
                    }
                }
                current_line = String::new();
                current_width = 0;
            } else {
                current_line = word.to_string();
                current_width = word_len;
            }
        } else if current_width + 1 + word_len <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_len;
        } else {
            // Word doesn't fit, start a new line
            lines.push(std::mem::take(&mut current_line));
            if word_len > max_width {
                // Word is too long, force split it
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if !chunk.is_empty() {
                        lines.push(chunk);
                    }
                }
                current_line = String::new();
                current_width = 0;
            } else {
                current_line = word.to_string();
                current_width = word_len;
            }
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Calculates the maximum scroll offset for a task's description.
///
/// Returns the number of lines that can be scrolled while keeping
/// at least one line visible.
///
/// # Arguments
///
/// * `task` - The task to calculate scroll for
/// * `visible_height` - The number of visible lines in the description area
/// * `panel_width` - The width of the panel (for text wrapping calculation)
#[must_use]
pub fn max_scroll_offset(task: &Task, visible_height: u16, panel_width: u16) -> u16 {
    let lines = build_description_lines(task, panel_width);
    let total_lines = lines.len() as u16;

    total_lines.saturating_sub(visible_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taim_protocol::LaneKind;

    #[test]
    fn state_indicator_mapping() {
        assert_eq!(
            state_indicator(TaskState::Idle),
            ('\u{25CB}', Color::DarkGray)
        );
        assert_eq!(
            state_indicator(TaskState::InFlight),
            ('\u{25CF}', Color::Blue)
        );
        assert_eq!(
            state_indicator(TaskState::NeedsAttention),
            ('\u{25C6}', Color::Yellow)
        );
        assert_eq!(
            state_indicator(TaskState::Success),
            ('\u{2713}', Color::Green)
        );
        assert_eq!(state_indicator(TaskState::Failed), ('\u{2717}', Color::Red));
    }

    #[test]
    fn state_display_name_mapping() {
        assert_eq!(state_display_name(TaskState::Idle), "Idle");
        assert_eq!(state_display_name(TaskState::InFlight), "In Progress");
        assert_eq!(
            state_display_name(TaskState::NeedsAttention),
            "Needs Attention"
        );
        assert_eq!(state_display_name(TaskState::Success), "Success");
        assert_eq!(state_display_name(TaskState::Failed), "Failed");
    }

    #[test]
    fn wrap_text_short_text() {
        let result = wrap_text("Hello world", 20);
        assert_eq!(result, vec!["Hello world"]);
    }

    #[test]
    fn wrap_text_long_text() {
        let result = wrap_text("This is a longer piece of text that needs wrapping", 20);
        assert!(result.len() > 1);
        for line in &result {
            assert!(line.chars().count() <= 20);
        }
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 20);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_zero_width() {
        let result = wrap_text("Hello world", 0);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_very_long_word() {
        let result = wrap_text("supercalifragilisticexpialidocious", 10);
        assert!(!result.is_empty());
        for line in &result {
            assert!(line.chars().count() <= 10);
        }
    }

    #[test]
    fn render_detail_panel_creates_output() {
        let task = Task::new("Test Task", "A test description for the task");
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);

        render_detail_panel(&task, 0, area, &mut buf);

        // Verify something was rendered (borders at minimum)
        let cell = buf.cell((0, 0)).expect("cell should exist");
        assert_ne!(cell.symbol(), " ");
    }

    #[test]
    fn render_detail_panel_handles_small_area() {
        let task = Task::new("Test Task", "Description");
        let area = Rect::new(0, 0, 5, 5);
        let mut buf = Buffer::empty(area);

        // Should not panic with tiny area
        render_detail_panel(&task, 0, area, &mut buf);
    }

    #[test]
    fn render_detail_panel_with_scroll() {
        let task = Task::new(
            "Test Task",
            "A very long description that should require scrolling when displayed in the detail panel. \
             This text contains multiple sentences to ensure we have enough content to test the \
             scrolling functionality properly.",
        );
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);

        render_detail_panel(&task, 5, area, &mut buf);

        // Should render without panicking
        let cell = buf.cell((0, 0)).expect("cell should exist");
        assert_ne!(cell.symbol(), " ");
    }

    #[test]
    fn build_detail_lines_includes_all_sections() {
        let mut task = Task::new("Test Title", "Test Description");
        task.state = TaskState::InFlight;
        task.lane = LaneKind::InProgress;

        let lines = build_detail_lines(&task, 40);

        // Convert lines to string for easier assertion
        let content: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(content.contains("Title:"));
        assert!(content.contains("Test Title"));
        assert!(content.contains("Status:"));
        assert!(content.contains("Lane:"));
        assert!(content.contains("Description:"));
        assert!(content.contains("Test Description"));
        assert!(content.contains("Created:"));
        assert!(content.contains("Updated:"));
        assert!(content.contains("[Esc] Close"));
    }

    #[test]
    fn max_scroll_offset_calculation() {
        let task = Task::new(
            "Test",
            "A description that spans multiple lines when wrapped. \
             More content here to increase the line count. \
             Even more content to ensure we have enough text to scroll. \
             This should definitely require scrolling when the visible height is small.",
        );

        // With visible_height=3 and a long description wrapped to ~30 char width,
        // we should have a positive scroll offset
        let offset = max_scroll_offset(&task, 3, 30);

        // Should return a positive value since description is longer than visible area
        assert!(offset > 0, "offset should be > 0 for long description, got {offset}");
    }
}
