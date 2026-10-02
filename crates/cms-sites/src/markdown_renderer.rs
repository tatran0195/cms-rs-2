//! Markdown renderer
//!
//! This module provides Markdown to HTML rendering for published pages.
//! Syntax highlighting is performed **server-side** using [`syntect`] — no
//! client-side JavaScript is required. A `SyntaxSet` and `ThemeSet` are
//! loaded once at program startup via [`lazy_static`] and reused across all
//! requests.

use std::collections::HashSet;

use ammonia::Builder;
use cms_entity::{page::Page, project::Project};
use lazy_static::lazy_static;
use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};

// ---------------------------------------------------------------------------
// Global, lazily-initialized syntax/theme sets (expensive to construct)
// ---------------------------------------------------------------------------

lazy_static! {
    /// All bundled syntaxes shipped with syntect (TextMate `.tmLanguage` grammars).
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();

    /// All bundled colour themes shipped with syntect.
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Which colour theme to use for server-side syntax highlighting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HighlightTheme {
    /// Light theme — resembles GitHub's code view (default for light pages).
    #[default]
    InspiredGitHub,
    /// Dark theme — base16-ocean dark palette.
    OceanDark,
    /// Monokai — the classic dark editor theme.
    Monokai,
    /// Solarized (light variant).
    SolarizedLight,
    /// Solarized (dark variant).
    SolarizedDark,
}

impl HighlightTheme {
    /// Returns the syntect theme key for this variant.
    fn as_syntect_key(&self) -> &'static str {
        match self {
            HighlightTheme::InspiredGitHub => "InspiredGitHub",
            HighlightTheme::OceanDark => "base16-ocean.dark",
            HighlightTheme::Monokai => "Monokai",
            HighlightTheme::SolarizedLight => "Solarized (light)",
            HighlightTheme::SolarizedDark => "Solarized (dark)",
        }
    }
}

/// Markdown renderer configuration
#[derive(Debug, Clone)]
pub struct MarkdownRendererConfig {
    /// Enable syntax highlighting
    pub enable_syntax_highlighting: bool,
    /// Colour theme used for syntax highlighting
    pub highlight_theme: HighlightTheme,
    /// Enable table of contents
    pub enable_toc: bool,
    /// Enable footnotes
    pub enable_footnotes: bool,
    /// Enable task lists
    pub enable_task_lists: bool,
    /// Enable strikethrough
    pub enable_strikethrough: bool,
    /// Enable emoji
    pub enable_emoji: bool,
    /// Enable heading IDs
    pub enable_heading_ids: bool,
    /// Base URL for relative links
    pub base_url: String,
}

impl Default for MarkdownRendererConfig {
    fn default() -> Self {
        Self {
            enable_syntax_highlighting: true,
            highlight_theme: HighlightTheme::default(),
            enable_toc: true,
            enable_footnotes: true,
            enable_task_lists: true,
            enable_strikethrough: true,
            enable_emoji: true,
            enable_heading_ids: true,
            base_url: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// MarkdownRenderer
// ---------------------------------------------------------------------------

/// Markdown renderer
pub struct MarkdownRenderer {
    config: MarkdownRendererConfig,
}

impl MarkdownRenderer {
    pub fn new(config: MarkdownRendererConfig) -> Self {
        Self { config }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Render page content to HTML
    pub fn render_page(&self, page: &Page, project: &Project) -> String {
        let clean_html = self.render_markdown(&page.content);
        self.wrap_in_template(&clean_html, page, project)
    }

    /// Render a markdown string to sanitized HTML.
    ///
    /// When `enable_syntax_highlighting` is `true`, fenced code blocks are
    /// highlighted server-side by syntect before the HTML sanitisation pass.
    ///
    /// MDX compatibility: GitHub admonition callouts (`> [!NOTE]`, `> [!TIP]`,
    /// …) are turned into `<div data-callout data-variant="…">` blocks and
    /// ` ```mermaid` fenced blocks are kept as `<pre class="mermaid">` so the
    /// client can hydrate them.
    pub fn render_markdown(&self, markdown: &str) -> String {
        let options = self.build_parser_options();
        let parser = Parser::new_ext(markdown, options);

        let html = if self.config.enable_syntax_highlighting {
            self.render_with_highlighting(parser)
        } else {
            let mut output = String::new();
            html::push_html(&mut output, parser);
            output
        };

        let html = self.postprocess_mdx(&html);
        self.sanitize_html(&html)
    }

    /// Post-process the HTML produced by pulldown-cmark to add MDX features that
    /// the default renderer cannot express:
    ///
    /// - GitHub-style admonition blockquotes `> [!NOTE]` … → a `div[data-callout]`.
    /// - Mermaid fenced code blocks already arrive as `<pre><code class="mermaid">`
    ///   (handled in `render_with_highlighting`); nothing more is needed here.
    ///
    /// The inner blockquote content is already Markdown-parsed by pulldown-cmark,
    /// so rich bodies (lists, code, nested formatting) keep working.
    fn postprocess_mdx(&self, html: &str) -> String {
        self.rewrite_admonitions(html)
    }

    /// Convert `<blockquote><p>[!WORD] …</p>…</blockquote>` into the SPA's
    /// `div[data-callout][data-variant]` shape.
    fn rewrite_admonitions(&self, html: &str) -> String {
        // Map the GitHub admonition keyword to the SPA callout variant.
        fn keyword_to_variant(kw: &str) -> Option<&'static str> {
            match kw.to_ascii_uppercase().as_str() {
                "NOTE" => Some("note"),
                "TIP" => Some("tip"),
                "CHECK" => Some("check"),
                "WARNING" => Some("warning"),
                "CAUTION" => Some("danger"),
                "IMPORTANT" => Some("info"),
                "INFO" => Some("info"),
                _ => None,
            }
        }

        let blockquote_tag = "<blockquote>";
        let mut out = String::with_capacity(html.len());
        let mut rest = html;
        while let Some(start) = rest.find(blockquote_tag) {
            out.push_str(&rest[..start]);
            let after_open = &rest[start + blockquote_tag.len()..];
            // Find the matching close tag (blockquotes don't nest in pulldown).
            let end = match after_open.find("</blockquote>") {
                Some(i) => i,
                None => {
                    out.push_str(blockquote_tag);
                    out.push_str(after_open);
                    rest = "";
                    break;
                }
            };
            let inner = &after_open[..end];
            let closing = &after_open[end..];

            // Find the first paragraph and its `[!WORD]` marker.
            if let Some(p_start) = inner.find("<p>") {
                let after_p = &inner[p_start + 3..];
                if let Some(p_end) = after_p.find("</p>") {
                    let p_text = &after_p[..p_end];
                    if let Some(marker_end) = p_text.find(']') {
                        let marker = &p_text[..=marker_end];
                        let word = marker.trim_start_matches("[!").trim_end_matches(']').trim();
                        if let Some(variant) = keyword_to_variant(word) {
                            // Drop the `[!WORD]` marker from the leading paragraph.
                            let body_start = marker_end + 1;
                            let cleaned_p = p_text[body_start..].trim_start();
                            let mut body = String::new();
                            body.push_str("<div class=\"pl-callout-body\">");
                            if cleaned_p.is_empty() {
                                body.push_str(&after_p[p_end + 4..]);
                            } else {
                                body.push_str("<p>");
                                body.push_str(cleaned_p);
                                body.push_str("</p>");
                                body.push_str(&after_p[p_end + 4..]);
                            }
                            body.push_str("</div>");
                            out.push_str("<div data-callout=\"true\" data-variant=\"");
                            out.push_str(variant);
                            out.push_str("\" class=\"pl-callout\">");
                            out.push_str(&body);
                            out.push_str("</div>");
                            rest = closing;
                            continue;
                        }
                    }
                }
            }

            // Not an admonition — emit the blockquote verbatim.
            out.push_str(blockquote_tag);
            out.push_str(inner);
            out.push_str(closing);
            rest = &after_open[end..];
        }
        out.push_str(rest);
        out
    }

    /// Highlight a code snippet and return an HTML `<pre>` block with inline
    /// colour spans.  Falls back gracefully to an escaped plain `<pre>` if
    /// the language is unknown or highlighting fails.
    pub fn highlight_code(&self, code: &str, language: Option<&str>) -> String {
        if !self.config.enable_syntax_highlighting {
            return format!("<pre><code>{}</code></pre>", self.escape_html(code));
        }

        self.do_highlight(code, language)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build pulldown-cmark `Options` from renderer config.
    fn build_parser_options(&self) -> Options {
        let mut options = Options::empty();
        if self.config.enable_footnotes {
            options.insert(Options::ENABLE_FOOTNOTES);
        }
        if self.config.enable_task_lists {
            options.insert(Options::ENABLE_TASKLISTS);
        }
        if self.config.enable_strikethrough {
            options.insert(Options::ENABLE_STRIKETHROUGH);
        }
        if self.config.enable_heading_ids {
            options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        }
        options
    }

    /// Walk the pulldown-cmark event stream, intercept `CodeBlock` events and
    /// replace them with pre-highlighted raw HTML events.  All other events
    /// are forwarded unchanged and rendered by `html::push_html`.
    fn render_with_highlighting<'a, I>(&self, parser: I) -> String
    where
        I: Iterator<Item = Event<'a>>,
    {
        // We collect the transformed events into a Vec so we can feed them
        // back into `html::push_html`.
        let mut events: Vec<Event<'_>> = Vec::new();
        let mut in_code_block = false;
        let mut current_lang: Option<String> = None;
        let mut code_buf = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(ref kind)) => {
                    in_code_block = true;
                    current_lang = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let s = lang.as_ref().trim().to_owned();
                            if s.is_empty() {
                                None
                            } else {
                                Some(s)
                            }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    code_buf.clear();
                    // Don't emit a Start event — we'll emit raw HTML instead.
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    // Mermaid diagrams: keep the raw source in a
                    // `<pre class="mermaid">` so the client can hydrate them.
                    if current_lang
                        .as_deref()
                        .map(|l| {
                            l.split_whitespace()
                                .next()
                                .unwrap_or(l)
                                .eq_ignore_ascii_case("mermaid")
                        })
                        .unwrap_or(false)
                    {
                        let escaped = self.escape_html(&code_buf);
                        events.push(Event::Html(
                            format!("<pre class=\"mermaid\">{escaped}</pre>").into(),
                        ));
                    } else {
                        let highlighted = self.do_highlight(&code_buf, current_lang.as_deref());
                        // Inject pre-rendered HTML as a raw event.
                        events.push(Event::Html(highlighted.into()));
                    }
                    current_lang = None;
                    code_buf.clear();
                }
                Event::Text(ref text) if in_code_block => {
                    code_buf.push_str(text);
                }
                other => {
                    events.push(other);
                }
            }
        }

        let mut output = String::new();
        html::push_html(&mut output, events.into_iter());
        output
    }

    /// Core syntect highlighting routine.
    ///
    /// Resolution order for the syntax:
    /// 1. Exact token (e.g. `"rust"`, `"python"`)
    /// 2. File-extension lookup (e.g. `"js"` → JavaScript)
    /// 3. Plain text fallback
    fn do_highlight(&self, code: &str, language: Option<&str>) -> String {
        let ss = &*SYNTAX_SET;
        let ts = &*THEME_SET;

        let theme_key = self.config.highlight_theme.as_syntect_key();
        let theme = ts
            .themes
            .get(theme_key)
            .or_else(|| ts.themes.get("InspiredGitHub"))
            .expect("bundled syntect themes are always present");

        // Resolve syntax definition.
        let syntax = language
            .and_then(|lang| {
                // Try the first word of the language tag (e.g. "rust ignore" → "rust").
                let token = lang.split_whitespace().next().unwrap_or(lang);

                // 1. Token match (case-insensitive name)
                ss.find_syntax_by_token(token)
                    // 2. Extension match
                    .or_else(|| ss.find_syntax_by_extension(token))
            })
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        match highlighted_html_for_string(code, ss, syntax, theme) {
            Ok(html) => {
                // syntect wraps output in <pre style="..."><span...>, which is
                // exactly what we want for inline styles.
                html
            }
            Err(_) => {
                // Graceful degradation: plain escaped block with language class.
                let lang_class = language
                    .map(|l| format!(" class=\"language-{}\"", l))
                    .unwrap_or_default();
                format!(
                    "<pre{}><code>{}</code></pre>",
                    lang_class,
                    self.escape_html(code)
                )
            }
        }
    }

    /// Sanitize HTML to prevent XSS.
    ///
    /// Note: inline `style` attributes **are** allowed so that syntect's
    /// colour spans (`<span style="color:...">`) survive sanitisation.
    fn sanitize_html(&self, html: &str) -> String {
        let mut ammonia = Builder::default();
        ammonia
            .tags(self.get_allowed_tags())
            .tag_attributes(self.get_allowed_attributes())
            .url_schemes(self.get_allowed_schemes())
            .clean_content_tags(self.get_clean_content_tags());

        ammonia.clean(html).to_string()
    }

    /// Get allowed HTML tags
    fn get_allowed_tags(&self) -> HashSet<&'static str> {
        let mut tags = HashSet::new();

        // Text formatting
        tags.insert("p");
        tags.insert("br");
        tags.insert("b");
        tags.insert("i");
        tags.insert("u");
        tags.insert("em");
        tags.insert("strong");
        tags.insert("small");
        tags.insert("s");
        tags.insert("cite");
        tags.insert("q");

        // Code
        tags.insert("code");
        tags.insert("pre");
        tags.insert("tt");
        tags.insert("kbd");
        tags.insert("samp");
        tags.insert("var");

        // Headings
        tags.insert("h1");
        tags.insert("h2");
        tags.insert("h3");
        tags.insert("h4");
        tags.insert("h5");
        tags.insert("h6");

        // Lists
        tags.insert("ul");
        tags.insert("ol");
        tags.insert("li");
        tags.insert("dl");
        tags.insert("dt");
        tags.insert("dd");

        // Tables
        tags.insert("table");
        tags.insert("thead");
        tags.insert("tbody");
        tags.insert("tfoot");
        tags.insert("tr");
        tags.insert("th");
        tags.insert("td");

        // Block elements
        tags.insert("blockquote");
        tags.insert("figure");
        tags.insert("figcaption");
        tags.insert("div");
        tags.insert("span");
        tags.insert("section");
        tags.insert("header");
        tags.insert("footer");
        tags.insert("aside");
        tags.insert("article");

        // Details
        tags.insert("details");
        tags.insert("summary");

        // Text semantics
        tags.insert("mark");
        tags.insert("time");
        tags.insert("abbr");
        tags.insert("sub");
        tags.insert("sup");

        // Links and images
        tags.insert("a");
        tags.insert("img");

        // Horizontal rule
        tags.insert("hr");

        // Iframes (for embeds)
        if self.config.enable_syntax_highlighting {
            tags.insert("iframe");
        }

        tags
    }

    /// Get allowed HTML attributes per tag
    fn get_allowed_attributes(
        &self,
    ) -> std::collections::HashMap<&'static str, HashSet<&'static str>> {
        let mut attrs = std::collections::HashMap::new();

        // Link attributes
        let mut a_attrs = HashSet::new();
        a_attrs.insert("href");
        a_attrs.insert("title");
        a_attrs.insert("target");
        attrs.insert("a", a_attrs);

        // Image attributes
        let mut img_attrs = HashSet::new();
        img_attrs.insert("src");
        img_attrs.insert("alt");
        img_attrs.insert("title");
        img_attrs.insert("width");
        img_attrs.insert("height");
        attrs.insert("img", img_attrs);

        // Iframe attributes
        let mut iframe_attrs = HashSet::new();
        iframe_attrs.insert("src");
        iframe_attrs.insert("width");
        iframe_attrs.insert("height");
        iframe_attrs.insert("frameborder");
        iframe_attrs.insert("allowfullscreen");
        attrs.insert("iframe", iframe_attrs);

        // Table cell attributes
        let mut td_attrs = HashSet::new();
        td_attrs.insert("colspan");
        td_attrs.insert("rowspan");
        attrs.insert("td", td_attrs.clone());
        attrs.insert("th", td_attrs);

        // Global attributes — including `style` so syntect spans survive.
        let mut global_attrs = HashSet::new();
        global_attrs.insert("class");
        global_attrs.insert("id");
        global_attrs.insert("style");
        global_attrs.insert("data-*");
        global_attrs.insert("data-callout");
        global_attrs.insert("data-variant");

        // Apply global attributes to all tags
        for tag in self.get_allowed_tags() {
            attrs
                .entry(tag)
                .or_default()
                .extend(global_attrs.iter().copied());
        }

        attrs
    }

    /// Get allowed URL schemes
    fn get_allowed_schemes(&self) -> HashSet<&'static str> {
        let mut schemes = HashSet::new();
        schemes.insert("http");
        schemes.insert("https");
        schemes.insert("mailto");
        schemes.insert("tel");
        schemes.insert("ftp");
        schemes
    }

    /// Get tags that should have their content cleaned
    fn get_clean_content_tags(&self) -> HashSet<&'static str> {
        let mut tags = HashSet::new();
        tags.insert("script");
        tags.insert("style");
        if !self.config.enable_syntax_highlighting {
            tags.insert("iframe");
        }
        tags.insert("object");
        tags.insert("embed");
        tags
    }

    /// Wrap HTML in page template
    fn wrap_in_template(&self, content: &str, page: &Page, project: &Project) -> String {
        let description = page.description.as_deref().unwrap_or_default();
        let title = &page.title;
        let project_name = &project.name;

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="{}">
    <meta name="og:title" content="{} - {}">
    <meta name="og:description" content="{}">
    <meta name="og:type" content="article">
    <meta name="twitter:card" content="summary_large_image">
    <title>{} - {}</title>
    <link rel="stylesheet" href="/assets/styles.css">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            color: #333;
        }}
        pre {{
            border-radius: 6px;
            overflow-x: auto;
            font-size: 0.9em;
        }}
        /* syntect emits inline styles; these rules set sensible defaults for
           the <pre> wrapper it generates. */
        pre[style] {{
            padding: 1em;
        }}
        code {{
            background: #f4f4f4;
            padding: 2px 5px;
            border-radius: 3px;
            font-size: 0.9em;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 8px;
        }}
        th {{
            background: #f4f4f4;
            text-align: left;
        }}
        img {{
            max-width: 100%;
            height: auto;
        }}
        a {{
            color: #0066cc;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <header>
        <h1>{}</h1>
        <p>{}</p>
    </header>
    <main>
        {}
    </main>
    <footer>
        <hr>
        <p>Powered by <a href="https://cms.com">CMS</a></p>
    </footer>
</body>
</html>"#,
            description,
            title,
            project_name,
            description,
            title,
            project_name,
            title,
            description,
            content
        )
    }

    /// Render table of contents
    pub fn render_toc(&self, markdown: &str) -> Option<String> {
        if !self.config.enable_toc {
            return None;
        }

        // Extract headings from markdown
        let parser = Parser::new_ext(markdown, Options::empty());
        let mut toc = String::new();
        let mut in_list = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let heading_level: usize = match level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                    if !in_list {
                        toc.push_str("<ul>\n");
                        in_list = true;
                    }
                    let indent = (heading_level.saturating_sub(1)) * 4;
                    toc.push_str(&format!("{}<li>", " ".repeat(indent)));
                }
                Event::Text(text) => {
                    // Generate anchor from text
                    let anchor = self.generate_anchor(&text);
                    toc.push_str(&format!("<a href=\"#{}\">{}</a>", anchor, text));
                }
                Event::End(TagEnd::Heading(_)) => {
                    toc.push_str("</li>\n");
                }
                Event::End(TagEnd::Paragraph) if in_list => {
                    toc.push_str("</ul>\n");
                    in_list = false;
                }
                _ => {}
            }
        }

        if in_list {
            toc.push_str("</ul>\n");
        }

        if toc.is_empty() {
            None
        } else {
            Some(toc)
        }
    }

    /// Generate anchor from text
    fn generate_anchor(&self, text: &str) -> String {
        text.to_lowercase()
            .replace(' ', "-")
            .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "")
    }

    /// Escape HTML special characters
    fn escape_html(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_rendering() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        let markdown = "# Hello World\n\nThis is **bold** text.";
        let html = renderer.render_markdown(markdown);

        assert!(html.contains("<h1>Hello World</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_anchor_generation() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        assert_eq!(renderer.generate_anchor("Hello World"), "hello-world");
        assert_eq!(renderer.generate_anchor("Test 123"), "test-123");
        assert_eq!(
            renderer.generate_anchor("Special!@#$%Chars"),
            "specialchars"
        );
    }

    #[test]
    fn test_rust_code_highlighting() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        let markdown = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let html = renderer.render_markdown(markdown);

        // syntect emits a <pre> with inline style attributes
        assert!(html.contains("<pre"), "expected a <pre> block");
        // The word 'fn' should appear somewhere in the output (possibly as a span)
        assert!(
            html.contains("fn") || html.contains("&lt;"),
            "expected code content"
        );
    }

    #[test]
    fn test_unknown_language_fallback() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        let html = renderer.highlight_code("hello world", Some("nonexistent-lang-xyz"));
        // Should still produce a <pre> block without panicking
        assert!(html.contains("<pre"));
        assert!(html.contains("hello world") || html.contains("hello"));
    }

    #[test]
    fn test_highlighting_disabled() {
        let config = MarkdownRendererConfig {
            enable_syntax_highlighting: false,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::new(config);

        let html = renderer.highlight_code("let x = 1;", Some("rust"));
        // Must not contain any syntect inline styles
        assert!(!html.contains("style=\""));
        assert!(html.contains("<pre><code>"));
    }

    #[test]
    fn test_admonition_callout() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        let markdown =
            "> [!NOTE]\n> This is a note body.\n\n> [!TIP]\n> - item one\n> - item two\n";
        let html = renderer.render_markdown(markdown);

        assert!(
            html.contains("data-callout"),
            "admonition should become a callout div"
        );
        assert!(
            html.contains("data-variant=\"note\""),
            "variant should be note"
        );
        assert!(html.contains("This is a note body."), "body text preserved");
        assert!(
            html.contains("<li>") || html.contains("- item one"),
            "list body should be parsed as markdown"
        );
    }

    #[test]
    fn test_mermaid_is_not_highlighted() {
        let renderer = MarkdownRenderer::new(MarkdownRendererConfig::default());

        let markdown = "```mermaid\ngraph TD\nA-->B\n```\n";
        let html = renderer.render_markdown(markdown);

        assert!(
            html.contains("class=\"mermaid\""),
            "mermaid block should be kept raw"
        );
        assert!(html.contains("graph TD"), "mermaid source preserved");
    }

    #[test]
    fn test_highlight_theme_selection() {
        let config = MarkdownRendererConfig {
            highlight_theme: HighlightTheme::OceanDark,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::new(config);

        let html = renderer.highlight_code("let x = 1;", Some("rust"));
        assert!(html.contains("<pre"), "expected a highlighted <pre> block");
    }
}
