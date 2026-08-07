//! Emit logic for the Markdown component.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use super::EmitContext;
use super::types::Markdown;
use crate::ir::Op;
use crate::protocol::text::{Alignment, Font};

impl Markdown {
    /// Emit IR ops for this markdown component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        // Skip empty content
        if self.content.trim().is_empty() {
            return;
        }

        // Ensure we start in a known state (left-aligned)
        ctx.push(Op::SetAlign(Alignment::Left));

        let parser = Parser::new(&self.content);
        let mut state = ParserState::new(self.show_urls);

        for event in parser {
            match event {
                Event::Start(tag) => state.handle_start_tag(tag, ctx),
                Event::End(tag_end) => state.handle_end_tag(tag_end, ctx),
                Event::Text(text) => state.handle_text(&text, ctx),
                Event::Code(code) => state.handle_inline_code(&code, ctx),
                Event::SoftBreak => ctx.push(Op::Text(" ".into())),
                Event::HardBreak => ctx.push(Op::Newline),
                Event::Rule => {
                    ctx.push(Op::Text("\u{2500}".repeat(ctx.chars_per_line())));
                    ctx.push(Op::Newline);
                }
                _ => {}
            }
        }
    }
}

/// Internal state for tracking nested formatting during parsing.
struct ParserState {
    show_urls: bool,
    list_depth: usize,
    list_counters: Vec<usize>,
    pending_url: Option<String>,
    pending_list_prefix: Option<String>,
    in_heading: bool,
    heading_level: Option<HeadingLevel>,
    just_finished_heading: bool,
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

    fn handle_start_tag(&mut self, tag: Tag, ctx: &mut EmitContext) {
        match tag {
            Tag::Paragraph => {
                if self.just_finished_heading {
                    ctx.push(Op::Newline);
                    self.just_finished_heading = false;
                }
            }
            Tag::Heading { level, .. } => {
                self.in_heading = true;
                self.heading_level = Some(level);

                match level {
                    HeadingLevel::H1 => {
                        ctx.push(Op::SetAlign(Alignment::Center));
                        ctx.push(Op::SetBold(true));
                        ctx.push(Op::SetSize {
                            height: 3,
                            width: 3,
                        });
                        ctx.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H2 => {
                        ctx.push(Op::SetAlign(Alignment::Center));
                        ctx.push(Op::SetBold(true));
                        ctx.push(Op::SetSize {
                            height: 2,
                            width: 2,
                        });
                        ctx.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H3 => {
                        ctx.push(Op::SetBold(true));
                        ctx.push(Op::SetSize {
                            height: 1,
                            width: 1,
                        });
                        ctx.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H4 => {
                        ctx.push(Op::SetBold(true));
                        ctx.push(Op::SetExpandedHeight(1));
                        ctx.push(Op::SetSmoothing(true));
                    }
                    HeadingLevel::H5 => {
                        ctx.push(Op::SetBold(true));
                    }
                    HeadingLevel::H6 => {
                        ctx.push(Op::SetFont(Font::B));
                        ctx.push(Op::SetBold(true));
                    }
                }
            }
            Tag::Strong => {
                ctx.push(Op::SetBold(true));
            }
            Tag::Emphasis => {
                ctx.push(Op::SetUnderline(true));
            }
            Tag::Link { dest_url, .. } => {
                ctx.push(Op::SetUnderline(true));
                if self.show_urls {
                    self.pending_url = Some(dest_url.to_string());
                }
            }
            Tag::List(start_num) => {
                if let Some(start) = start_num {
                    self.list_counters.push(start as usize);
                }
                self.list_depth += 1;
                if self.list_depth == 1 {
                    ctx.push(Op::Newline);
                }
            }
            Tag::Item => {
                let indent_dots = (self.list_depth - 1) * 32;
                if indent_dots > 0 {
                    ctx.push(Op::SetAbsolutePosition(indent_dots as u16));
                }

                if let Some(counter) = self.list_counters.last_mut() {
                    self.pending_list_prefix = Some(format!("{}. ", counter));
                    *counter += 1;
                } else {
                    self.pending_list_prefix = Some("* ".into());
                }
            }
            Tag::CodeBlock(_kind) => {
                ctx.push(Op::SetInvert(true));
                ctx.push(Op::SetFont(Font::B));
            }
            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag_end: TagEnd, ctx: &mut EmitContext) {
        match tag_end {
            TagEnd::Paragraph => {
                ctx.push(Op::Newline);
                ctx.push(Op::Newline);
            }
            TagEnd::Heading(_level) => {
                self.in_heading = false;

                match self.heading_level {
                    Some(HeadingLevel::H1) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetSize {
                            height: 0,
                            width: 0,
                        });
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::SetSmoothing(false));
                        ctx.push(Op::SetAlign(Alignment::Left));
                        ctx.push(Op::Feed { units: 12 });
                    }
                    Some(HeadingLevel::H2) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetSize {
                            height: 0,
                            width: 0,
                        });
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::SetSmoothing(false));
                        ctx.push(Op::SetAlign(Alignment::Left));
                        ctx.push(Op::Feed { units: 6 });
                    }
                    Some(HeadingLevel::H3) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetSize {
                            height: 0,
                            width: 0,
                        });
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::SetSmoothing(false));
                        ctx.push(Op::Feed { units: 4 });
                    }
                    Some(HeadingLevel::H4) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetExpandedHeight(0));
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::SetSmoothing(false));
                        ctx.push(Op::Feed { units: 4 });
                    }
                    Some(HeadingLevel::H5) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::Feed { units: 2 });
                    }
                    Some(HeadingLevel::H6) => {
                        ctx.push(Op::Newline);
                        ctx.push(Op::SetBold(false));
                        ctx.push(Op::SetFont(Font::A));
                    }
                    None => {
                        ctx.push(Op::Newline);
                    }
                }

                self.heading_level = None;
                self.just_finished_heading = true;
            }
            TagEnd::Strong => {
                ctx.push(Op::SetBold(false));
            }
            TagEnd::Emphasis => {
                ctx.push(Op::SetUnderline(false));
            }
            TagEnd::Link => {
                ctx.push(Op::SetUnderline(false));

                if let Some(url) = self.pending_url.take() {
                    ctx.push(Op::Text(" (".into()));
                    ctx.push(Op::SetFont(Font::C));
                    ctx.push(Op::Text(url));
                    ctx.push(Op::SetFont(Font::A));
                    ctx.push(Op::Text(")".into()));
                }
            }
            TagEnd::List(_is_ordered) => {
                if !self.list_counters.is_empty() {
                    self.list_counters.pop();
                }
                self.list_depth = self.list_depth.saturating_sub(1);
                ctx.push(Op::Newline);
            }
            TagEnd::Item => {
                ctx.push(Op::Newline);
                ctx.push(Op::SetAbsolutePosition(0));
            }
            TagEnd::CodeBlock => {
                ctx.push(Op::SetInvert(false));
                ctx.push(Op::SetFont(Font::A));
                ctx.push(Op::Newline);
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str, ctx: &mut EmitContext) {
        if let Some(prefix) = self.pending_list_prefix.take() {
            ctx.push(Op::Text(format!("{}{}", prefix, text)));
        } else {
            ctx.push(Op::Text(text.to_string()));
        }
    }

    fn handle_inline_code(&mut self, code: &str, ctx: &mut EmitContext) {
        ctx.push(Op::SetInvert(true));
        ctx.push(Op::Text(code.to_string()));
        ctx.push(Op::SetInvert(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EmitContext {
        EmitContext::new(576)
    }

    fn compile_markdown(text: &str) -> Vec<Op> {
        let md = Markdown::new(text);
        let mut ctx = ctx();
        md.emit(&mut ctx);
        ctx.ops
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
    }

    #[test]
    fn test_unordered_list() {
        let ops = compile_markdown("- Item 1\n- Item 2");
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
    fn test_empty_markdown() {
        let ops = compile_markdown("");
        assert!(ops.is_empty());
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
        assert!(text_ops.iter().any(|s: &&str| s.contains("\u{2500}")));
    }
}
