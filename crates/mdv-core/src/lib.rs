//! Markdown to safe HTML, shared by graphical front ends.

use std::collections::HashSet;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::{
    highlighting::ThemeSet,
    html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const THEME: &str = "base16-ocean.dark";
const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

/// Renders GitHub-flavored Markdown to sanitized HTML.
///
/// Documents are untrusted input, so the output never carries scripts,
/// event handlers or `javascript:` links, even if the source embeds them.
pub fn to_html(markdown: &str) -> String {
    let syntaxes = SyntaxSet::load_defaults_newlines();
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    let mut events = Vec::new();
    let mut code: Option<(String, String)> = None;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::Text(text) if code.is_some() => {
                if let Some((_, source)) = code.as_mut() {
                    source.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, source)) = code.take() {
                    events.push(Event::Html(highlight(&source, &lang, &syntaxes).into()));
                }
            }
            other => events.push(other),
        }
    }

    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, events.into_iter());
    sanitize(&unsafe_html)
}

/// Returns the CSS for the classes emitted on highlighted code.
pub fn syntax_css() -> String {
    let themes = ThemeSet::load_defaults();
    css_for_theme_with_class_style(&themes.themes[THEME], CLASS_STYLE)
        .expect("bundled theme converts to CSS")
}

fn highlight(source: &str, lang: &str, syntaxes: &SyntaxSet) -> String {
    let syntax = syntaxes
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, syntaxes, CLASS_STYLE);
    for line in LinesWithEndings::from(source) {
        // On a highlighting failure the block still renders, just with fewer colors.
        let _ = generator.parse_html_for_line_which_includes_newline(line);
    }
    format!("<pre><code>{}</code></pre>\n", generator.finalize())
}

fn sanitize(html: &str) -> String {
    ammonia::Builder::default()
        .add_tags(["input"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        .add_tag_attributes("span", ["class"])
        .add_tag_attributes("th", ["style"])
        .add_tag_attributes("td", ["style"])
        .filter_style_properties(HashSet::from(["text-align"]))
        .clean(html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_github_flavored_elements() {
        let html = to_html("# T\n\n|a|\n|:-:|\n|b|\n\n~~x~~\n\n- [x] done\n");
        assert!(html.contains("<h1>T</h1>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("text-align"));
        assert!(html.contains("<del>x</del>"));
        assert!(html.contains("checkbox"));
    }

    #[test]
    fn highlights_code_and_escapes_its_content() {
        let html = to_html("```rust\nfn main() { \"<script>\"; }\n```\n");
        assert!(html.contains("hl-"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn removes_active_content() {
        let html = to_html(
            "<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n[x](javascript:alert(1))\n",
        );
        assert!(!html.contains("<script"));
        assert!(!html.contains("onerror"));
        assert!(!html.contains("javascript:"));
    }
}
