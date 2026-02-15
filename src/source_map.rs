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
