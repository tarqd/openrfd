use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::annotation::*;
use crate::config::Config;

/// Import annotations from a GitHub PR's review comments.
///
/// Reads PR review comments via the `gh` CLI and converts them to
/// W3C Web Annotations stored on the RFD's custom ref (`refs/rfd/NNNN`).
pub fn import_pr_annotations(
    repo_root: &Path,
    config: &Config,
    rfd_number: u32,
    pr_number: u32,
) -> Result<Vec<Annotation>> {
    let padded = config.pad_number(rfd_number);

    // Use gh CLI to fetch PR review comments
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

    let source = format!("rfd/{}/README.md", padded);
    let mut annotations = Vec::new();

    for comment in &comments {
        let body_text = comment["body"].as_str().unwrap_or("");
        if body_text.is_empty() {
            continue;
        }

        let user_name = comment["user"]["login"].as_str().unwrap_or("unknown");
        let user_url = comment["user"]["html_url"].as_str();
        let created = comment["created_at"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Utc::now);
        let commit_id = comment["original_commit_id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let diff_hunk = comment["diff_hunk"].as_str().unwrap_or("");
        let line = comment["original_line"].as_u64().unwrap_or(0) as u32;

        // Build selectors from the diff hunk context
        let mut selectors = Vec::new();

        // Extract the last line of the diff hunk as the quoted text
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

        let annotation = Annotation {
            context: "http://www.w3.org/ns/anno.jsonld".into(),
            annotation_type: "Annotation".into(),
            id: Annotation::new_id(rfd_number, config.rfd.pad_width),
            creator: Creator {
                creator_type: "Person".into(),
                name: user_name.to_string(),
                email: None,
                url: user_url.map(|s| s.to_string()),
            },
            created,
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: "TextualBody".into(),
                value: body_text.to_string(),
                format: "text/markdown".into(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: "SpecificResource".into(),
                source: source.clone(),
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
            }),
            resolved: None,
        };

        annotations.push(annotation);
    }

    // Save to the RFD's custom ref (never touches working tree)
    if !annotations.is_empty() {
        let collection = AnnotationCollection {
            context: "http://www.w3.org/ns/anno.jsonld".into(),
            collection_type: "AnnotationCollection".into(),
            label: format!("PR #{} Discussion", pr_number),
            generator: Some(Generator {
                generator_type: "Software".into(),
                name: "openrfd".into(),
                homepage: Some("https://github.com/openrfd/openrfd".into()),
            }),
            items: annotations.clone(),
        };

        let repo = crate::refs::open_repo(repo_root)?;
        let ref_name = config.ref_name(rfd_number);
        let path = format!("annotations/pr-{}.json", pr_number);
        collection.save_to_ref(
            &repo,
            &ref_name,
            &path,
            &format!("Import annotations from PR #{}", pr_number),
        )?;

        eprintln!(
            "Imported {} annotations from PR #{} to {}",
            collection.items.len(),
            pr_number,
            ref_name
        );
    } else {
        eprintln!("No review comments found on PR #{}", pr_number);
    }

    Ok(annotations)
}
