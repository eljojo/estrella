use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::components::Component;
use crate::ir::Op;
use crate::protocol::text::{Alignment, Font};

/// Markdown text that gets parsed and converted to IR operations.
///
/// Supports:
/// - Basic formatting: **bold**, *italic*, `code`
/// - Headers: # H1 through ###### H6
/// - Lists: bullets (`-`, `*`) and numbered (`1.`, `2.`)
/// - Links: `[text](url)` with emphasis
///
/// # Example
///
/// ```rust
/// use estrella::components::*;
///
/// let receipt = Receipt::new()
///     .child(Markdown::new(r#"
/// # My Receipt
///
/// **Total**: $4.50
///
/// Items:
/// - Coffee
/// - Muffin
///     "#))
///     .cut();
/// ```
pub struct Markdown {
    text: String,
    show_urls: bool, // Optional: append URLs in links
}

impl Markdown {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            show_urls: false,
        }
    }

    pub fn show_urls(mut self) -> Self {
        self.show_urls = true;
        self
    }
}

impl Component for Markdown {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Skip empty content
        if self.text.trim().is_empty() {
            return;
        }

        // Ensure we start in a known state (left-aligned)
        ops.push(Op::SetAlign(Alignment::Left));

        let parser = Parser::new(&self.text);
        let mut state = ParserState::new(self.show_urls);

        for event in parser {
            match event {
                Event::Start(tag) => state.handle_start_tag(tag, ops),
                Event::End(tag_end) => state.handle_end_tag(tag_end, ops),
                Event::Text(text) => state.handle_text(&text, ops),
                Event::Code(code) => state.handle_inline_code(&code, ops),
                Event::SoftBreak => ops.push(Op::Text(" ".into())),
                Event::HardBreak => ops.push(Op::Newline),
                Event::Rule => {
                    // Horizontal rule: emit divider
                    ops.push(Op::Text("─".repeat(48)));
                    ops.push(Op::Newline);
                }
                _ => {} // Ignore HTML, task lists, footnotes, etc.
            }
        }
    }
}

/// Internal state for tracking nested formatting during parsing
struct ParserState {
    show_urls: bool,
    list_depth: usize,
    list_counters: Vec<usize>, // Stack of counters for ordered lists
    pending_url: Option<String>,
    pending_list_prefix: Option<String>, // Bullet or number to prepend to next text
    in_heading: bool,
    heading_level: Option<HeadingLevel>,
    just_finished_heading: bool, // Track if we just finished a heading
}

impl ParserState {
    fn new(show_urls: bool) -> Self {
        Self {
            show_urls,
            list_depth: 0,
            list_counters: Vec::new(),
            pending_url: None,
            pending_list_prefix: None,
            in_heading: false,
            heading_level: None,
            just_finished_heading: false,
        }
    }

    fn handle_start_tag(&mut self, tag: Tag, ops: &mut Vec<Op>) {
        match tag {
            Tag::Paragraph => {
                // Add extra spacing if paragraph follows a heading
                if self.just_finished_heading {
                    ops.push(Op::Newline);
                    self.just_finished_heading = false;
                }
            }
            Tag::Heading { level, .. } => {
                self.in_heading = true;
                self.heading_level = Some(level);

                match level {
                    HeadingLevel::H1 => {
                        ops.push(Op::SetAlign(Alignment::Center));
                        ops.push(Op::SetBold(true));
                        ops.push(Op::SetSize {
                            height: 3,
                            width: 3,
                        });
                        ops.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H2 => {
                        ops.push(Op::SetAlign(Alignment::Center));
                        ops.push(Op::SetBold(true));
                        ops.push(Op::SetSize {
                            height: 2,
                            width: 2,
                        });
                        ops.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H3 => {
                        ops.push(Op::SetBold(true));
                    }
                    _ => {
                        ops.push(Op::SetBold(true));
                    }
                }
            }
            Tag::Strong => {
                ops.push(Op::SetBold(true));
            }
            Tag::Emphasis => {
                ops.push(Op::SetUnderline(true));
            }
            Tag::Link { dest_url, .. } => {
                ops.push(Op::SetUnderline(true));
                if self.show_urls {
                    self.pending_url = Some(dest_url.to_string());
                }
            }
            Tag::List(start_num) => {
                if let Some(start) = start_num {
                    self.list_counters.push(start as usize);
                }
                self.list_depth += 1;
                // Add extra spacing before list if at top level
                if self.list_depth == 1 {
                    ops.push(Op::Newline);
                }
            }
            Tag::Item => {
                // Emit indentation
                let indent_dots = (self.list_depth - 1) * 32; // 2 chars * 16 dots/char
                if indent_dots > 0 {
                    ops.push(Op::SetAbsolutePosition(indent_dots as u16));
                }

                // Store the bullet/number prefix to prepend to first text
                if let Some(counter) = self.list_counters.last_mut() {
                    // Ordered list
                    self.pending_list_prefix = Some(format!("{}. ", counter));
                    *counter += 1;
                } else {
                    // Unordered list - use asterisk for better compatibility
                    self.pending_list_prefix = Some("* ".into());
                }
            }
            Tag::CodeBlock(_kind) => {
                // Code blocks: use inverted text
                ops.push(Op::SetInvert(true));
                ops.push(Op::SetFont(Font::B));
            }
            _ => {} // BlockQuote, Table, etc. - ignore for now
        }
    }

    fn handle_end_tag(&mut self, tag_end: TagEnd, ops: &mut Vec<Op>) {
        match tag_end {
            TagEnd::Paragraph => {
                ops.push(Op::Newline);
                ops.push(Op::Newline); // Blank line between paragraphs
            }
            TagEnd::Heading(_level) => {
                self.in_heading = false;

                match self.heading_level {
                    Some(HeadingLevel::H1) => {
                        // Newline FIRST (printer buffers line, applies styles at newline)
                        ops.push(Op::Newline);
                        // Then reset styles
                        ops.push(Op::SetSize {
                            height: 0,
                            width: 0,
                        });
                        ops.push(Op::SetBold(false));
                        ops.push(Op::SetSmoothing(false));
                        ops.push(Op::SetAlign(Alignment::Left));
                        ops.push(Op::Feed { units: 12 }); // 3mm spacing
                    }
                    Some(HeadingLevel::H2) => {
                        // Newline FIRST (printer buffers line, applies styles at newline)
                        ops.push(Op::Newline);
                        // Then reset styles
                        ops.push(Op::SetSize {
                            height: 0,
                            width: 0,
                        });
                        ops.push(Op::SetBold(false));
                        ops.push(Op::SetSmoothing(false));
                        ops.push(Op::SetAlign(Alignment::Left));
                        ops.push(Op::Feed { units: 6 }); // 1.5mm spacing
                    }
                    Some(HeadingLevel::H3) => {
                        ops.push(Op::Newline);
                        ops.push(Op::SetBold(false));
                        ops.push(Op::Feed { units: 4 }); // 1mm spacing
                    }
                    _ => {
                        ops.push(Op::Newline);
                        ops.push(Op::SetBold(false));
                    }
                }

                self.heading_level = None;
                self.just_finished_heading = true;
            }
            TagEnd::Strong => {
                ops.push(Op::SetBold(false));
            }
            TagEnd::Emphasis => {
                ops.push(Op::SetUnderline(false));
            }
            TagEnd::Link => {
                ops.push(Op::SetUnderline(false));

                if let Some(url) = self.pending_url.take() {
                    ops.push(Op::Text(" (".into()));
                    ops.push(Op::SetFont(Font::C));
                    ops.push(Op::Text(url));
                    ops.push(Op::SetFont(Font::A));
                    ops.push(Op::Text(")".into()));
                }
            }
            TagEnd::List(_is_ordered) => {
                if !self.list_counters.is_empty() {
                    self.list_counters.pop();
                }
                self.list_depth = self.list_depth.saturating_sub(1);
                ops.push(Op::Newline); // Blank line after list
            }
            TagEnd::Item => {
                ops.push(Op::Newline);
                // Reset position to left margin
                ops.push(Op::SetAbsolutePosition(0));
            }
            TagEnd::CodeBlock => {
                ops.push(Op::SetInvert(false));
                ops.push(Op::SetFont(Font::A));
                ops.push(Op::Newline);
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str, ops: &mut Vec<Op>) {
        // If we have a pending list prefix, prepend it to this text
        if let Some(prefix) = self.pending_list_prefix.take() {
            ops.push(Op::Text(format!("{}{}", prefix, text)));
        } else {
            ops.push(Op::Text(text.to_string()));
        }
    }

    fn handle_inline_code(&mut self, code: &str, ops: &mut Vec<Op>) {
        ops.push(Op::SetInvert(true));
        ops.push(Op::Text(code.to_string()));
        ops.push(Op::SetInvert(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to compile markdown to IR ops
    fn compile_markdown(text: &str) -> Vec<Op> {
        let md = Markdown::new(text);
        let mut ops = Vec::new();
        md.emit(&mut ops);
        ops
    }

    #[test]
    fn test_basic_bold() {
        let ops = compile_markdown("**bold text**");

        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::Text("bold text".into())));
        assert!(ops.contains(&Op::SetBold(false)));
    }

    #[test]
    fn test_basic_italic() {
        let ops = compile_markdown("*italic text*");

        assert!(ops.contains(&Op::SetUnderline(true)));
        assert!(ops.contains(&Op::Text("italic text".into())));
        assert!(ops.contains(&Op::SetUnderline(false)));
    }

    #[test]
    fn test_heading_h1() {
        let ops = compile_markdown("# Title");

        assert!(ops.contains(&Op::SetAlign(Alignment::Center)));
        assert!(ops.contains(&Op::SetSize {
            height: 3,
            width: 3
        }));
        assert!(ops.contains(&Op::Text("Title".into())));
        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::SetSmoothing(true)));
    }

    #[test]
    fn test_heading_h2() {
        let ops = compile_markdown("## Subtitle");

        assert!(ops.contains(&Op::SetAlign(Alignment::Center)));
        assert!(ops.contains(&Op::SetSize {
            height: 2,
            width: 2
        }));
        assert!(ops.contains(&Op::Text("Subtitle".into())));
    }

    #[test]
    fn test_heading_h3() {
        let ops = compile_markdown("### Section");

        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::Text("Section".into())));
        assert!(ops.contains(&Op::SetBold(false)));
        // H3 should not center or resize
        assert!(!ops.contains(&Op::SetAlign(Alignment::Center)));
    }

    #[test]
    fn test_unordered_list() {
        let ops = compile_markdown("- Item 1\n- Item 2");

        // Should contain bullet characters
        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        assert!(text_ops.iter().any(|s: &&str| s.contains("* ")));
    }

    #[test]
    fn test_ordered_list() {
        let ops = compile_markdown("1. First\n2. Second");

        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        // Check for combined text (bullet/number + content)
        assert!(text_ops.iter().any(|s: &&str| s.starts_with("1. ")));
        assert!(text_ops.iter().any(|s: &&str| s.starts_with("2. ")));
    }

    #[test]
    fn test_inline_code() {
        let ops = compile_markdown("`code`");

        assert!(ops.contains(&Op::SetInvert(true)));
        assert!(ops.contains(&Op::Text("code".into())));
        assert!(ops.contains(&Op::SetInvert(false)));
    }

    #[test]
    fn test_link_underline() {
        let ops = compile_markdown("[click here](https://example.com)");

        assert!(ops.contains(&Op::SetUnderline(true)));
        assert!(ops.contains(&Op::Text("click here".into())));
        assert!(ops.contains(&Op::SetUnderline(false)));

        // Should not show URL by default
        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(!text_ops.iter().any(|s: &&str| s.contains("https://")));
    }

    #[test]
    fn test_link_with_url_shown() {
        let md = Markdown::new("[click here](https://example.com)").show_urls();
        let mut ops = Vec::new();
        md.emit(&mut ops);

        // Should show URL after link text
        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(text_ops.iter().any(|s: &&str| s.contains("https://example.com")));
    }

    #[test]
    fn test_nested_formatting() {
        let ops = compile_markdown("**bold and *italic* text**");

        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::SetUnderline(true)));
        assert!(ops.contains(&Op::SetUnderline(false)));
        assert!(ops.contains(&Op::SetBold(false)));
    }

    #[test]
    fn test_paragraph_spacing() {
        let ops = compile_markdown("First paragraph.\n\nSecond paragraph.");

        // Count newlines - should have double newlines between paragraphs
        let newline_count = ops.iter().filter(|op| matches!(op, Op::Newline)).count();
        assert!(newline_count >= 4); // Two paragraphs = 4+ newlines
    }

    #[test]
    fn test_horizontal_rule() {
        let ops = compile_markdown("---");

        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        assert!(text_ops.iter().any(|s: &&str| s.contains("─")));
    }

    #[test]
    fn test_code_block() {
        let ops = compile_markdown("```\ncode block\n```");

        assert!(ops.contains(&Op::SetInvert(true)));
        assert!(ops.contains(&Op::SetFont(Font::B)));
        // Code block text may include newlines as separate ops
        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(text_ops.iter().any(|s: &&str| s.contains("code block")));
        assert!(ops.contains(&Op::SetInvert(false)));
        assert!(ops.contains(&Op::SetFont(Font::A)));
    }

    #[test]
    fn test_nested_list() {
        let ops = compile_markdown("- Item 1\n  - Nested");

        // Should have position commands for indentation
        assert!(ops.iter().any(|op| matches!(op, Op::SetAbsolutePosition(_))));
    }

    #[test]
    fn test_empty_markdown() {
        let ops = compile_markdown("");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_complex_receipt() {
        let md = r#"
# Coffee Shop Receipt

**Date**: 2024-01-15
**Order #**: 1234

## Items Ordered

- Espresso ($3.50)
- Croissant ($4.00)

---

**Total**: $8.00

Thank you!
        "#;

        let ops = compile_markdown(md);

        // Verify key elements are present
        assert!(ops.contains(&Op::SetSize {
            height: 3,
            width: 3
        })); // H1
        assert!(ops.contains(&Op::SetSize {
            height: 2,
            width: 2
        })); // H2

        let text_ops: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        assert!(text_ops.contains(&"Coffee Shop Receipt"));
        assert!(text_ops.contains(&"Items Ordered"));
        assert!(text_ops.iter().any(|s: &&str| s.contains("$8.00")));
        assert!(text_ops.iter().any(|s: &&str| s.contains("* "))); // Bullet
    }
}
