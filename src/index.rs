use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::rfd::{find_rfd_file, Rfd};

/// Regenerate the `rfd.csv` index file.
pub fn generate_csv_index(repo_root: &Path, config: &Config) -> Result<()> {
    let rfd_dir = config.rfd_dir(repo_root);
    let csv_path = repo_root.join("rfd.csv");

    let mut writer = csv::Writer::from_path(&csv_path)
        .with_context(|| format!("creating {}", csv_path.display()))?;

    writer.write_record(["number", "title", "state", "authors", "discussion"])?;

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

        writer.write_record([
            &name_str,
            &rfd.title(),
            rfd.frontmatter.state.as_str(),
            rfd.frontmatter.authors.as_deref().unwrap_or(""),
            rfd.frontmatter.discussion.as_deref().unwrap_or(""),
        ])?;
    }

    writer.flush()?;
    eprintln!("Updated {}", csv_path.display());
    Ok(())
}
