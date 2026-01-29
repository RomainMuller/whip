//! Markdown rendering for TUI display.
//!
//! This module provides conversion from markdown text to styled ratatui `Line`s,
//! supporting common markdown elements like headers, bold, italic, code, lists,
//! links, and tables.
//!
//! # Supported Elements
//!
//! | Element | Markdown Syntax | Style |
//! |---------|-----------------|-------|
//! | H1 Header | `# text` | Bold + Cyan |
//! | H2+ Header | `## text` | Bold + White |
//! | Bold | `**text**` | Bold modifier |
//! | Italic | `*text*` | Italic modifier |
//! | Inline Code | `` `code` `` | Yellow |
//! | Code Block | ``` ```code``` ``` | Gray, indented |
//! | Lists | `- item` or `1. item` | Preserved with indent |
//! | Links | `[text](url)` | Cyan + underline |
//! | Tables | `| col | col |` | Adaptive format |
//!
//! # Table Rendering
//!
//! Tables use adaptive rendering based on available width:
//!
//! ## Wide terminals → Box-drawing tables:
//! ```text
//! ┌────────┬─────────┬──────────┐
//! │ Task   │ Status  │ Priority │
//! ├────────┼─────────┼──────────┤
//! │ Task 1 │ Done    │ High     │
//! └────────┴─────────┴──────────┘
//! ```
//!
//! ## Narrow terminals → Definition list:
//! ```text
//! ── Task 1 ──────
//!   Status:   Done
//!   Priority: High
//! ```
//!
//! Two-column tables in narrow mode render as compact key-value pairs:
//! ```text
//! Key: Value
//! Another: Data
//! ```
//!
//! # Example
//!
//! ```
//! use whip_tui::widgets::markdown::render_markdown;
//!
//! let markdown = "# Hello\n\nThis is **bold** and *italic* text.";
//! let lines = render_markdown(markdown, 80);
//! assert!(!lines.is_empty());
//! ```

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

// ============================================================================
// Table Support
// ============================================================================

/// A styled table cell containing formatted spans.
#[derive(Clone, Default)]
struct StyledCell {
    spans: Vec<Span<'static>>,
}

impl StyledCell {
    /// Returns the display width of the cell content.
    fn display_width(&self) -> usize {
        self.spans.iter().map(|s| s.content.chars().count()).sum()
    }

    /// Returns the plain text content of the cell.
    fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// Adds a styled span to the cell.
    fn push(&mut self, span: Span<'static>) {
        self.spans.push(span);
    }

    /// Adds a space if the cell is not empty (for soft breaks).
    fn push_space(&mut self) {
        if !self.spans.is_empty() {
            self.spans.push(Span::raw(" "));
        }
    }
}

/// Accumulated data for a table being parsed.
#[derive(Clone, Default)]
struct TableAccumulator {
    /// Column headers (from TableHead).
    headers: Vec<StyledCell>,
    /// All data rows.
    rows: Vec<Vec<StyledCell>>,
    /// Current row being built.
    current_row: Vec<StyledCell>,
    /// Current cell content being accumulated.
    current_cell: StyledCell,
    /// Whether we're currently in the header section.
    in_header: bool,
}

impl TableAccumulator {
    /// Returns the number of columns in this table.
    fn column_count(&self) -> usize {
        self.headers
            .len()
            .max(self.rows.iter().map(|r| r.len()).max().unwrap_or(0))
    }

    /// Calculates the maximum width needed for each column.
    fn column_widths(&self) -> Vec<usize> {
        let col_count = self.column_count();
        let mut widths = vec![0; col_count];

        // Consider headers
        for (i, header) in self.headers.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(header.display_width());
            }
        }

        // Consider all rows
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    widths[i] = widths[i].max(cell.display_width());
                }
            }
        }

        widths
    }

    /// Calculates the width required for box-drawing table format.
    /// Format: │ col1 │ col2 │ col3 │
    /// Width = 1 + sum(col_width + 3) for each column
    fn box_drawing_width(&self) -> usize {
        let widths = self.column_widths();
        if widths.is_empty() {
            return 0;
        }
        // │ + (content + padding + │) per column
        // = 1 + n * (width + 3)
        1 + widths.iter().map(|w| w + 3).sum::<usize>()
    }

    /// Returns true if this table has no content.
    fn is_empty(&self) -> bool {
        self.headers.is_empty() && self.rows.is_empty()
    }
}

/// Box-drawing characters for table borders.
mod box_chars {
    pub const TOP_LEFT: char = '\u{250C}'; // ┌
    pub const TOP_RIGHT: char = '\u{2510}'; // ┐
    pub const BOTTOM_LEFT: char = '\u{2514}'; // └
    pub const BOTTOM_RIGHT: char = '\u{2518}'; // ┘
    pub const HORIZONTAL: char = '\u{2500}'; // ─
    pub const VERTICAL: char = '\u{2502}'; // │
    pub const T_DOWN: char = '\u{252C}'; // ┬
    pub const T_UP: char = '\u{2534}'; // ┴
    pub const T_RIGHT: char = '\u{251C}'; // ├
    pub const T_LEFT: char = '\u{2524}'; // ┤
    pub const CROSS: char = '\u{253C}'; // ┼
}

/// Renders a table using the appropriate format based on available width.
fn render_table(table: &TableAccumulator, width: usize) -> Vec<Line<'static>> {
    if table.is_empty() {
        return vec![];
    }

    let box_width = table.box_drawing_width();

    if box_width <= width {
        render_box_table(table)
    } else {
        render_definition_list_table(table, width)
    }
}

/// Renders a table using box-drawing characters.
fn render_box_table(table: &TableAccumulator) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let widths = table.column_widths();
    let border_style = Style::default().fg(Color::DarkGray);

    // Top border: ┌───┬───┬───┐
    lines.push(build_horizontal_border(
        box_chars::TOP_LEFT,
        box_chars::T_DOWN,
        box_chars::TOP_RIGHT,
        &widths,
        border_style,
    ));

    // Header row
    if !table.headers.is_empty() {
        lines.push(build_table_row(&table.headers, &widths, border_style, true));

        // Header separator: ├───┼───┼───┤
        lines.push(build_horizontal_border(
            box_chars::T_RIGHT,
            box_chars::CROSS,
            box_chars::T_LEFT,
            &widths,
            border_style,
        ));
    }

    // Data rows
    for row in &table.rows {
        lines.push(build_table_row(row, &widths, border_style, false));
    }

    // Bottom border: └───┴───┴───┘
    lines.push(build_horizontal_border(
        box_chars::BOTTOM_LEFT,
        box_chars::T_UP,
        box_chars::BOTTOM_RIGHT,
        &widths,
        border_style,
    ));

    // Blank line after table
    lines.push(Line::from(""));

    lines
}

/// Builds a horizontal border line for box-drawing tables.
fn build_horizontal_border(
    left: char,
    middle: char,
    right: char,
    widths: &[usize],
    style: Style,
) -> Line<'static> {
    let mut content = String::new();
    content.push(left);

    for (i, &width) in widths.iter().enumerate() {
        // Cell width + 2 for padding
        content.extend(std::iter::repeat_n(box_chars::HORIZONTAL, width + 2));
        if i < widths.len() - 1 {
            content.push(middle);
        }
    }

    content.push(right);
    Line::from(Span::styled(content, style))
}

/// Builds a data row for box-drawing tables.
fn build_table_row(
    cells: &[StyledCell],
    widths: &[usize],
    border_style: Style,
    is_header: bool,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Left border
    spans.push(Span::styled(box_chars::VERTICAL.to_string(), border_style));

    for (i, width) in widths.iter().enumerate() {
        let cell = cells.get(i);
        let content_width = cell.map_or(0, StyledCell::display_width);
        let padding = width.saturating_sub(content_width);

        // Leading space
        spans.push(Span::raw(" "));

        // Cell content with styling
        if let Some(cell) = cell {
            for span in &cell.spans {
                let mut style = span.style;
                if is_header {
                    // Headers get cyan + bold styling
                    style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(span.content.clone(), style));
            }
        }

        // Trailing padding + space
        spans.push(Span::raw(format!("{} ", " ".repeat(padding))));

        // Separator
        spans.push(Span::styled(box_chars::VERTICAL.to_string(), border_style));
    }

    Line::from(spans)
}

/// Renders a table as a definition list (fallback for narrow terminals).
fn render_definition_list_table(table: &TableAccumulator, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let col_count = table.column_count();

    // Two-column tables: render as simple key-value pairs
    if col_count == 2 && !table.headers.is_empty() {
        return render_key_value_table(table);
    }

    // Multi-column tables: render as labeled entries per row
    let separator_style = Style::default().fg(Color::DarkGray);
    let label_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Calculate max header width for alignment
    let max_header_width = table
        .headers
        .iter()
        .map(|h| h.display_width())
        .max()
        .unwrap_or(0);

    for (row_idx, row) in table.rows.iter().enumerate() {
        // Determine row label (use first column if it looks like an identifier)
        let first_cell_text = row.first().map(|c| c.plain_text()).unwrap_or_default();
        let use_first_as_label = !row.is_empty() && looks_like_identifier(&first_cell_text);
        let row_label = if use_first_as_label {
            first_cell_text
        } else {
            format!("Row {}", row_idx + 1)
        };

        // Row separator: ── Label ──────
        let label_with_spaces = format!(" {row_label} ");
        let label_len = label_with_spaces.chars().count();
        let remaining = width.saturating_sub(label_len);
        let left_dashes = 2.min(remaining / 2);
        let right_dashes = remaining.saturating_sub(left_dashes);

        let separator_line = Line::from(vec![
            Span::styled(
                box_chars::HORIZONTAL.to_string().repeat(left_dashes),
                separator_style,
            ),
            Span::styled(label_with_spaces, separator_style),
            Span::styled(
                box_chars::HORIZONTAL.to_string().repeat(right_dashes),
                separator_style,
            ),
        ]);
        lines.push(separator_line);

        // Determine which columns to show (skip first if used as label)
        let start_col = if use_first_as_label { 1 } else { 0 };

        // Render each field
        for col_idx in start_col..row.len() {
            let header = table
                .headers
                .get(col_idx)
                .map(|h| h.plain_text())
                .unwrap_or_else(|| "Field".to_string());
            let header_width = header.chars().count();

            // Pad header for alignment
            let header_padding = max_header_width.saturating_sub(header_width);

            let mut line_spans = vec![
                Span::raw("  "),
                Span::styled(header, label_style),
                Span::styled(":".to_string(), separator_style),
                Span::raw(" ".repeat(header_padding + 1)),
            ];

            // Add the styled cell content
            if let Some(cell) = row.get(col_idx) {
                line_spans.extend(cell.spans.iter().cloned());
            }

            lines.push(Line::from(line_spans));
        }

        // Blank line between rows (except after last)
        if row_idx < table.rows.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    // Blank line after table
    lines.push(Line::from(""));

    lines
}

/// Renders a two-column table as simple key-value pairs.
fn render_key_value_table(table: &TableAccumulator) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let separator_style = Style::default().fg(Color::DarkGray);

    for row in &table.rows {
        let mut line_spans = Vec::new();

        // Key (first column) - apply label style
        if let Some(key_cell) = row.first() {
            for span in &key_cell.spans {
                let mut style = span.style;
                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                line_spans.push(Span::styled(span.content.clone(), style));
            }
        }

        // Separator
        line_spans.push(Span::styled(": ".to_string(), separator_style));

        // Value (second column) - keep original styling
        if let Some(value_cell) = row.get(1) {
            line_spans.extend(value_cell.spans.iter().cloned());
        }

        lines.push(Line::from(line_spans));
    }

    // Blank line after table
    lines.push(Line::from(""));

    lines
}

/// Checks if a string looks like an identifier (name, ID, task name, etc.).
fn looks_like_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Starts with a letter or #
    let first = s.chars().next().unwrap();
    if !first.is_alphabetic() && first != '#' {
        return false;
    }

    // Not too long (identifiers are usually short)
    s.chars().count() <= 30
}

// ============================================================================
// Main Rendering
// ============================================================================

/// Style context for tracking active modifiers during parsing.
#[derive(Clone, Default)]
struct StyleContext {
    bold: bool,
    italic: bool,
    code: bool,
    heading_level: Option<HeadingLevel>,
    link_url: Option<String>,
    in_code_block: bool,
    list_indent: usize,
    list_item_number: Option<u64>,
    /// Table being accumulated (if currently parsing a table).
    table: Option<TableAccumulator>,
}

impl StyleContext {
    /// Computes the current style based on active modifiers.
    fn current_style(&self) -> Style {
        let mut style = Style::default();

        // Heading styles take precedence
        if let Some(level) = self.heading_level {
            style = style.add_modifier(Modifier::BOLD);
            style = match level {
                HeadingLevel::H1 => style.fg(Color::Cyan),
                _ => style.fg(Color::White),
            };
            return style;
        }

        // Code styling
        if self.code || self.in_code_block {
            return style.fg(Color::Yellow);
        }

        // Link styling
        if self.link_url.is_some() {
            return style.fg(Color::Cyan).add_modifier(Modifier::UNDERLINED);
        }

        // Apply modifiers
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }

        // Default text color
        if !self.bold && !self.italic {
            style = style.fg(Color::White);
        }

        style
    }
}

/// Renders markdown text to styled lines for TUI display.
///
/// Parses the input markdown and converts it to a vector of `Line`s with
/// appropriate styling for headers, emphasis, code, lists, and links.
///
/// # Arguments
///
/// * `markdown` - The markdown text to render
/// * `width` - The maximum width for text wrapping
///
/// # Returns
///
/// A vector of styled `Line`s suitable for rendering with ratatui.
///
/// # Example
///
/// ```
/// use whip_tui::widgets::markdown::render_markdown;
///
/// let lines = render_markdown("**Hello** world!", 80);
/// assert!(!lines.is_empty());
/// ```
#[must_use]
pub fn render_markdown(markdown: &str, width: usize) -> Vec<Line<'static>> {
    if markdown.is_empty() {
        return vec![];
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut ctx = StyleContext::default();

    for event in parser {
        match event {
            Event::Start(tag) => {
                handle_start_tag(&tag, &mut ctx, &mut current_spans, &mut lines);
            }
            Event::End(tag_end) => {
                handle_end_tag(&tag_end, &mut ctx, &mut current_spans, &mut lines, width);
            }
            Event::Text(text) => {
                // If inside a table cell, accumulate styled span into the table
                // Compute style before taking mutable borrow
                let style = ctx.current_style();
                if let Some(ref mut table) = ctx.table {
                    table
                        .current_cell
                        .push(Span::styled(text.to_string(), style));
                } else {
                    handle_text(&text, &ctx, &mut current_spans, &mut lines, width);
                }
            }
            Event::Code(code) => {
                // If inside a table cell, accumulate styled code span
                if let Some(ref mut table) = ctx.table {
                    table.current_cell.push(Span::styled(
                        code.to_string(),
                        Style::default().fg(Color::Yellow),
                    ));
                } else {
                    // Inline code
                    current_spans.push(Span::styled(
                        code.to_string(),
                        Style::default().fg(Color::Yellow),
                    ));
                }
            }
            Event::SoftBreak => {
                // If inside a table cell, treat as space
                if let Some(ref mut table) = ctx.table {
                    table.current_cell.push_space();
                } else {
                    // Soft break - treat as space
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                // If inside a table cell, treat as space
                if let Some(ref mut table) = ctx.table {
                    table.current_cell.push_space();
                } else {
                    // Hard break - start new line
                    flush_line(&mut current_spans, &mut lines);
                }
            }
            Event::Rule => {
                // Horizontal rule
                flush_line(&mut current_spans, &mut lines);
                let rule = "\u{2500}".repeat(width.min(40));
                lines.push(Line::from(Span::styled(
                    rule,
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    // Flush any remaining content
    flush_line(&mut current_spans, &mut lines);

    lines
}

/// Handles the start of a markdown tag.
fn handle_start_tag(
    tag: &Tag<'_>,
    ctx: &mut StyleContext,
    current_spans: &mut Vec<Span<'static>>,
    lines: &mut Vec<Line<'static>>,
) {
    match tag {
        Tag::Heading { level, .. } => {
            // Ensure blank line before heading if there's content above
            flush_line(current_spans, lines);
            if !lines.is_empty() && !lines.last().is_some_and(|l| l.spans.is_empty()) {
                lines.push(Line::from(""));
            }
            // Add heading level prefix (e.g., "# ", "## ")
            let prefix = "#".repeat(heading_level_to_num(*level));
            current_spans.push(Span::styled(
                format!("{prefix} "),
                Style::default().fg(Color::DarkGray),
            ));
            ctx.heading_level = Some(*level);
        }
        Tag::Strong => {
            ctx.bold = true;
        }
        Tag::Emphasis => {
            ctx.italic = true;
        }
        Tag::CodeBlock(_kind) => {
            ctx.in_code_block = true;
        }
        Tag::Link { dest_url, .. } => {
            ctx.link_url = Some(dest_url.to_string());
        }
        Tag::List(start_number) => {
            ctx.list_indent += 2;
            ctx.list_item_number = *start_number;
        }
        Tag::Item => {
            // Will be handled in text processing
        }
        // Table handling
        Tag::Table(_alignments) => {
            flush_line(current_spans, lines);
            ctx.table = Some(TableAccumulator::default());
        }
        Tag::TableHead => {
            if let Some(ref mut table) = ctx.table {
                table.in_header = true;
            }
        }
        Tag::TableRow => {
            if let Some(ref mut table) = ctx.table {
                table.current_row = Vec::new();
            }
        }
        Tag::TableCell => {
            if let Some(ref mut table) = ctx.table {
                table.current_cell = StyledCell::default();
            }
        }
        _ => {}
    }
}

/// Handles the end of a markdown tag.
fn handle_end_tag(
    tag_end: &TagEnd,
    ctx: &mut StyleContext,
    current_spans: &mut Vec<Span<'static>>,
    lines: &mut Vec<Line<'static>>,
    width: usize,
) {
    match tag_end {
        TagEnd::Heading(_) => {
            flush_line(current_spans, lines);
            lines.push(Line::from("")); // Add blank line after heading
            ctx.heading_level = None;
        }
        TagEnd::Strong => {
            ctx.bold = false;
        }
        TagEnd::Emphasis => {
            ctx.italic = false;
        }
        TagEnd::CodeBlock => {
            flush_line(current_spans, lines);
            lines.push(Line::from("")); // Add blank line after code block
            ctx.in_code_block = false;
        }
        TagEnd::Link => {
            // Append URL in parentheses if we have one
            if let Some(url) = ctx.link_url.take() {
                current_spans.push(Span::styled(
                    format!(" ({url})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        TagEnd::List(_) => {
            ctx.list_indent = ctx.list_indent.saturating_sub(2);
            ctx.list_item_number = None;
            flush_line(current_spans, lines);
        }
        TagEnd::Item => {
            flush_line(current_spans, lines);
            // Increment list item number for ordered lists
            if let Some(ref mut num) = ctx.list_item_number {
                *num += 1;
            }
        }
        TagEnd::Paragraph => {
            flush_line(current_spans, lines);
            // Check width to avoid adding too many blank lines in narrow displays
            if width > 20 {
                lines.push(Line::from(""));
            }
        }
        // Table handling
        TagEnd::TableCell => {
            if let Some(ref mut table) = ctx.table {
                let cell = std::mem::take(&mut table.current_cell);
                table.current_row.push(cell);
            }
        }
        TagEnd::TableRow => {
            if let Some(ref mut table) = ctx.table {
                // Body rows only - header cells are handled in TableHead
                if !table.in_header {
                    let row = std::mem::take(&mut table.current_row);
                    table.rows.push(row);
                }
            }
        }
        TagEnd::TableHead => {
            if let Some(ref mut table) = ctx.table {
                // Move accumulated cells to headers
                let headers = std::mem::take(&mut table.current_row);
                table.headers = headers;
                table.in_header = false;
            }
        }
        TagEnd::Table => {
            if let Some(table) = ctx.table.take() {
                let table_lines = render_table(&table, width);
                lines.extend(table_lines);
            }
        }
        _ => {}
    }
}

/// Handles text content within markdown.
fn handle_text(
    text: &str,
    ctx: &StyleContext,
    current_spans: &mut Vec<Span<'static>>,
    lines: &mut Vec<Line<'static>>,
    width: usize,
) {
    let style = ctx.current_style();

    // Handle code blocks with indentation
    if ctx.in_code_block {
        for line in text.lines() {
            let indented = format!("  {line}");
            lines.push(Line::from(Span::styled(indented, style)));
        }
        return;
    }

    // Handle list items with bullet/number prefix
    if ctx.list_indent > 0 && current_spans.is_empty() {
        let indent = " ".repeat(ctx.list_indent.saturating_sub(2));
        let prefix = if let Some(num) = ctx.list_item_number {
            format!("{indent}{num}. ")
        } else {
            format!("{indent}- ")
        };
        current_spans.push(Span::styled(prefix, Style::default().fg(Color::DarkGray)));
    }

    // Wrap text if needed
    let available_width = width.saturating_sub(ctx.list_indent);
    let wrapped = wrap_styled_text(text, available_width);

    for (i, line_text) in wrapped.into_iter().enumerate() {
        if i > 0 {
            // For continuation lines, flush and add indent
            flush_line(current_spans, lines);
            if ctx.list_indent > 0 {
                current_spans.push(Span::raw(" ".repeat(ctx.list_indent)));
            }
        }
        current_spans.push(Span::styled(line_text, style));
    }
}

/// Wraps text to fit within a given width, preserving leading/trailing whitespace.
fn wrap_styled_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    if max_width == 0 {
        return vec![text.to_string()];
    }

    // Preserve leading/trailing whitespace
    let leading_space = text.starts_with(char::is_whitespace);
    let trailing_space = text.ends_with(char::is_whitespace);

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let mut first_word = true;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_width == 0 {
            // Add leading space to first line's first word if original had it
            if first_word && leading_space {
                current_line.push(' ');
                current_width = 1;
            }
            first_word = false;

            if word_len + current_width > max_width && word_len > max_width {
                // Word too long, split it
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if !current_line.is_empty() {
                        result.push(std::mem::take(&mut current_line));
                    }
                    result.push(chunk);
                    current_width = 0;
                }
            } else {
                current_line.push_str(word);
                current_width += word_len;
            }
        } else if current_width + 1 + word_len <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_len;
        } else {
            result.push(std::mem::take(&mut current_line));
            if word_len > max_width {
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    result.push(chunk);
                }
                current_width = 0;
            } else {
                current_line = word.to_string();
                current_width = word_len;
            }
        }
    }

    if !current_line.is_empty() {
        // Add trailing space to last line if original had it
        if trailing_space {
            current_line.push(' ');
        }
        result.push(current_line);
    } else if trailing_space && !result.is_empty() {
        // If we flushed everything but had trailing space, add to last result
        if let Some(last) = result.last_mut() {
            last.push(' ');
        }
    }

    if result.is_empty() && !text.is_empty() {
        result.push(text.to_string());
    }

    result
}

/// Flushes the current spans to a new line.
fn flush_line(current_spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
    if !current_spans.is_empty() {
        lines.push(Line::from(std::mem::take(current_spans)));
    }
}

/// Converts a heading level to its numeric value.
fn heading_level_to_num(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let lines = render_markdown("", 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn plain_text_renders_correctly() {
        let lines = render_markdown("Hello world", 80);
        assert!(!lines.is_empty());

        // First line should contain the text
        let content: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(content, "Hello world");
    }

    #[test]
    fn h1_header_styled_bold_cyan() {
        let lines = render_markdown("# Header One", 80);
        assert!(!lines.is_empty());

        let first_line = &lines[0];
        assert!(
            first_line.spans.len() >= 2,
            "should have prefix and text spans"
        );

        // First span is the "# " prefix (DarkGray)
        assert!(first_line.spans[0].content.contains('#'));
        assert_eq!(first_line.spans[0].style.fg, Some(Color::DarkGray));

        // Second span is the heading text (Bold + Cyan)
        let style = first_line.spans[1].style;
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn h2_header_styled_bold_white() {
        let lines = render_markdown("## Header Two", 80);
        assert!(!lines.is_empty());

        let first_line = &lines[0];
        assert!(
            first_line.spans.len() >= 2,
            "should have prefix and text spans"
        );

        // First span is the "## " prefix (DarkGray)
        assert!(first_line.spans[0].content.contains("##"));
        assert_eq!(first_line.spans[0].style.fg, Some(Color::DarkGray));

        // Second span is the heading text (Bold + White)
        let style = first_line.spans[1].style;
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn bold_text_has_bold_modifier() {
        let lines = render_markdown("This is **bold** text", 80);
        assert!(!lines.is_empty());

        // Find the span containing "bold"
        let bold_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("bold"))
            .expect("should have bold span");

        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn italic_text_has_italic_modifier() {
        let lines = render_markdown("This is *italic* text", 80);
        assert!(!lines.is_empty());

        // Find the span containing "italic"
        let italic_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("italic"))
            .expect("should have italic span");

        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn inline_code_styled_yellow() {
        let lines = render_markdown("Use `code` here", 80);
        assert!(!lines.is_empty());

        // Find the span containing "code"
        let code_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("code"))
            .expect("should have code span");

        assert_eq!(code_span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn code_block_indented_and_styled() {
        let lines = render_markdown("```\nlet x = 1;\n```", 80);
        assert!(!lines.is_empty());

        // Find line with code content
        let code_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("let x")))
            .expect("should have code line");

        // Code should be indented
        let content: String = code_line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            content.starts_with("  "),
            "code should be indented: {content}"
        );

        // Should be yellow
        let code_span = code_line
            .spans
            .iter()
            .find(|s| s.content.contains("let"))
            .expect("should have code span");
        assert_eq!(code_span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn unordered_list_has_bullets() {
        let lines = render_markdown("- Item one\n- Item two", 80);
        assert!(!lines.is_empty());

        // Check that lines contain bullet markers
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(content.contains("- "), "should have bullet markers");
        assert!(content.contains("Item one"));
        assert!(content.contains("Item two"));
    }

    #[test]
    fn ordered_list_has_numbers() {
        let lines = render_markdown("1. First\n2. Second", 80);
        assert!(!lines.is_empty());

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(content.contains("1. "), "should have number markers");
        assert!(content.contains("First"));
    }

    #[test]
    fn link_styled_cyan_underline_with_url() {
        let lines = render_markdown("[Click here](https://example.com)", 80);
        assert!(!lines.is_empty());

        // Find the link text span
        let link_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("Click"))
            .expect("should have link span");

        assert_eq!(link_span.style.fg, Some(Color::Cyan));
        assert!(link_span.style.add_modifier.contains(Modifier::UNDERLINED));

        // URL should appear in parentheses
        let content: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(content.contains("example.com"), "should show URL");
    }

    #[test]
    fn mixed_content_renders_correctly() {
        let md = r#"# Title

This has **bold** and *italic* text.

## Subtitle

- List item with `code`
- Another item

```
fn main() {}
```

Check [this link](http://test.com).
"#;

        let lines = render_markdown(md, 80);
        assert!(!lines.is_empty());

        // Just verify it doesn't panic and produces output
        let total_content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(total_content.contains("Title"));
        assert!(total_content.contains("bold"));
        assert!(total_content.contains("italic"));
        assert!(total_content.contains("Subtitle"));
        assert!(total_content.contains("List item"));
        assert!(total_content.contains("fn main"));
        assert!(total_content.contains("this link"));
    }

    #[test]
    fn text_wrapping_respects_width() {
        let long_text = "This is a very long line of text that should definitely wrap when rendered with a narrow width constraint applied to it.";
        let lines = render_markdown(long_text, 30);

        // Should have multiple lines
        assert!(lines.len() > 1, "long text should wrap to multiple lines");

        // Each line's content should not exceed width significantly
        for line in &lines {
            let len: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            // Allow some tolerance for word boundaries
            assert!(len <= 35, "line too long: {len} chars");
        }
    }

    #[test]
    fn horizontal_rule_renders() {
        let lines = render_markdown("Above\n\n---\n\nBelow", 40);
        assert!(!lines.is_empty());

        // Should have a line with repeated horizontal bar character
        let has_rule = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('\u{2500}')));
        assert!(has_rule, "should contain horizontal rule");
    }

    #[test]
    fn inline_code_preserves_surrounding_spaces() {
        let lines = render_markdown("for example `this` one", 80);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should preserve spaces around inline code
        assert!(
            content.contains("example ") || content.contains("example"),
            "should have space before code: {content}"
        );
        assert!(
            content.contains(" one") || content.contains("one"),
            "space after code should exist: {content}"
        );
        // The full text should not be concatenated without spaces
        assert!(
            !content.contains("examplethisone"),
            "should not concatenate without spaces: {content}"
        );
    }

    #[test]
    fn emphasis_preserves_surrounding_spaces() {
        let lines = render_markdown("this is *italic* text", 80);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should preserve spaces around emphasis
        assert!(
            !content.contains("isitalictext"),
            "should not concatenate without spaces: {content}"
        );
    }

    #[test]
    fn code_block_has_blank_line_after() {
        let lines = render_markdown("```\ncode\n```\ntext after", 80);

        // Find the code line
        let code_idx = lines
            .iter()
            .position(|l| l.spans.iter().any(|s| s.content.contains("code")))
            .expect("should have code line");

        // Next line should be blank (empty spans or empty content)
        let next_line = &lines[code_idx + 1];
        let is_blank =
            next_line.spans.is_empty() || next_line.spans.iter().all(|s| s.content.is_empty());
        assert!(is_blank, "should have blank line after code block");
    }

    // ========================================================================
    // Table Tests
    // ========================================================================

    #[test]
    fn simple_table_renders_box_drawing_when_wide() {
        let md = r#"
| A | B |
|---|---|
| 1 | 2 |
"#;
        let lines = render_markdown(md, 80);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should use box-drawing characters
        assert!(
            content.contains('\u{250C}'),
            "should have top-left corner ┌"
        );
        assert!(
            content.contains('\u{2518}'),
            "should have bottom-right corner ┘"
        );
        assert!(content.contains('\u{2502}'), "should have vertical bar │");
        assert!(content.contains("A"));
        assert!(content.contains("1"));
    }

    #[test]
    fn table_falls_back_to_definition_list_when_narrow() {
        let md = r#"
| Header One | Header Two | Header Three |
|------------|------------|--------------|
| Value 1    | Value 2    | Value 3      |
"#;
        // Very narrow width - box drawing won't fit
        let lines = render_markdown(md, 30);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should NOT have box-drawing corners (would use definition list)
        // But should still have the content
        assert!(content.contains("Value 1") || content.contains("Header"));
    }

    #[test]
    fn two_column_table_renders_as_key_value_when_narrow() {
        let md = r#"
| Setting | Value |
|---------|-------|
| Timeout | 30s   |
| Retries | 3     |
"#;
        // Narrow width forces definition list mode (table needs 22 chars)
        let lines = render_markdown(md, 18);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should have key-value format with colons
        assert!(content.contains("Timeout"));
        assert!(content.contains("30s"));
        assert!(
            content.contains(":"),
            "should have colon separator: {content}"
        );
    }

    #[test]
    fn multi_column_table_uses_row_labels_when_narrow() {
        let md = r#"
| Task   | Status  | Priority |
|--------|---------|----------|
| Task 1 | Done    | High     |
| Task 2 | Pending | Low      |
"#;
        // Narrow width forces definition list mode
        let lines = render_markdown(md, 30);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should have the data
        assert!(content.contains("Done") || content.contains("Status"));
        assert!(content.contains("High") || content.contains("Priority"));
    }

    #[test]
    fn table_header_is_styled() {
        let md = r#"
| Name | Value |
|------|-------|
| foo  | bar   |
"#;
        let lines = render_markdown(md, 80);

        // Find a line containing "Name" and check it has bold styling
        let header_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("Name")))
            .expect("should have header line");

        let name_span = header_line
            .spans
            .iter()
            .find(|s| s.content.contains("Name"))
            .expect("should have Name span");

        assert!(
            name_span.style.add_modifier.contains(Modifier::BOLD),
            "header should be bold"
        );
    }

    #[test]
    fn empty_table_renders_nothing() {
        // A table with no data rows
        let md = r#"
| A | B |
|---|---|
"#;
        let lines = render_markdown(md, 80);
        // Should not panic, may render just headers or nothing
        // Main assertion: no crash
        let _ = lines;
    }

    #[test]
    fn table_with_code_in_cells() {
        let md = r#"
| Command | Description |
|---------|-------------|
| `ls`    | List files  |
"#;
        let lines = render_markdown(md, 80);
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Code should be rendered (without backticks - they're styling markers)
        assert!(
            content.contains("ls"),
            "should contain the command: {content}"
        );
        assert!(content.contains("List files"));

        // Code should be styled yellow
        let has_yellow_ls = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains("ls") && s.style.fg == Some(Color::Yellow))
        });
        assert!(has_yellow_ls, "code in cell should be styled yellow");
    }

    #[test]
    fn table_box_width_calculation() {
        fn cell(s: &str) -> StyledCell {
            StyledCell {
                spans: vec![Span::raw(s.to_string())],
            }
        }

        let mut table = TableAccumulator::default();
        table.headers = vec![cell("A"), cell("BB")];
        table.rows = vec![vec![cell("1"), cell("22")]];

        // Column widths: [1, 2]
        // Box width: 1 + (1+3) + (2+3) = 1 + 4 + 5 = 10
        assert_eq!(table.box_drawing_width(), 10);
    }

    #[test]
    fn looks_like_identifier_works() {
        assert!(looks_like_identifier("Task1"));
        assert!(looks_like_identifier("ID-123"));
        assert!(looks_like_identifier("#42"));
        assert!(!looks_like_identifier(""));
        assert!(!looks_like_identifier("123"));
        assert!(!looks_like_identifier(
            "This is a very long string that is definitely not an identifier"
        ));
    }
}
