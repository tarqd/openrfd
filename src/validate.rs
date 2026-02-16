use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use miette::NamedSource;

use crate::annotation::{AnchorHealth, AnnotationCollection};
use crate::config::Config;
use crate::diagnostic::ValidationDiagnostic;
use crate::rfd::{find_rfd_file, Rfd};
use crate::selector::check_selectors;

/// Validation result for a single RFD.
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Rich diagnostics with source context for pretty rendering.
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate an RFD document.
pub fn validate_rfd(path: &Path) -> ValidationResult {
    let mut result = ValidationResult::default();
    let filename = path.display().to_string();

    // Check the file exists and is readable
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(format!("cannot read file: {}", e));
            return result;
        }
    };

    // Check frontmatter exists
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        result.errors.push("missing YAML frontmatter (file must start with ---)".into());
        result.diagnostics.push(ValidationDiagnostic {
            message: "missing YAML frontmatter".into(),
            src: NamedSource::new(&filename, content.clone()),
            span: (0, content.len().min(40)).into(),
            label: "expected `---` here".into(),
            advice: Some(
                "add frontmatter at the top of the file:\n\n  ---\n  authors: Your Name\n  state: prediscussion\n  ---".into(),
            ),
        });
        return result;
    }

    // Parse the RFD
    let rfd = match Rfd::parse(0, &content, path.to_path_buf()) {
        Ok(r) => r,
        Err(e) => {
            result.errors.push(format!("invalid frontmatter: {}", e));
            // The parse error itself is already a diagnostic; push a summary
            // so the caller can also render the original diagnostic via the chain.
            return result;
        }
    };

    // Find frontmatter span for pointing at specific fields
    let fm_end = trimmed
        .find("\n---")
        .map(|i| i + 4) // include "\n---"
        .unwrap_or(0);
    let fm_content = &trimmed[..fm_end];

    // Check required fields
    match &rfd.frontmatter.authors {
        Some(authors) if !authors.trim().is_empty() => {}
        _ => {
            result.warnings.push("missing 'authors' field".into());
            result.diagnostics.push(ValidationDiagnostic {
                message: "missing 'authors' field".into(),
                src: NamedSource::new(&filename, content.clone()),
                span: (0, fm_end).into(),
                label: "no `authors` field in frontmatter".into(),
                advice: Some("add `authors: Your Name` to the frontmatter".into()),
            });
        }
    }

    // Check title
    if rfd.title().is_empty() {
        result.warnings.push("no title heading found".into());
        let body_offset = fm_content.len();
        let body_len = content.len().saturating_sub(body_offset).min(80);
        result.diagnostics.push(ValidationDiagnostic {
            message: "no title heading found".into(),
            src: NamedSource::new(&filename, content.clone()),
            span: (body_offset, body_len.max(1)).into(),
            label: "expected a markdown heading (e.g. `# RFD 0001 My Title`)".into(),
            advice: Some(
                "add a level-1 heading after the frontmatter:\n\n  # RFD NNNN Your Title"
                    .into(),
            ),
        });
    }

    result
}

/// Validate all RFDs in the repository.
pub fn validate_all(repo_root: &Path, config: &Config) -> Result<bool> {
    let rfd_dir = config.rfd_dir(repo_root);
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut count = 0;

    let mut entries: Vec<_> = std::fs::read_dir(&rfd_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip non-numeric directories
        if name_str.parse::<u32>().is_err() {
            continue;
        }

        let dir = entry.path();
        let file = match find_rfd_file(&dir) {
            Some(f) => f,
            None => {
                eprintln!("RFD {}: {}", name_str, "no README.md found".red());
                total_errors += 1;
                continue;
            }
        };

        count += 1;
        let result = validate_rfd(&file);

        if result.is_ok() && result.warnings.is_empty() {
            eprintln!("RFD {}... {}", name_str, "ok".green());
        } else {
            eprintln!("RFD {}...", name_str);

            // Prefer rich diagnostics when available
            if !result.diagnostics.is_empty() {
                for diag in &result.diagnostics {
                    eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
                }
                total_errors += result.errors.len();
                total_warnings += result.warnings.len();
            } else {
                for err in &result.errors {
                    eprintln!("  {} {}", "ERROR:".red().bold(), err);
                    total_errors += 1;
                }
                for warn in &result.warnings {
                    eprintln!("  {} {}", "WARNING:".yellow().bold(), warn);
                    total_warnings += 1;
                }
            }
        }
    }

    eprintln!();
    if total_errors > 0 {
        eprintln!(
            "Validated {} RFDs: {} error(s), {} warning(s)",
            count, total_errors, total_warnings
        );
    } else {
        eprintln!(
            "{} {} RFDs valid ({} warning(s))",
            "All".green().bold(),
            count,
            total_warnings
        );
    }

    Ok(total_errors == 0)
}

/// Check annotation health for a specific RFD.
/// Reads annotations from the RFD's custom ref (`refs/rfd/NNNN`).
pub fn check_annotations(repo_root: &Path, config: &Config, rfd_number: u32) -> Result<()> {
    let padded = config.pad_number(rfd_number);
    let rfd_dir = repo_root.join("rfd").join(&padded);

    // Read the RFD source from the working tree
    let rfd_file = find_rfd_file(&rfd_dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", padded))?;
    let rfd = Rfd::load(&rfd_file)?;

    // Load annotations from the custom ref
    let repo = crate::refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(rfd_number);
    let collections = AnnotationCollection::load_all_from_ref(&repo, &ref_name)?;

    let mut live = 0;
    let mut approximate = 0;
    let mut stale = 0;
    let mut orphaned = 0;

    for (_filename, collection) in collections {
        for annotation in &collection.items {
            if let crate::annotation::AnnotationTarget::Resource(ref resource) = annotation.target {
                let health = check_selectors(&rfd.body, &resource.selector);
                match health {
                    AnchorHealth::Live => live += 1,
                    AnchorHealth::Approximate => approximate += 1,
                    AnchorHealth::Stale => stale += 1,
                    AnchorHealth::Orphaned => orphaned += 1,
                }
            }
        }
    }

    eprintln!("Annotation health for RFD {}:", padded);
    if live > 0 {
        eprintln!("  {} live", format!("{}", live).green());
    }
    if approximate > 0 {
        eprintln!("  {} approximate", format!("{}", approximate).yellow());
    }
    if stale > 0 {
        eprintln!("  {} stale", format!("{}", stale).yellow());
    }
    if orphaned > 0 {
        eprintln!("  {} orphaned", format!("{}", orphaned).red());
    }
    if live + approximate + stale + orphaned == 0 {
        eprintln!("  (no annotations)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_rfd(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("README.md");
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_validate_valid_rfd() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rfd(
            dir.path(),
            "---\nauthors: Alice\nstate: discussion\n---\n\n# RFD 0001 Test\n\nContent here.\n",
        );
        let result = validate_rfd(&path);
        assert!(result.is_ok());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_missing_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rfd(dir.path(), "# No frontmatter\n\nJust a heading.\n");
        let result = validate_rfd(&path);
        assert!(!result.is_ok());
        assert!(result.errors[0].contains("missing YAML frontmatter"));
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn test_validate_missing_authors_warns() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rfd(
            dir.path(),
            "---\nstate: prediscussion\n---\n\n# RFD 0001 Test\n",
        );
        let result = validate_rfd(&path);
        assert!(result.is_ok()); // warnings don't cause errors
        assert!(result.warnings.iter().any(|w| w.contains("authors")));
        assert!(result.diagnostics.iter().any(|d| d.message.contains("authors")));
    }

    #[test]
    fn test_validate_missing_title_warns() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rfd(
            dir.path(),
            "---\nauthors: Bob\nstate: prediscussion\n---\n\nNo heading here.\n",
        );
        let result = validate_rfd(&path);
        assert!(result.warnings.iter().any(|w| w.contains("title")));
    }


    #[test]
    fn test_validate_nonexistent_file() {
        let result = validate_rfd(Path::new("/nonexistent/file.md"));
        assert!(!result.is_ok());
        assert!(result.errors[0].contains("cannot read file"));
    }
}
