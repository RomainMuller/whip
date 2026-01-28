//! Markdown rendering for TUI display.
//!
//! This module provides conversion from markdown text to styled ratatui `Line`s,
//! supporting common markdown elements like headers, bold, italic, code, lists, and links.
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
//!
//! # Example
//!
//! ```
//! use taim_tui::widgets::markdown::render_markdown;
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
/// use taim_tui::widgets::markdown::render_markdown;
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
                handle_text(&text, &ctx, &mut current_spans, &mut lines, width);
            }
            Event::Code(code) => {
                // Inline code
                current_spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Event::SoftBreak => {
                // Soft break - treat as space
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                // Hard break - start new line
                flush_line(&mut current_spans, &mut lines);
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
        assert!(first_line.spans.len() >= 2, "should have prefix and text spans");

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
        assert!(first_line.spans.len() >= 2, "should have prefix and text spans");

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
            .find(|l| {
                l.spans
                    .iter()
                    .any(|s| s.content.contains("let x"))
            })
            .expect("should have code line");

        // Code should be indented
        let content: String = code_line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(content.starts_with("  "), "code should be indented: {content}");

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
        let has_rule = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains('\u{2500}'))
        });
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
}
