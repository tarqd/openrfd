//! Low-level git plumbing for reading and writing trees on custom refs
//! (e.g., `refs/rfd/0042`) without touching the working tree or index.
//!
//! This is the storage layer for annotations, metadata, and revision snapshots.
//! Inspired by Gerrit NoteDb — every mutation is a commit on the ref, the tree
//! at HEAD always reflects current state, and `git log` is the audit trail.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use git2::{Oid, Repository, Signature};

/// A file to write into the ref's tree. Path is relative to tree root
/// (e.g., "annotations/pr-47.json").
pub struct TreeEntry {
    pub path: String,
    pub content: Vec<u8>,
}

/// Open the repository at the given root.
pub fn open_repo(repo_root: &Path) -> Result<Repository> {
    Repository::open(repo_root).context("failed to open git repository")
}

/// Read a file from the tree at the tip of a ref.
/// Returns None if the ref or file doesn't exist.
pub fn read_file(repo: &Repository, ref_name: &str, path: &str) -> Result<Option<Vec<u8>>> {
    let reference = match repo.find_reference(ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let commit = reference
        .peel_to_commit()
        .context("ref does not point to a commit")?;
    let tree = commit.tree()?;

    match tree.get_path(std::path::Path::new(path)) {
        Ok(entry) => {
            let obj = entry.to_object(repo)?;
            let blob = obj
                .as_blob()
                .ok_or_else(|| anyhow::anyhow!("{} is not a blob", path))?;
            Ok(Some(blob.content().to_vec()))
        }
        Err(_) => Ok(None),
    }
}

/// List files in a directory within the tree at the tip of a ref.
/// Returns empty vec if the ref or directory doesn't exist.
pub fn list_dir(repo: &Repository, ref_name: &str, dir: &str) -> Result<Vec<String>> {
    let reference = match repo.find_reference(ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(vec![]),
    };

    let commit = reference
        .peel_to_commit()
        .context("ref does not point to a commit")?;
    let tree = commit.tree()?;

    let subtree = if dir.is_empty() {
        tree
    } else {
        match tree.get_path(std::path::Path::new(dir)) {
            Ok(entry) => {
                let obj = entry.to_object(repo)?;
                obj.into_tree()
                    .map_err(|_| anyhow::anyhow!("{} is not a tree", dir))?
            }
            Err(_) => return Ok(vec![]),
        }
    };

    let mut names = Vec::new();
    for entry in subtree.iter() {
        if let Some(name) = entry.name() {
            names.push(name.to_string());
        }
    }

    Ok(names)
}

/// Create a commit on a custom ref, building a tree from scratch or
/// updating the existing tree. This never touches the working directory.
///
/// `entries` contains the files to write. Existing files in the tree not
/// mentioned in `entries` are preserved. To remove a file, omit it and
/// set `replace_tree` to true, or use `remove_files`.
///
/// Returns the new commit OID.
pub fn write_commit(
    repo: &Repository,
    ref_name: &str,
    message: &str,
    entries: &[TreeEntry],
    remove_paths: &[&str],
) -> Result<Oid> {
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("openrfd", "openrfd@localhost"))
        .context("could not determine git signature")?;

    // Get current tip commit (if ref exists)
    let parent = repo
        .find_reference(ref_name)
        .ok()
        .and_then(|r| r.peel_to_commit().ok());

    // Build a flat map of path -> blob OID from the existing tree
    let mut file_map: BTreeMap<String, Oid> = BTreeMap::new();
    if let Some(ref parent_commit) = parent {
        collect_tree_entries(repo, &parent_commit.tree()?, "", &mut file_map)?;
    }

    // Apply removals
    for path in remove_paths {
        file_map.remove(*path);
    }

    // Apply new/updated entries
    for entry in entries {
        let blob_oid = repo.blob(&entry.content)?;
        file_map.insert(entry.path.clone(), blob_oid);
    }

    // Build the nested tree structure from the flat map
    let tree_oid = build_nested_tree(repo, &file_map)?;
    let tree = repo.find_tree(tree_oid)?;

    // Create the commit
    let parents: Vec<&git2::Commit> = parent.as_ref().into_iter().collect();
    let commit_oid = repo.commit(Some(ref_name), &sig, &sig, message, &tree, &parents)?;

    Ok(commit_oid)
}

/// Create the initial commit on a new ref (no parent).
pub fn create_ref(
    repo: &Repository,
    ref_name: &str,
    message: &str,
    entries: &[TreeEntry],
) -> Result<Oid> {
    // Ensure the ref doesn't already exist
    if repo.find_reference(ref_name).is_ok() {
        anyhow::bail!("ref {} already exists", ref_name);
    }

    let sig = repo
        .signature()
        .or_else(|_| Signature::now("openrfd", "openrfd@localhost"))
        .context("could not determine git signature")?;

    let mut file_map: BTreeMap<String, Oid> = BTreeMap::new();
    for entry in entries {
        let blob_oid = repo.blob(&entry.content)?;
        file_map.insert(entry.path.clone(), blob_oid);
    }

    let tree_oid = build_nested_tree(repo, &file_map)?;
    let tree = repo.find_tree(tree_oid)?;

    let commit_oid = repo.commit(Some(ref_name), &sig, &sig, message, &tree, &[])?;

    Ok(commit_oid)
}

/// Check if a ref exists.
pub fn ref_exists(repo: &Repository, ref_name: &str) -> bool {
    repo.find_reference(ref_name).is_ok()
}

/// Read the commit log of a ref, returning (message, timestamp) pairs.
pub fn read_log(repo: &Repository, ref_name: &str, max_count: usize) -> Result<Vec<LogEntry>> {
    let reference = match repo.find_reference(ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(vec![]),
    };

    let commit = reference.peel_to_commit()?;
    let mut entries = Vec::new();
    let mut current = Some(commit);

    while let Some(c) = current {
        if entries.len() >= max_count {
            break;
        }

        entries.push(LogEntry {
            oid: c.id(),
            message: c.message().unwrap_or("").to_string(),
            author: c.author().name().unwrap_or("").to_string(),
            timestamp: c.time().seconds(),
        });

        current = c.parent(0).ok();
    }

    Ok(entries)
}

/// A commit log entry.
pub struct LogEntry {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

impl LogEntry {
    /// Parse a structured footer value from the commit message.
    /// Footers are lines in the format "Key: Value" after the first blank line.
    pub fn footer(&self, key: &str) -> Option<String> {
        let prefix = format!("{}: ", key);
        for line in self.message.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(&prefix) {
                return Some(rest.to_string());
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively collect all blob entries from a tree into a flat path -> OID map.
fn collect_tree_entries(
    repo: &Repository,
    tree: &git2::Tree,
    prefix: &str,
    map: &mut BTreeMap<String, Oid>,
) -> Result<()> {
    for entry in tree.iter() {
        let name = entry.name().unwrap_or("");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };

        match entry.kind() {
            Some(git2::ObjectType::Blob) => {
                map.insert(path, entry.id());
            }
            Some(git2::ObjectType::Tree) => {
                let subtree = repo.find_tree(entry.id())?;
                collect_tree_entries(repo, &subtree, &path, map)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Build a nested git tree from a flat map of "path/to/file" -> blob OID.
///
/// For example, given:
///   "README.md" -> oid1
///   "annotations/pr-47.json" -> oid2
///   "annotations/cli.json" -> oid3
///
/// Produces a tree with:
///   README.md (blob)
///   annotations/ (tree)
///     pr-47.json (blob)
///     cli.json (blob)
fn build_nested_tree(repo: &Repository, file_map: &BTreeMap<String, Oid>) -> Result<Oid> {
    // Group files by top-level directory
    let mut top_level_blobs: BTreeMap<&str, Oid> = BTreeMap::new();
    let mut subdirs: BTreeMap<String, BTreeMap<String, Oid>> = BTreeMap::new();

    for (path, oid) in file_map {
        if let Some(slash_pos) = path.find('/') {
            let dir = &path[..slash_pos];
            let rest = &path[slash_pos + 1..];
            subdirs
                .entry(dir.to_string())
                .or_default()
                .insert(rest.to_string(), *oid);
        } else {
            top_level_blobs.insert(path.as_str(), *oid);
        }
    }

    let mut builder = repo.treebuilder(None)?;

    // Add top-level blobs
    for (name, oid) in &top_level_blobs {
        builder.insert(name, *oid, 0o100644)?;
    }

    // Recursively build subtrees
    for (dir_name, sub_map) in &subdirs {
        let subtree_oid = build_nested_tree(repo, sub_map)?;
        builder.insert(dir_name, subtree_oid, 0o040000)?;
    }

    let tree_oid = builder.write()?;
    Ok(tree_oid)
}

/// Push a custom ref to the remote.
pub fn push_ref(repo_root: &Path, ref_name: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["push", "origin", ref_name])
        .output()
        .context("failed to push ref")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("push {} failed: {}", ref_name, err);
    }

    Ok(())
}

/// Fetch custom refs from origin.
pub fn fetch_rfd_refs(repo_root: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["fetch", "origin", "refs/rfd/*:refs/rfd/*"])
        .output()
        .context("failed to fetch rfd refs")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        // Not fatal — refs may not exist on remote yet
        eprintln!("warning: fetching rfd refs: {}", err.trim());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an initial commit so we have a valid HEAD
        {
            let sig = Signature::now("test", "test@test.com").unwrap();
            let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn test_create_and_read_ref() {
        let (_dir, repo) = temp_repo();

        let entries = vec![
            TreeEntry {
                path: "README.md".into(),
                content: b"# RFD 0001 Test".to_vec(),
            },
            TreeEntry {
                path: "annotations/cli.json".into(),
                content: b"{}".to_vec(),
            },
        ];

        create_ref(&repo, "refs/rfd/0001", "Reserve RFD 0001\n\nState: prediscussion", &entries).unwrap();

        // Read back
        let content = read_file(&repo, "refs/rfd/0001", "README.md").unwrap();
        assert_eq!(content, Some(b"# RFD 0001 Test".to_vec()));

        let ann = read_file(&repo, "refs/rfd/0001", "annotations/cli.json").unwrap();
        assert_eq!(ann, Some(b"{}".to_vec()));

        // List annotations dir
        let files = list_dir(&repo, "refs/rfd/0001", "annotations").unwrap();
        assert_eq!(files, vec!["cli.json"]);
    }

    #[test]
    fn test_write_commit_preserves_existing() {
        let (_dir, repo) = temp_repo();

        let initial = vec![
            TreeEntry {
                path: "README.md".into(),
                content: b"initial".to_vec(),
            },
        ];
        create_ref(&repo, "refs/rfd/0001", "init", &initial).unwrap();

        // Add a file, preserving README.md
        let update = vec![TreeEntry {
            path: "annotations/new.json".into(),
            content: b"new".to_vec(),
        }];
        write_commit(&repo, "refs/rfd/0001", "add annotation", &update, &[]).unwrap();

        // Both files should exist
        assert!(read_file(&repo, "refs/rfd/0001", "README.md").unwrap().is_some());
        assert!(read_file(&repo, "refs/rfd/0001", "annotations/new.json").unwrap().is_some());
    }

    #[test]
    fn test_read_log() {
        let (_dir, repo) = temp_repo();

        let entries = vec![TreeEntry {
            path: "README.md".into(),
            content: b"v1".to_vec(),
        }];
        create_ref(&repo, "refs/rfd/0001", "Reserve RFD 0001\n\nState: prediscussion\nRevision: 1", &entries).unwrap();

        write_commit(
            &repo,
            "refs/rfd/0001",
            "State: discussion\n\nDiscussion: https://example.com/pr/1",
            &[],
            &[],
        ).unwrap();

        let log = read_log(&repo, "refs/rfd/0001", 10).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].footer("State"), Some("discussion".into()));
        assert_eq!(log[0].footer("Discussion"), Some("https://example.com/pr/1".into()));
        assert_eq!(log[1].footer("State"), Some("prediscussion".into()));
        assert_eq!(log[1].footer("Revision"), Some("1".into()));
    }

    #[test]
    fn test_remove_files() {
        let (_dir, repo) = temp_repo();

        let entries = vec![
            TreeEntry { path: "a.txt".into(), content: b"a".to_vec() },
            TreeEntry { path: "b.txt".into(), content: b"b".to_vec() },
        ];
        create_ref(&repo, "refs/rfd/0001", "init", &entries).unwrap();

        write_commit(&repo, "refs/rfd/0001", "remove b", &[], &["b.txt"]).unwrap();

        assert!(read_file(&repo, "refs/rfd/0001", "a.txt").unwrap().is_some());
        assert!(read_file(&repo, "refs/rfd/0001", "b.txt").unwrap().is_none());
    }
}
