use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

/// Find the repository root by walking up from the current directory.
pub fn find_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        anyhow::bail!("not inside a git repository");
    }

    let path = String::from_utf8(output.stdout)
        .context("invalid UTF-8 in git output")?
        .trim()
        .to_string();

    Ok(PathBuf::from(path))
}

/// Get the current branch name.
pub fn current_branch(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["branch", "--show-current"])
        .output()
        .context("failed to get current branch")?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get the HEAD commit SHA.
pub fn head_sha(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to get HEAD SHA")?;

    if !output.status.success() {
        anyhow::bail!("no commits yet");
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Check if the working tree is clean.
pub fn is_clean(repo_root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to check git status")?;

    Ok(String::from_utf8(output.stdout)?.trim().is_empty())
}

/// Create and checkout a new branch from the current HEAD.
pub fn create_branch(repo_root: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["checkout", "-b", branch])
        .output()
        .context("failed to create branch")?;

    if !output.status.success() {
        // Branch might already exist, try checkout
        let output2 = Command::new("git")
            .current_dir(repo_root)
            .args(["checkout", branch])
            .output()
            .context("failed to checkout branch")?;

        if !output2.status.success() {
            let err = String::from_utf8_lossy(&output2.stderr);
            anyhow::bail!("failed to checkout branch {}: {}", branch, err);
        }
    }

    Ok(())
}

/// Checkout an existing branch.
pub fn checkout(repo_root: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["checkout", branch])
        .output()
        .context("failed to checkout branch")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("failed to checkout {}: {}", branch, err);
    }

    Ok(())
}

/// Check if a branch exists locally.
pub fn branch_exists(repo_root: &Path, branch: &str) -> bool {
    Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--verify", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Stage files and create a commit.
pub fn commit(repo_root: &Path, paths: &[&str], message: &str) -> Result<()> {
    let mut add_args = vec!["add"];
    add_args.extend(paths);

    let output = Command::new("git")
        .current_dir(repo_root)
        .args(&add_args)
        .output()
        .context("failed to git add")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git add failed: {}", err);
    }

    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["commit", "-m", message])
        .output()
        .context("failed to git commit")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git commit failed: {}", err);
    }

    Ok(())
}

/// Push a branch to origin.
pub fn push(repo_root: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["push", "-u", "origin", branch])
        .output()
        .context("failed to push")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("push failed: {}", err);
    }

    Ok(())
}

/// Merge a branch into the current branch with a merge commit.
pub fn merge(repo_root: &Path, branch: &str, message: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["merge", "--no-ff", branch, "-m", message])
        .output()
        .context("failed to merge")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("merge failed: {}", err);
    }

    Ok(())
}

/// Check if there are staged changes.
pub fn has_staged_changes(repo_root: &Path) -> bool {
    Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(false)
}
