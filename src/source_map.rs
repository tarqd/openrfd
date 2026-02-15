use std::ops::Range;

/// A mapping from source markdown byte ranges to output HTML byte ranges.
#[derive(Debug, Clone)]
pub struct SourceSpan {
    /// Byte range in the original markdown source.
    pub source_range: Range<usize>,
    /// Byte range in the rendered HTML output.
    pub output_range: Range<usize>,
    /// Source line numbers (1-based, inclusive).
    pub line_range: Range<u32>,
}

/// Rendered HTML document with source mapping information.
#[derive(Debug, Clone)]
pub struct RenderedDocument {
    /// The rendered HTML content.
    pub html: String,
    /// Source spans mapping HTML regions back to markdown.
    pub source_map: Vec<SourceSpan>,
}

impl RenderedDocument {
    /// Find the source span that contains the given HTML byte offset.
    pub fn source_for_output(&self, html_offset: usize) -> Option<&SourceSpan> {
        self.source_map
            .iter()
            .find(|span| span.output_range.contains(&html_offset))
    }

    /// Find the source span that contains the given markdown byte offset.
    pub fn output_for_source(&self, source_offset: usize) -> Option<&SourceSpan> {
        self.source_map
            .iter()
            .find(|span| span.source_range.contains(&source_offset))
    }
}

/// Compute 1-based line number for a byte offset in text.
pub fn line_number_at(text: &str, byte_offset: usize) -> u32 {
    let offset = byte_offset.min(text.len());
    text[..offset].matches('\n').count() as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_number_at_first_line() {
        assert_eq!(line_number_at("hello\nworld", 0), 1);
        assert_eq!(line_number_at("hello\nworld", 3), 1);
    }

    #[test]
    fn test_line_number_at_second_line() {
        assert_eq!(line_number_at("hello\nworld", 6), 2);
        assert_eq!(line_number_at("hello\nworld", 10), 2);
    }

    #[test]
    fn test_line_number_at_newline_boundary() {
        // Offset at the newline itself counts it
        assert_eq!(line_number_at("hello\nworld", 5), 1);
        // One past the newline
        assert_eq!(line_number_at("hello\nworld\nfoo", 11), 2);
        assert_eq!(line_number_at("hello\nworld\nfoo", 12), 3);
    }

    #[test]
    fn test_line_number_at_beyond_end() {
        // Clamps to text length
        assert_eq!(line_number_at("hello\nworld", 999), 2);
    }

    #[test]
    fn test_line_number_at_empty_string() {
        assert_eq!(line_number_at("", 0), 1);
    }

    #[test]
    fn test_rendered_document_lookup() {
        let doc = RenderedDocument {
            html: "<p>hello</p><p>world</p>".into(),
            source_map: vec![
                SourceSpan {
                    source_range: 0..5,
                    output_range: 0..12,
                    line_range: 1..2,
                },
                SourceSpan {
                    source_range: 6..11,
                    output_range: 12..24,
                    line_range: 2..3,
                },
            ],
        };

        // Find source span by HTML offset
        let span = doc.source_for_output(5).unwrap();
        assert_eq!(span.source_range, 0..5);

        let span = doc.source_for_output(15).unwrap();
        assert_eq!(span.source_range, 6..11);

        assert!(doc.source_for_output(99).is_none());

        // Find output span by source offset
        let span = doc.output_for_source(3).unwrap();
        assert_eq!(span.output_range, 0..12);

        let span = doc.output_for_source(8).unwrap();
        assert_eq!(span.output_range, 12..24);

        assert!(doc.output_for_source(99).is_none());
    }
}
