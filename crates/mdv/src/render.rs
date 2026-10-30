//! Turns Markdown into terminal lines: prose goes through termimad,
//! top-level fenced code blocks go through syntect for syntax highlighting.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};
use termimad::{FmtText, MadSkin};

const THEME: &str = "base16-ocean.dark";
const CODE_INDENT: &str = "  ";
const RESET: &str = "\x1b[0m";

pub struct Renderer {
    skin: MadSkin,
    syntaxes: SyntaxSet,
    theme: Theme,
}

/// A fenced code block that starts at column 0 (not nested in a list or quote).
struct CodeBlock {
    range: Range<usize>,
    lang: String,
    code: String,
}

impl Renderer {
    pub fn new(skin: MadSkin) -> Self {
        let mut themes = ThemeSet::load_defaults();
        Self {
            skin,
            syntaxes: SyntaxSet::load_defaults_newlines(),
            theme: themes.themes.remove(THEME).expect("bundled theme exists"),
        }
    }

    /// Renders `markdown` into lines that fit `width` columns (code lines are not wrapped).
    pub fn render(&self, markdown: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut cursor = 0;
        for block in top_level_code_blocks(markdown) {
            self.push_prose(&markdown[cursor..block.range.start], width, &mut lines);
            self.push_code(&block, &mut lines);
            cursor = block.range.end;
        }
        self.push_prose(&markdown[cursor..], width, &mut lines);
        lines
    }

    fn push_prose(&self, prose: &str, width: usize, lines: &mut Vec<String>) {
        if prose.trim().is_empty() {
            return;
        }
        let text = FmtText::from(&self.skin, prose, Some(width)).to_string();
        lines.extend(text.lines().map(str::to_owned));
    }

    fn push_code(&self, block: &CodeBlock, lines: &mut Vec<String>) {
        let syntax = self
            .syntaxes
            .find_syntax_by_token(&block.lang)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        for line in LinesWithEndings::from(&block.code) {
            let line = match highlighter.highlight_line(line, &self.syntaxes) {
                Ok(ranges) => as_24_bit_terminal_escaped(&ranges, false),
                Err(_) => line.to_owned(),
            };
            lines.push(format!(
                "{CODE_INDENT}{}{RESET}",
                line.trim_end_matches('\n')
            ));
        }
        lines.push(String::new());
    }
}

fn top_level_code_blocks(markdown: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<CodeBlock> = None;
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                if starts_at_line_start(markdown, range.start) =>
            {
                let lang = info.split_whitespace().next().unwrap_or("").to_owned();
                current = Some(CodeBlock {
                    range,
                    lang,
                    code: String::new(),
                });
            }
            Event::Text(text) => {
                if let Some(block) = current.as_mut() {
                    block.code.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = current.take() {
                    blocks.push(block);
                }
            }
            _ => {}
        }
    }
    blocks
}

/// Blocks inside lists or quotes are indented or prefixed, so slicing them out
/// would break the surrounding container; those stay with termimad.
fn starts_at_line_start(markdown: &str, offset: usize) -> bool {
    offset == 0 || markdown[..offset].ends_with('\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(md: &str) -> Vec<String> {
        Renderer::new(MadSkin::no_style()).render(md, 80)
    }

    #[test]
    fn highlights_fenced_code_with_truecolor_escapes() {
        let lines = render("# Title\n\n```rust\nfn main() {}\n```\n\nAfter\n");
        let code = lines.iter().find(|l| l.contains("main")).unwrap();
        assert!(code.contains("\x1b[38;2;"), "no 24-bit color in {code:?}");
        assert!(code.ends_with(RESET));
        assert!(lines.iter().any(|l| l.contains("Title")));
        assert!(lines.iter().any(|l| l.contains("After")));
    }

    #[test]
    fn unknown_language_falls_back_to_plain_text() {
        let lines = render("```nope\nhello\n```\n");
        assert!(lines.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn code_inside_lists_stays_with_termimad() {
        let lines = render("- item\n\n  ```rust\n  let x = 1;\n  ```\n");
        assert!(lines.iter().all(|l| !l.contains("\x1b[38;2;")));
        assert!(lines.iter().any(|l| l.contains("let x = 1;")));
    }
}
