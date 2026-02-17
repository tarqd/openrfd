use markdown::mdast::Node;
use markdown::{Constructs, ParseOptions};

use crate::source_map::{RenderedDocument, SourceSpan};

/// Parse options: GFM features + frontmatter.
fn parse_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            frontmatter: true,
            ..Constructs::gfm()
        },
        ..ParseOptions::gfm()
    }
}

/// Render markdown to HTML with source mapping via `data-source` attributes.
pub fn render_markdown(source: &str) -> RenderedDocument {
    let ast = markdown::to_mdast(source, &parse_options()).unwrap_or_else(|_| {
        // Fallback: root with a single text node
        Node::Root(markdown::mdast::Root {
            children: vec![Node::Text(markdown::mdast::Text {
                value: source.to_string(),
                position: None,
            })],
            position: None,
        })
    });

    let mut html = String::with_capacity(source.len() * 2);
    let mut source_map = Vec::new();

    render_node(&ast, &mut html, &mut source_map);

    RenderedDocument { html, source_map }
}

// ---------------------------------------------------------------------------
// AST → HTML renderer
// ---------------------------------------------------------------------------

fn render_node(node: &Node, html: &mut String, source_map: &mut Vec<SourceSpan>) {
    match node {
        Node::Root(root) => {
            for child in &root.children {
                render_node(child, html, source_map);
            }
        }

        // -- Block elements (with data-source) --

        Node::Paragraph(_) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            html.push_str(&format!("<p{ds}>"));
            render_children(node, html, source_map);
            html.push_str("</p>\n");
            push_span(node, out_start, html.len(), source_map);
        }

        Node::Heading(heading) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            html.push_str(&format!("<h{}{ds}>", heading.depth));
            render_children(node, html, source_map);
            html.push_str(&format!("</h{}>\n", heading.depth));
            push_span(node, out_start, html.len(), source_map);
        }

        Node::Blockquote(_) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            html.push_str(&format!("<blockquote{ds}>\n"));
            render_children(node, html, source_map);
            html.push_str("</blockquote>\n");
            push_span(node, out_start, html.len(), source_map);
        }

        Node::Code(code) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            let lang_attr = code
                .lang
                .as_deref()
                .filter(|l| !l.is_empty())
                .map(|l| format!(" class=\"language-{}\"", l))
                .unwrap_or_default();
            html.push_str(&format!("<pre{ds}><code{lang_attr}>"));
            html.push_str(&html_escape(&code.value));
            html.push_str("</code></pre>\n");
            push_span(node, out_start, html.len(), source_map);
        }

        Node::List(list) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            if list.ordered {
                let start = list.start.unwrap_or(1);
                html.push_str(&format!("<ol start=\"{start}\"{ds}>\n"));
            } else {
                html.push_str(&format!("<ul{ds}>\n"));
            }
            render_children(node, html, source_map);
            if list.ordered {
                html.push_str("</ol>\n");
            } else {
                html.push_str("</ul>\n");
            }
            push_span(node, out_start, html.len(), source_map);
        }

        Node::ListItem(item) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            html.push_str(&format!("<li{ds}>"));
            if let Some(checked) = item.checked {
                if checked {
                    html.push_str("<input type=\"checkbox\" checked=\"\" disabled=\"\" /> ");
                } else {
                    html.push_str("<input type=\"checkbox\" disabled=\"\" /> ");
                }
            }
            render_children(node, html, source_map);
            html.push_str("</li>\n");
            push_span(node, out_start, html.len(), source_map);
        }

        Node::Table(_) => {
            let ds = data_source_attr(node);
            let out_start = html.len();
            html.push_str(&format!("<table{ds}>\n"));
            // First child row is the head
            if let Some(children) = node.children() {
                let mut first = true;
                for child in children {
                    if first {
                        html.push_str("<thead>\n");
                        render_table_row(child, html, source_map, true);
                        html.push_str("</thead>\n<tbody>\n");
                        first = false;
                    } else {
                        render_table_row(child, html, source_map, false);
                    }
                }
                if !first {
                    html.push_str("</tbody>\n");
                }
            }
            html.push_str("</table>\n");
            push_span(node, out_start, html.len(), source_map);
        }

        Node::ThematicBreak(_) => {
            html.push_str("<hr />\n");
        }

        Node::FootnoteDefinition(def) => {
            html.push_str(&format!(
                "<div class=\"footnote\" id=\"fn-{}\">\n<p>{}: ",
                html_escape(&def.identifier),
                html_escape(&def.identifier)
            ));
            render_children(node, html, source_map);
            html.push_str("</p>\n</div>\n");
        }

        // -- Inline elements --

        Node::Text(text) => {
            html.push_str(&html_escape(&text.value));
        }

        Node::InlineCode(code) => {
            html.push_str("<code>");
            html.push_str(&html_escape(&code.value));
            html.push_str("</code>");
        }

        Node::Emphasis(_) => {
            html.push_str("<em>");
            render_children(node, html, source_map);
            html.push_str("</em>");
        }

        Node::Strong(_) => {
            html.push_str("<strong>");
            render_children(node, html, source_map);
            html.push_str("</strong>");
        }

        Node::Delete(_) => {
            html.push_str("<del>");
            render_children(node, html, source_map);
            html.push_str("</del>");
        }

        Node::Link(link) => {
            html.push_str(&format!("<a href=\"{}\"", html_escape(&link.url)));
            if let Some(title) = &link.title {
                if !title.is_empty() {
                    html.push_str(&format!(" title=\"{}\"", html_escape(title)));
                }
            }
            html.push('>');
            render_children(node, html, source_map);
            html.push_str("</a>");
        }

        Node::Image(img) => {
            html.push_str(&format!(
                "<img src=\"{}\" alt=\"{}\"",
                html_escape(&img.url),
                html_escape(&img.alt)
            ));
            if let Some(title) = &img.title {
                if !title.is_empty() {
                    html.push_str(&format!(" title=\"{}\"", html_escape(title)));
                }
            }
            html.push_str(" />");
        }

        Node::Break(_) => {
            html.push_str("<br />\n");
        }

        Node::Html(raw) => {
            html.push_str(&raw.value);
        }

        Node::FootnoteReference(r) => {
            html.push_str(&format!(
                "<sup class=\"footnote-ref\"><a href=\"#fn-{id}\">[{id}]</a></sup>",
                id = html_escape(&r.identifier)
            ));
        }

        // Skip frontmatter, definitions, MDX, math, etc.
        Node::Yaml(_)
        | Node::Toml(_)
        | Node::Definition(_)
        | Node::ImageReference(_)
        | Node::LinkReference(_) => {}

        // Any other node: try rendering children
        _ => {
            render_children(node, html, source_map);
        }
    }
}

fn render_table_row(node: &Node, html: &mut String, source_map: &mut Vec<SourceSpan>, is_head: bool) {
    html.push_str("<tr>");
    if let Some(children) = node.children() {
        let tag = if is_head { "th" } else { "td" };
        for cell in children {
            html.push_str(&format!("<{tag}>"));
            render_children(cell, html, source_map);
            html.push_str(&format!("</{tag}>"));
        }
    }
    html.push_str("</tr>\n");
}

fn render_children(node: &Node, html: &mut String, source_map: &mut Vec<SourceSpan>) {
    if let Some(children) = node.children() {
        for child in children {
            render_node(child, html, source_map);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `data-source="start-end"` attribute string for a node.
fn data_source_attr(node: &Node) -> String {
    node.position()
        .map(|pos| {
            format!(
                " data-source=\"{}-{}\"",
                pos.start.offset, pos.end.offset
            )
        })
        .unwrap_or_default()
}

/// Record a source span if the node has position info.
fn push_span(
    node: &Node,
    out_start: usize,
    out_end: usize,
    source_map: &mut Vec<SourceSpan>,
) {
    if let Some(pos) = node.position() {
        source_map.push(SourceSpan {
            source_range: pos.start.offset..pos.end.offset,
            output_range: out_start..out_end,
            line_range: (pos.start.line as u32)..(pos.end.line as u32),
        });
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
