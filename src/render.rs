use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::source_map::{line_number_at, RenderedDocument, SourceSpan};

/// Render markdown to HTML with source mapping via `data-source` attributes.
pub fn render_markdown(source: &str) -> RenderedDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, options);

    let mut html = String::with_capacity(source.len() * 2);
    let mut source_map = Vec::new();

    // Track current block-level element for source mapping
    let mut block_start_source: Option<usize> = None;
    let mut block_start_output: Option<usize> = None;
    let mut _in_block = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(ref tag) => {
                if is_block_tag(tag) {
                    block_start_source = Some(range.start);
                    block_start_output = Some(html.len());
                    _in_block = true;

                    let line_start = line_number_at(source, range.start);
                    write_open_tag(&mut html, tag, Some(&format!("{}-{}", range.start, range.end)), line_start);
                } else {
                    write_open_tag(&mut html, tag, None, 0);
                }
            }
            Event::End(ref tag) => {
                if is_block_end_tag(tag) {
                    write_close_tag(&mut html, tag);

                    // Record source mapping
                    if let (Some(src_start), Some(out_start)) =
                        (block_start_source, block_start_output)
                    {
                        let src_end = range.end;
                        let out_end = html.len();
                        source_map.push(SourceSpan {
                            source_range: src_start..src_end,
                            output_range: out_start..out_end,
                            line_range: line_number_at(source, src_start)
                                ..line_number_at(source, src_end),
                        });
                    }
                    block_start_source = None;
                    block_start_output = None;
                    _in_block = false;
                } else {
                    write_close_tag(&mut html, tag);
                }
            }
            Event::Text(text) => {
                html.push_str(&html_escape(&text));
            }
            Event::Code(code) => {
                html.push_str("<code>");
                html.push_str(&html_escape(&code));
                html.push_str("</code>");
            }
            Event::SoftBreak => {
                html.push('\n');
            }
            Event::HardBreak => {
                html.push_str("<br />\n");
            }
            Event::Rule => {
                html.push_str("<hr />\n");
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                html.push_str(&raw);
            }
            Event::FootnoteReference(name) => {
                html.push_str(&format!(
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{name}\">[{name}]</a></sup>"
                ));
            }
            Event::TaskListMarker(checked) => {
                if checked {
                    html.push_str("<input type=\"checkbox\" checked=\"\" disabled=\"\" /> ");
                } else {
                    html.push_str("<input type=\"checkbox\" disabled=\"\" /> ");
                }
            }
            _ => {}
        }
    }

    RenderedDocument { html, source_map }
}

fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
    )
}

fn is_block_end_tag(tag: &TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
    )
}

fn write_open_tag(html: &mut String, tag: &Tag, data_source: Option<&str>, _line: u32) {
    let ds = data_source
        .map(|s| format!(" data-source=\"{}\"", s))
        .unwrap_or_default();
    match tag {
        Tag::Paragraph => html.push_str(&format!("<p{ds}>")),
        Tag::Heading { level, .. } => {
            html.push_str(&format!("<{}{ds}>", level))
        }
        Tag::BlockQuote(_) => html.push_str(&format!("<blockquote{ds}>")),
        Tag::CodeBlock(kind) => {
            let lang = match kind {
                pulldown_cmark::CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                    format!(" class=\"language-{}\"", lang)
                }
                _ => String::new(),
            };
            html.push_str(&format!("<pre{ds}><code{lang}>"));
        }
        Tag::List(Some(start)) => html.push_str(&format!("<ol start=\"{start}\"{ds}>")),
        Tag::List(None) => html.push_str(&format!("<ul{ds}>")),
        Tag::Item => html.push_str(&format!("<li{ds}>")),
        Tag::Table(_) => html.push_str(&format!("<table{ds}>")),
        Tag::TableHead => html.push_str("<thead><tr>"),
        Tag::TableRow => html.push_str("<tr>"),
        Tag::TableCell => html.push_str("<td>"),
        Tag::Emphasis => html.push_str("<em>"),
        Tag::Strong => html.push_str("<strong>"),
        Tag::Strikethrough => html.push_str("<del>"),
        Tag::Link { dest_url, title, .. } => {
            html.push_str(&format!("<a href=\"{}\"", html_escape(dest_url)));
            if !title.is_empty() {
                html.push_str(&format!(" title=\"{}\"", html_escape(title)));
            }
            html.push('>');
        }
        Tag::Image { dest_url, title, .. } => {
            html.push_str(&format!("<img src=\"{}\"", html_escape(dest_url)));
            if !title.is_empty() {
                html.push_str(&format!(" title=\"{}\"", html_escape(title)));
            }
            html.push_str(" alt=\"");
        }
        _ => {}
    }
}

fn write_close_tag(html: &mut String, tag: &TagEnd) {
    match tag {
        TagEnd::Paragraph => html.push_str("</p>\n"),
        TagEnd::Heading(level) => html.push_str(&format!("</{}>\n", level)),
        TagEnd::BlockQuote(_) => html.push_str("</blockquote>\n"),
        TagEnd::CodeBlock => html.push_str("</code></pre>\n"),
        TagEnd::List(true) => html.push_str("</ol>\n"),
        TagEnd::List(false) => html.push_str("</ul>\n"),
        TagEnd::Item => html.push_str("</li>\n"),
        TagEnd::Table => html.push_str("</table>\n"),
        TagEnd::TableHead => html.push_str("</thead>\n"),
        TagEnd::TableRow => html.push_str("</tr>\n"),
        TagEnd::TableCell => html.push_str("</td>"),
        TagEnd::Emphasis => html.push_str("</em>"),
        TagEnd::Strong => html.push_str("</strong>"),
        TagEnd::Strikethrough => html.push_str("</del>"),
        TagEnd::Link => html.push_str("</a>"),
        TagEnd::Image => html.push_str("\" />"),
        _ => {}
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_render() {
        let doc = render_markdown("# Hello\n\nA paragraph.\n");
        assert!(doc.html.contains("data-source="));
        assert!(doc.html.contains("<h1"));
        assert!(doc.html.contains("Hello"));
        assert!(doc.html.contains("<p"));
        assert!(doc.html.contains("A paragraph."));
    }

    #[test]
    fn test_source_map_populated() {
        let doc = render_markdown("# Title\n\nParagraph one.\n\nParagraph two.\n");
        assert!(!doc.source_map.is_empty());
    }
}
