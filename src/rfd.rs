use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use miette::NamedSource;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{InvalidFrontmatter, MissingFrontmatter};
use crate::state::{State, Visibility};

/// YAML frontmatter for an RFD document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub authors: Option<String>,
    pub state: State,
    #[serde(default)]
    pub visibility: Option<Visibility>,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// A parsed RFD document.
#[derive(Debug, Clone)]
pub struct Rfd {
    pub number: u32,
    pub frontmatter: Frontmatter,
    /// Raw markdown content after frontmatter.
    pub body: String,
    /// Path to the RFD file.
    pub path: PathBuf,
}

impl Rfd {
    /// Parse an RFD from a file path, inferring the number from the directory name.
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

        // Infer number from parent directory name (e.g., rfd/0042/README.md -> 42)
        let number = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0);

        Self::parse(number, &content, path.to_path_buf())
    }

    /// Parse an RFD from raw content.
    pub fn parse(number: u32, content: &str, path: PathBuf) -> Result<Self> {
        let filename = path.display().to_string();
        let (frontmatter, body) = parse_frontmatter(content, &filename)?;

        Ok(Rfd {
            number,
            frontmatter,
            body,
            path,
        })
    }

    /// Extract the title from the first markdown heading.
    pub fn title(&self) -> String {
        for line in self.body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                // Strip leading # and optional RFD number prefix
                let title = trimmed.trim_start_matches('#').trim();
                // Remove "RFD NNNN " prefix if present
                if let Some(rest) = title.strip_prefix("RFD ") {
                    if let Some(pos) = rest.find(|c: char| !c.is_ascii_digit()) {
                        return rest[pos..].trim().to_string();
                    }
                }
                return title.to_string();
            }
        }
        String::new()
    }

    /// Get the full markdown content (frontmatter + body).
    pub fn to_content(&self) -> String {
        let yaml = serde_yaml::to_string(&self.frontmatter).unwrap_or_default();
        format!("---\n{}---\n{}", yaml, self.body)
    }

    /// Write the RFD back to disk.
    pub fn save(&self) -> Result<()> {
        let content = self.to_content();
        std::fs::write(&self.path, &content)
            .with_context(|| format!("writing {}", self.path.display()))?;
        Ok(())
    }

    /// Update a single frontmatter field in the file on disk without
    /// re-serializing the entire frontmatter (preserves formatting).
    pub fn set_field(path: &Path, field: &str, value: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;

        let updated = set_frontmatter_field(&content, field, value)?;
        std::fs::write(path, &updated)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Read a single frontmatter field from a file.
    pub fn get_field(path: &Path, field: &str) -> Result<Option<String>> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        Ok(get_frontmatter_field(&content, field))
    }
}

/// Parse YAML frontmatter from markdown content using markdown-rs.
/// Returns (frontmatter, body_after_frontmatter).
fn parse_frontmatter(content: &str, filename: &str) -> Result<(Frontmatter, String)> {
    let content = content.trim_start_matches('\u{feff}'); // strip BOM

    let options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            frontmatter: true,
            ..markdown::Constructs::default()
        },
        ..markdown::ParseOptions::default()
    };

    let ast = markdown::to_mdast(content, &options)
        .map_err(|msg| anyhow::anyhow!("markdown parse error: {}", msg))?;

    // Extract the Yaml node from root children
    let root = match &ast {
        markdown::mdast::Node::Root(root) => root,
        _ => anyhow::bail!("unexpected AST root node"),
    };

    let yaml_node = root.children.iter().find_map(|node| {
        if let markdown::mdast::Node::Yaml(yaml) = node {
            Some(yaml)
        } else {
            None
        }
    });

    let yaml = yaml_node.ok_or_else(|| {
        let span_len = content.len().min(40);
        MissingFrontmatter {
            src: NamedSource::new(filename, content.to_string()),
            span: (0, span_len).into(),
        }
    })?;

    // Byte offset where the YAML content starts inside the document
    // (after the opening "---\n").
    let yaml_content_offset = yaml
        .position
        .as_ref()
        .map(|p| p.start.offset)
        .unwrap_or(0)
        + 4; // skip `---\n`

    let frontmatter: Frontmatter = serde_yaml::from_str(&yaml.value).map_err(|e| {
        let (span_offset, span_len) = if let Some(loc) = e.location() {
            // serde_yaml location index is relative to the yaml string
            let abs = yaml_content_offset + loc.index();
            (abs, 1)
        } else {
            // Fall back to highlighting the whole yaml block
            (yaml_content_offset, yaml.value.len())
        };

        InvalidFrontmatter {
            src: NamedSource::new(filename, content.to_string()),
            span: (span_offset, span_len).into(),
            reason: e.to_string(),
            advice: Some(
                "required fields: `state` (prediscussion, ideation, discussion, \
                 published, committed, abandoned)"
                    .into(),
            ),
        }
    })?;

    // Body is everything after the frontmatter block
    let body = if let Some(pos) = &yaml.position {
        let after = &content[pos.end.offset..];
        if let Some(stripped) = after.strip_prefix('\n') {
            stripped.to_string()
        } else {
            after.to_string()
        }
    } else {
        String::new()
    };

    Ok((frontmatter, body))
}

/// Get a field value from frontmatter without full parsing.
fn get_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let content = content.trim_start_matches('\u{feff}');
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end = after_first.find("\n---")?;
    let yaml = &after_first[..end];

    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{}:", field)) {
            let val = rest.trim();
            if val.is_empty() {
                return None;
            }
            return Some(val.to_string());
        }
    }
    None
}

/// Set a field value in frontmatter, preserving formatting.
fn set_frontmatter_field(content: &str, field: &str, value: &str) -> Result<String> {
    let content = content.trim_start_matches('\u{feff}');
    if !content.starts_with("---") {
        anyhow::bail!("missing frontmatter");
    }

    let after_first = &content[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("unterminated frontmatter"))?;

    let yaml = &after_first[..end];
    let rest = &content[3 + end..];

    let prefix = format!("{}:", field);
    let mut found = false;
    let mut new_yaml = String::new();

    for line in yaml.lines() {
        if line.trim().starts_with(&prefix) {
            new_yaml.push_str(&format!("{}: {}", field, value));
            found = true;
        } else {
            new_yaml.push_str(line);
        }
        new_yaml.push('\n');
    }

    if !found {
        // Insert before the closing ---
        new_yaml.push_str(&format!("{}: {}\n", field, value));
    }

    Ok(format!("---{}{}", new_yaml.trim_end_matches('\n'), rest))
}

/// Find the RFD README file in a directory.
pub fn find_rfd_file(dir: &Path) -> Option<PathBuf> {
    let md = dir.join("README.md");
    if md.exists() {
        return Some(md);
    }
    None
}

/// Scan `rfd/` directories on main to find the next available number.
pub fn next_rfd_number(repo_root: &Path) -> u32 {
    let rfd_dir = repo_root.join("rfd");
    let mut max = 0u32;

    if let Ok(entries) = std::fs::read_dir(&rfd_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(num) = name.parse::<u32>() {
                        max = max.max(num);
                    }
                }
            }
        }
    }

    max + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nauthors: Jane\nstate: discussion\n---\n\n# RFD 0001 Test\n";
        let (fm, body) = parse_frontmatter(content, "test.md").unwrap();
        assert_eq!(fm.state, State::Discussion);
        assert_eq!(fm.authors.as_deref(), Some("Jane"));
        assert!(body.contains("# RFD 0001 Test"));
    }

    #[test]
    fn test_get_field() {
        let content = "---\nauthors: Jane\nstate: discussion\n---\nbody";
        assert_eq!(
            get_frontmatter_field(content, "state"),
            Some("discussion".into())
        );
        assert_eq!(
            get_frontmatter_field(content, "authors"),
            Some("Jane".into())
        );
        assert_eq!(get_frontmatter_field(content, "missing"), None);
    }

    #[test]
    fn test_set_field() {
        let content = "---\nauthors: Jane\nstate: prediscussion\n---\nbody";
        let updated = set_frontmatter_field(content, "state", "discussion").unwrap();
        assert!(updated.contains("state: discussion"));
        assert!(!updated.contains("prediscussion"));
    }

    #[test]
    fn test_title_extraction() {
        let rfd = Rfd {
            number: 42,
            frontmatter: Frontmatter {
                authors: None,
                state: State::Prediscussion,
                visibility: None,
                labels: vec![],
            },
            body: "\n# RFD 0042 Service Mesh Design\n\nContent here.\n".into(),
            path: PathBuf::from("rfd/0042/README.md"),
        };
        assert_eq!(rfd.title(), "Service Mesh Design");
    }
}
