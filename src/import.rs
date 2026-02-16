use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::annotation::*;
use crate::config::Config;

/// Sync result summary.
pub struct SyncResult {
    pub created: usize,
    pub updated: usize,
    pub total: usize,
}

/// Sync annotations from a GitHub PR's review comments.
///
/// Unlike a one-shot import, this merges GitHub's current state with the
/// existing annotation collection:
///
/// - **New comments** get stable IDs derived from the GitHub comment ID.
/// - **Edited comments** have their body text updated and `modified` set.
/// - **Local state** (`resolved`) is preserved across syncs.
/// - **Replies** are threaded via `in_reply_to_id` → `AnnotationTarget::Reference`.
/// - **Deleted comments** (present locally, absent on GitHub) are left as-is
///   — they represent discussion that happened and shouldn't silently vanish.
pub fn sync_pr_annotations(
    repo_root: &Path,
    config: &Config,
    rfd_number: u32,
    pr_number: u32,
) -> Result<SyncResult> {
    let padded = config.pad_number(rfd_number);
    let repo = crate::refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(rfd_number);
    let storage_path = format!("annotations/pr-{}.json", pr_number);

    // 1. Fetch current PR review comments from GitHub
    let comments = fetch_pr_comments(repo_root, pr_number)?;

    // 2. Load existing annotations (if any)
    let existing = AnnotationCollection::load_from_ref(&repo, &ref_name, &storage_path)?;
    let mut existing_by_id: HashMap<String, Annotation> = existing
        .map(|c| c.items)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id.clone(), a))
        .collect();

    let source = format!("rfd/{}/README.md", padded);
    let mut items: Vec<Annotation> = Vec::new();
    let mut created_count = 0;
    let mut updated_count = 0;

    for comment in &comments {
        let gh_id = match comment["id"].as_u64() {
            Some(id) => id,
            None => continue,
        };

        let body_text = comment["body"].as_str().unwrap_or("");
        if body_text.is_empty() {
            continue;
        }

        let annotation_id =
            Annotation::github_id(rfd_number, config.rfd.pad_width, gh_id);

        if let Some(mut existing_ann) = existing_by_id.remove(&annotation_id) {
            // Existing annotation — check if body was edited
            if existing_ann.body.value != body_text {
                existing_ann.body.value = body_text.to_string();
                existing_ann.modified = comment["updated_at"]
                    .as_str()
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());
                updated_count += 1;
            }
            // Preserve local state (resolved, etc.) — don't overwrite
            items.push(existing_ann);
        } else {
            // New annotation
            let annotation = comment_to_annotation(
                comment,
                &annotation_id,
                gh_id,
                &source,
                rfd_number,
                config,
            );
            created_count += 1;
            items.push(annotation);
        }
    }

    // Annotations for comments deleted on GitHub: keep them so discussion
    // history isn't lost. They remain in existing_by_id after the loop.
    let mut orphans: Vec<Annotation> = existing_by_id.into_values().collect();
    orphans.sort_by_key(|a| a.created);
    items.extend(orphans);

    let total = items.len();

    // 3. Write merged collection
    let collection = AnnotationCollection {
        context: "http://www.w3.org/ns/anno.jsonld".into(),
        collection_type: "AnnotationCollection".into(),
        label: format!("PR #{} Discussion", pr_number),
        generator: Some(Generator {
            generator_type: "Software".into(),
            name: "openrfd".into(),
            homepage: Some("https://github.com/openrfd/openrfd".into()),
        }),
        items,
    };

    collection.save_to_ref(
        &repo,
        &ref_name,
        &storage_path,
        &format!("Sync annotations from PR #{}", pr_number),
    )?;

    eprintln!(
        "Synced PR #{}: {} created, {} updated, {} total",
        pr_number, created_count, updated_count, total,
    );

    Ok(SyncResult {
        created: created_count,
        updated: updated_count,
        total,
    })
}

/// Discover the PR number for an RFD branch via `gh pr list`.
pub fn find_pr_for_rfd(repo_root: &Path, config: &Config, rfd_number: u32) -> Result<Option<u32>> {
    let branch = config.branch_name(rfd_number);

    let output = std::process::Command::new("gh")
        .current_dir(repo_root)
        .args([
            "pr",
            "list",
            "--head",
            &branch,
            "--json",
            "number",
            "--limit",
            "1",
        ])
        .output()
        .context("failed to run gh CLI — is it installed?")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr list failed: {}", err);
    }

    let json_str = String::from_utf8(output.stdout).context("invalid UTF-8 from gh")?;
    let prs: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).context("parsing gh pr list JSON")?;

    Ok(prs.first().and_then(|pr| pr["number"].as_u64()).map(|n| n as u32))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fetch all review comments for a PR via `gh api`.
fn fetch_pr_comments(repo_root: &Path, pr_number: u32) -> Result<Vec<serde_json::Value>> {
    let output = std::process::Command::new("gh")
        .current_dir(repo_root)
        .args([
            "api",
            &format!(
                "repos/{{owner}}/{{repo}}/pulls/{}/comments",
                pr_number
            ),
            "--paginate",
        ])
        .output()
        .context("failed to run gh CLI — is it installed?")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh api failed: {}", err);
    }

    let json_str = String::from_utf8(output.stdout).context("invalid UTF-8 from gh")?;
    let comments: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).context("parsing PR comments JSON")?;

    Ok(comments)
}

/// Convert a GitHub review comment JSON object into a W3C Annotation.
fn comment_to_annotation(
    comment: &serde_json::Value,
    annotation_id: &str,
    gh_id: u64,
    source: &str,
    rfd_number: u32,
    config: &Config,
) -> Annotation {
    let body_text = comment["body"].as_str().unwrap_or("");
    let user_name = comment["user"]["login"].as_str().unwrap_or("unknown");
    let user_url = comment["user"]["html_url"].as_str();

    let created = comment["created_at"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(Utc::now);

    let updated = comment["updated_at"]
        .as_str()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    // Set modified only if the comment was actually edited
    let modified = updated.filter(|u| *u != created);

    let commit_id = comment["original_commit_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let diff_hunk = comment["diff_hunk"].as_str().unwrap_or("");
    let line = comment["original_line"].as_u64().unwrap_or(0) as u32;

    // Build selectors from the diff hunk context
    let mut selectors = Vec::new();
    if let Some(last_line) = diff_hunk.lines().last() {
        let exact = last_line.trim_start_matches(['+', '-', ' ']).to_string();
        if !exact.is_empty() {
            selectors.push(Selector::TextQuoteSelector {
                exact,
                prefix: None,
                suffix: None,
            });
        }
    }
    if line > 0 {
        selectors.push(Selector::FragmentSelector {
            value: format!("line={},{}", line, line),
            conforms_to: "http://tools.ietf.org/rfc/rfc5147".into(),
        });
    }

    // Thread replies: GitHub's `in_reply_to_id` links reply comments to
    // the root comment of the conversation thread.
    let in_reply_to = comment["in_reply_to_id"].as_u64();
    let target = if let Some(parent_gh_id) = in_reply_to {
        let parent_id =
            Annotation::github_id(rfd_number, config.rfd.pad_width, parent_gh_id);
        AnnotationTarget::Reference(parent_id)
    } else {
        AnnotationTarget::Resource(SpecificResource {
            resource_type: "SpecificResource".into(),
            source: source.to_string(),
            state: if !commit_id.is_empty() {
                Some(GitState {
                    state_type: "GitState".into(),
                    commit: commit_id,
                    r#ref: Some(config.branch_name(rfd_number)),
                })
            } else {
                None
            },
            selector: selectors,
        })
    };

    // Use Replying motivation for threaded replies
    let motivation = if in_reply_to.is_some() {
        Motivation::Replying
    } else {
        Motivation::Commenting
    };

    Annotation {
        context: "http://www.w3.org/ns/anno.jsonld".into(),
        annotation_type: "Annotation".into(),
        id: annotation_id.to_string(),
        creator: Creator {
            creator_type: "Person".into(),
            name: user_name.to_string(),
            email: None,
            url: user_url.map(|s| s.to_string()),
        },
        created,
        motivation,
        body: AnnotationBody {
            body_type: "TextualBody".into(),
            value: body_text.to_string(),
            format: "text/markdown".into(),
        },
        target,
        modified,
        resolved: None,
        origin: Some(format!("github:{}", gh_id)),
    }
}
