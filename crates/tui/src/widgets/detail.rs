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
use whip_protocol::{Task, TaskState};

use super::markdown::render_markdown;
use super::task_card::state_color;

/// Returns a color for a label based on its name (deterministic).
///
/// Uses a simple hash-based color selection so the same label always gets the same color.
///
/// # Examples
///
/// ```
/// use whip_tui::widgets::label_color;
/// use ratatui::style::Color;
///
/// // Same label always gets the same color
/// let color1 = label_color("bug");
/// let color2 = label_color("bug");
/// assert_eq!(color1, color2);
/// ```
#[must_use]
pub fn label_color(label: &str) -> Color {
    let hash: u32 = label
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
    match hash % 6 {
        0 => Color::LightBlue,
        1 => Color::LightGreen,
        2 => Color::LightYellow,
        3 => Color::LightMagenta,
        4 => Color::LightCyan,
        _ => Color::LightRed,
    }
}

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
/// use whip_protocol::TaskState;
/// use whip_tui::widgets::state_indicator;
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
/// use whip_protocol::Task;
/// use whip_tui::widgets::render_detail_panel;
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
    render_footer(task, chunks[4], buf);
}

/// Calculates the height needed for metadata based on available width.
///
/// The height calculation depends on what fits and whether GitHub metadata is present:
///
/// **Without GitHub:**
/// - 1 line: Lane+Status+Created+Updated all fit
/// - 2 lines: Lane+Status on line 1, Created+Updated on line 2
/// - 3 lines: Lane+Status on line 1, Created on line 2, Updated on line 3
///
/// **With GitHub:**
/// - 1 line: All metadata fits on one line
/// - 2 lines: Lane+Status on line 1, Created+Updated+GitHub on line 2
/// - 3 lines: Lane+Status on line 1, Created+Updated on line 2, GitHub on line 3
/// - 4 lines: Each item on its own line
#[must_use]
pub fn calculate_metadata_height(task: &Task, width: u16) -> u16 {
    let state_name = state_display_name(task.state);
    let lane_name = task.lane.display_name();
    let created_fmt = task.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_fmt = task.updated_at.format("%Y-%m-%d %H:%M").to_string();

    // Calculate lengths of each segment (including separators)
    // Lane: "Lane: Backlog" = 6 + lane_name.len()
    let lane_len = 6 + lane_name.len();
    // Status: "  │  ● In Progress" ~= 5 + indicator(1) + space(1) + state_name
    let status_len = 7 + state_name.len();
    // Created: "  │  Created: 2025-01-28 10:30" ~= 5 + 9 + 16
    let created_len = 5 + 9 + created_fmt.len();
    // Updated: "  │  Updated: 2025-01-28 10:30" ~= 5 + 9 + 16
    let updated_len = 5 + 9 + updated_fmt.len();

    // GitHub: "  │   owner/repo#123" ~= 5 + 1 (icon) + 1 (space) + owner + 1 (/) + repo + 1 (#) + number
    let github_len = task.github.as_ref().map_or(0, |gh| {
        let num_len = gh.number.to_string().len();
        5 + 1 + 1 + gh.owner.len() + 1 + gh.repo.len() + 1 + num_len
    });

    let full_line = lane_len + status_len + created_len + updated_len + github_len;
    let first_line = lane_len + status_len;
    // Second line in 2-row mode: "Created: timestamp" padded to lane_len+2, then "│  Updated: timestamp"
    // lane_len + 2 (for "  ") = column where │ appears
    // Then: "│  " (3) + "Updated: " (9) + timestamp
    let lane_section_len = lane_len + 2; // includes the "  " before │
    let created_with_label = 9 + created_fmt.len(); // "Created: " + timestamp
    let left_col = lane_section_len.max(created_with_label);
    let timestamps_line = left_col + 3 + 9 + updated_fmt.len(); // left_col + "│  " + "Updated: " + timestamp
    let timestamps_with_github = timestamps_line + github_len;

    let w = width as usize;

    if task.github.is_some() {
        // With GitHub metadata
        if full_line <= w {
            1 // Everything fits on one line
        } else if first_line <= w && timestamps_with_github <= w {
            2 // Lane+Status on first line, timestamps + GitHub on second
        } else if first_line <= w && timestamps_line <= w {
            3 // Lane+Status on line 1, timestamps on line 2, GitHub on line 3
        } else {
            4 // Each item on its own line
        }
    } else {
        // Without GitHub metadata (original behavior)
        let full_line_no_github = lane_len + status_len + created_len + updated_len;
        if full_line_no_github <= w {
            1 // Everything fits on one line
        } else if first_line <= w && timestamps_line <= w {
            2 // Lane+Status on first line, timestamps on second
        } else {
            3 // Each major group on its own line
        }
    }
}

/// Renders the metadata (status, lane, timestamps, GitHub link) with responsive layout.
fn render_metadata(task: &Task, area: Rect, buf: &mut Buffer) {
    let (indicator, indicator_color) = state_indicator(task.state);
    let state_name = state_display_name(task.state);
    let created_fmt = task.created_at.format("%Y-%m-%d %H:%M").to_string();
    let updated_fmt = task.updated_at.format("%Y-%m-%d %H:%M").to_string();

    let height = calculate_metadata_height(task, area.width);

    // Build GitHub spans if available
    let github_spans = task.github.as_ref().map(|gh| {
        vec![
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{F09B} ", Style::default().fg(Color::White)), // GitHub octicon
            Span::styled(
                format!("{}/{}#{}", gh.owner, gh.repo, gh.number),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]
    });

    let lines: Vec<Line<'static>> = if height == 1 {
        // Everything on one line
        let mut spans = vec![
            Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                task.lane.display_name().to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{indicator} "),
                Style::default().fg(indicator_color),
            ),
            Span::styled(state_name, Style::default().fg(state_color(task.state))),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
            Span::styled(created_fmt, Style::default().fg(Color::White)),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
            Span::styled(updated_fmt, Style::default().fg(Color::White)),
        ];
        if let Some(gh_spans) = github_spans {
            spans.extend(gh_spans);
        }
        vec![Line::from(spans)]
    } else if height == 2 {
        // Lane+Status on first line, timestamps (+ GitHub if fits) on second
        // Align the │ delimiter: both lines have same column width before │
        // First line:  "Lane: In Progress │  ● In Progress"
        // Second line: "Created: 10:30    │  Updated: 10:30  │   owner/repo#123"

        // Calculate column width: max of lane section and created section
        let lane_section = format!("Lane: {}", task.lane.display_name());
        let created_section = format!("Created: {created_fmt}");
        let col_width = lane_section.len().max(created_section.len());

        let mut second_line_spans = vec![
            Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{created_fmt:width$}", width = col_width - 9), // -9 for "Created: "
                Style::default().fg(Color::White),
            ),
            Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
            Span::styled(updated_fmt, Style::default().fg(Color::White)),
        ];
        if let Some(gh_spans) = github_spans {
            second_line_spans.extend(gh_spans);
        }

        vec![
            Line::from(vec![
                Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:width$}", task.lane.display_name(), width = col_width - 6), // -6 for "Lane: "
                    Style::default().fg(Color::White),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{indicator} "),
                    Style::default().fg(indicator_color),
                ),
                Span::styled(state_name, Style::default().fg(state_color(task.state))),
            ]),
            Line::from(second_line_spans),
        ]
    } else if height == 3 {
        // Three lines layout depends on whether GitHub is present:
        // - With GitHub: Lane+Status, Created+Updated, GitHub
        // - Without GitHub: Lane+Status, Created, Updated
        if task.github.is_some() {
            // GitHub present: timestamps on one line, GitHub on third
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        task.lane.display_name().to_string(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{indicator} "),
                        Style::default().fg(indicator_color),
                    ),
                    Span::styled(state_name, Style::default().fg(state_color(task.state))),
                ]),
                Line::from(vec![
                    Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(created_fmt, Style::default().fg(Color::White)),
                    Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(updated_fmt, Style::default().fg(Color::White)),
                ]),
            ];
            if let Some(gh) = &task.github {
                lines.push(Line::from(vec![
                    Span::styled("\u{F09B} ", Style::default().fg(Color::White)), // GitHub octicon
                    Span::styled(
                        format!("{}/{}#{}", gh.owner, gh.repo, gh.number),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
            }
            lines
        } else {
            // No GitHub: each timestamp on its own line (original behavior)
            vec![
                Line::from(vec![
                    Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        task.lane.display_name().to_string(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{indicator} "),
                        Style::default().fg(indicator_color),
                    ),
                    Span::styled(state_name, Style::default().fg(state_color(task.state))),
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
        }
    } else {
        // Four lines: each item on its own line (narrow terminal with GitHub)
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Lane: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    task.lane.display_name().to_string(),
                    Style::default().fg(Color::White),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{indicator} "),
                    Style::default().fg(indicator_color),
                ),
                Span::styled(state_name, Style::default().fg(state_color(task.state))),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::styled(created_fmt, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::styled(updated_fmt, Style::default().fg(Color::White)),
            ]),
        ];
        if let Some(gh) = &task.github {
            lines.push(Line::from(vec![
                Span::styled("\u{F09B} ", Style::default().fg(Color::White)), // GitHub octicon
                Span::styled(
                    format!("{}/{}#{}", gh.owner, gh.repo, gh.number),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]));
        }
        lines
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

/// Renders the description section with scrolling support, labels, and markdown formatting.
fn render_description(task: &Task, scroll_offset: u16, area: Rect, buf: &mut Buffer) {
    // Calculate readable width (max 100 chars for readability, or full width if smaller)
    let content_width = area.width.min(100) as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Add labels if present (from GitHub source)
    if let Some(gh) = &task.github
        && !gh.labels.is_empty()
    {
        let label_spans: Vec<Span<'static>> = gh
            .labels
            .iter()
            .flat_map(|label| {
                let color = label_color(label);
                vec![
                    Span::styled(
                        format!(" {label} "),
                        Style::default().fg(Color::Black).bg(color),
                    ),
                    Span::raw(" "), // spacing between labels
                ]
            })
            .collect();
        lines.push(Line::from(label_spans));
        lines.push(Line::from("")); // blank line after labels
    }

    // Add description content
    if task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Render description as markdown
        lines.extend(render_markdown(&task.description, content_width));
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
fn render_footer(task: &Task, area: Rect, buf: &mut Buffer) {
    let mut spans = vec![
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::styled(" Back to board  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::styled(" Scroll", Style::default().fg(Color::DarkGray)),
    ];

    // Add "Open in browser" hint if task has GitHub source
    if task.github.is_some() {
        spans.push(Span::styled("  ", Style::default()));
        spans.push(Span::styled("[o]", Style::default().fg(Color::Yellow)));
        spans.push(Span::styled(
            " Open in browser",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let footer = Paragraph::new(Line::from(spans));
    footer.render(area, buf);
}

/// Builds the description lines for scroll calculation.
///
/// This returns just the description lines (for scroll offset calculation),
/// using markdown rendering to match the actual display. Includes labels
/// if the task has GitHub metadata with labels.
fn build_description_lines(task: &Task, width: u16) -> Vec<Line<'static>> {
    let content_width = width.min(100) as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Account for labels if present
    if let Some(gh) = &task.github
        && !gh.labels.is_empty()
    {
        // Labels line + blank line
        lines.push(Line::from("")); // placeholder for labels
        lines.push(Line::from("")); // blank line after labels
    }

    if task.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Use markdown renderer for consistent line count
        lines.extend(render_markdown(&task.description, content_width));
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
#[cfg(test)]
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

/// Calculates the description area dimensions for a given panel area and task.
///
/// This computes the height and width of the scrollable description area
/// within the detail panel, accounting for borders, metadata, separators, and footer.
///
/// Returns `(visible_height, content_width)` or `None` if the area is too small.
///
/// # Arguments
///
/// * `task` - The task being displayed (needed for metadata height calculation)
/// * `area` - The full area of the detail panel (including borders)
#[must_use]
pub fn description_area_dimensions(task: &Task, area: Rect) -> Option<(u16, u16)> {
    // Minimum area check (same as render_detail_panel)
    if area.width < 20 || area.height < 10 {
        return None;
    }

    // Account for the outer block borders (2 rows, 2 columns)
    let inner_height = area.height.saturating_sub(2);
    let inner_width = area.width.saturating_sub(2);

    // Calculate metadata height (same logic as in render_detail_panel)
    let metadata_height = calculate_metadata_height(task, inner_width);

    // Layout: Metadata + Separator (1) + Description (flex) + Separator (1) + Footer (1)
    // Description height = inner_height - metadata_height - 3
    let description_height = inner_height.saturating_sub(metadata_height + 3);

    // Minimum of 1 line for description
    if description_height == 0 {
        return None;
    }

    Some((description_height, inner_width))
}

#[cfg(test)]
mod tests {
    use super::*;
    use whip_protocol::{GitHubSource, LaneKind};

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
        assert!(
            offset > 0,
            "offset should be > 0 for long description, got {offset}"
        );
    }

    #[test]
    fn label_color_is_deterministic() {
        // Same label should always get the same color
        let color1 = label_color("bug");
        let color2 = label_color("bug");
        assert_eq!(color1, color2);

        let color3 = label_color("enhancement");
        let color4 = label_color("enhancement");
        assert_eq!(color3, color4);
    }

    #[test]
    fn label_color_covers_all_colors() {
        // Different labels should produce different colors (given enough variety)
        let colors: std::collections::HashSet<Color> = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t", "u", "v", "w", "x", "y", "z",
        ]
        .iter()
        .map(|s| label_color(s))
        .collect();

        // Should have multiple distinct colors
        assert!(
            colors.len() >= 3,
            "expected at least 3 distinct colors, got {}",
            colors.len()
        );
    }

    #[test]
    fn label_color_returns_valid_colors() {
        let valid_colors = [
            Color::LightBlue,
            Color::LightGreen,
            Color::LightYellow,
            Color::LightMagenta,
            Color::LightCyan,
            Color::LightRed,
        ];

        for label in [
            "bug",
            "feature",
            "enhancement",
            "help wanted",
            "good first issue",
        ] {
            let color = label_color(label);
            assert!(
                valid_colors.contains(&color),
                "label_color({label:?}) returned unexpected color: {color:?}"
            );
        }
    }

    #[test]
    fn render_detail_panel_with_github_metadata() {
        let mut task = Task::new("Test Task", "A test description");
        task.github = Some(GitHubSource {
            owner: "rust-lang".to_string(),
            repo: "rust".to_string(),
            number: 12345,
            url: "https://github.com/rust-lang/rust/issues/12345".to_string(),
            labels: vec!["bug".to_string(), "help wanted".to_string()],
            author: "octocat".to_string(),
            comment_count: 42,
        });

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_detail_panel(&task, 0, area, &mut buf);

        // Convert buffer to string for easier inspection
        let content: String = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        buf.cell((x, y))
                            .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' '))
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Verify GitHub reference is displayed
        assert!(
            content.contains("rust-lang/rust#12345"),
            "Expected GitHub reference in output, got:\n{content}"
        );

        // Verify labels are displayed
        assert!(
            content.contains("bug"),
            "Expected 'bug' label in output, got:\n{content}"
        );
        assert!(
            content.contains("help wanted"),
            "Expected 'help wanted' label in output, got:\n{content}"
        );

        // Verify "Open in browser" hint is displayed
        assert!(
            content.contains("[o]") && content.contains("Open in browser"),
            "Expected '[o] Open in browser' hint in output, got:\n{content}"
        );
    }

    #[test]
    fn render_detail_panel_without_github_no_browser_hint() {
        let task = Task::new("Test Task", "A test description");

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);

        render_detail_panel(&task, 0, area, &mut buf);

        // Convert buffer to string for easier inspection
        let content: String = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        buf.cell((x, y))
                            .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' '))
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Verify "Open in browser" hint is NOT displayed
        assert!(
            !content.contains("Open in browser"),
            "Expected no 'Open in browser' hint without GitHub source, got:\n{content}"
        );
    }

    #[test]
    fn calculate_metadata_height_with_github() {
        let mut task = Task::new("Test Task", "Description");
        task.github = Some(GitHubSource {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            number: 123,
            url: "https://github.com/owner/repo/issues/123".to_string(),
            labels: vec![],
            author: "author".to_string(),
            comment_count: 0,
        });

        // Wide terminal should fit everything on fewer lines
        let height_wide = calculate_metadata_height(&task, 200);
        assert!(
            height_wide <= 2,
            "expected 1-2 lines for wide terminal, got {height_wide}"
        );

        // Narrow terminal needs more lines
        let height_narrow = calculate_metadata_height(&task, 40);
        assert!(
            height_narrow >= 2,
            "expected 2+ lines for narrow terminal, got {height_narrow}"
        );
    }

    #[test]
    fn build_description_lines_includes_labels() {
        let mut task = Task::new("Test Task", "Description text");
        task.github = Some(GitHubSource {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            number: 123,
            url: "https://github.com/owner/repo/issues/123".to_string(),
            labels: vec!["bug".to_string(), "help wanted".to_string()],
            author: "author".to_string(),
            comment_count: 0,
        });

        let lines = build_description_lines(&task, 80);

        // Should have extra lines for labels (1 label line + 1 blank line)
        // compared to a task without labels
        let task_no_labels = Task::new("Test Task", "Description text");
        let lines_no_labels = build_description_lines(&task_no_labels, 80);

        assert!(
            lines.len() > lines_no_labels.len(),
            "expected more lines with labels ({} vs {})",
            lines.len(),
            lines_no_labels.len()
        );
    }
}
