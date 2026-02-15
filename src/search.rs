use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::config::Config;
use crate::rfd::{find_rfd_file, Rfd};

/// A search result entry.
#[derive(Debug, Serialize)]
pub struct SearchEntry {
    pub number: String,
    pub title: String,
    pub state: String,
    pub authors: String,
    /// Matching line with context.
    pub context: String,
    pub line: u32,
}

/// Search across all RFDs for a query string.
pub fn search(repo_root: &Path, config: &Config, query: &str) -> Result<Vec<SearchEntry>> {
    let rfd_dir = config.rfd_dir(repo_root);
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&rfd_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();

        if name_str.parse::<u32>().is_err() {
            continue;
        }

        let dir = entry.path();
        let file = match find_rfd_file(&dir) {
            Some(f) => f,
            None => continue,
        };

        let rfd = match Rfd::load(&file) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Search through lines
        for (i, line) in rfd.body.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                results.push(SearchEntry {
                    number: name_str.clone(),
                    title: rfd.title(),
                    state: rfd.frontmatter.state.to_string(),
                    authors: rfd.frontmatter.authors.clone().unwrap_or_default(),
                    context: line.trim().to_string(),
                    line: (i + 1) as u32,
                });
            }
        }
    }

    Ok(results)
}

/// Build a JSON search index for the static site's client-side search.
#[derive(Debug, Serialize)]
pub struct SearchIndexEntry {
    pub id: u32,
    pub number: String,
    pub title: String,
    pub state: String,
    pub authors: String,
    pub body: String,
    pub labels: Vec<String>,
}

pub fn build_search_index(repo_root: &Path, config: &Config) -> Result<Vec<SearchIndexEntry>> {
    let rfd_dir = config.rfd_dir(repo_root);
    let mut index = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&rfd_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();

        let num = match name_str.parse::<u32>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let dir = entry.path();
        let file = match find_rfd_file(&dir) {
            Some(f) => f,
            None => continue,
        };

        let rfd = match Rfd::load(&file) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Strip markdown syntax for plain-text search body
        let body = strip_markdown(&rfd.body);

        index.push(SearchIndexEntry {
            id: num,
            number: name_str,
            title: rfd.title(),
            state: rfd.frontmatter.state.to_string(),
            authors: rfd.frontmatter.authors.clone().unwrap_or_default(),
            body,
            labels: rfd.frontmatter.labels.clone(),
        });
    }

    Ok(index)
}

/// Rough markdown stripping for search indexing.
fn strip_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.lines() {
        let trimmed = line.trim();
        // Skip heading markers
        if trimmed.starts_with('#') {
            let text = trimmed.trim_start_matches('#').trim();
            out.push_str(text);
        } else if trimmed.starts_with("```") {
            continue;
        } else if trimmed.starts_with("---") {
            continue;
        } else {
            out.push_str(trimmed);
        }
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_headings() {
        let md = "# Title\n## Subtitle\nParagraph text.";
        let stripped = strip_markdown(md);
        assert!(stripped.contains("Title"));
        assert!(stripped.contains("Subtitle"));
        assert!(stripped.contains("Paragraph text."));
        assert!(!stripped.contains('#'));
    }

    #[test]
    fn test_strip_markdown_code_fences() {
        let md = "Before\n```rust\nlet x = 1;\n```\nAfter";
        let stripped = strip_markdown(md);
        assert!(stripped.contains("Before"));
        assert!(stripped.contains("After"));
        // Code fence delimiters are skipped
        assert!(!stripped.contains("```"));
        // Content between fences is preserved (simple line-by-line stripping)
        assert!(stripped.contains("let x = 1;"));
    }

    #[test]
    fn test_strip_markdown_frontmatter_delimiters() {
        let md = "---\nauthors: Alice\nstate: published\n---\n\nContent";
        let stripped = strip_markdown(md);
        assert!(stripped.contains("Content"));
        assert!(stripped.contains("authors: Alice"));
        // --- delimiters are skipped
        assert!(!stripped.contains("---"));
    }

    #[test]
    fn test_strip_markdown_empty() {
        assert_eq!(strip_markdown("").trim(), "");
    }
}
