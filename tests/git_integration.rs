//! Integration tests for the NoteDb-style ref-based storage.
//!
//! These tests exercise the full workflow of storing annotations and state
//! transitions on custom git refs (`refs/rfd/NNNN`), using real git repos
//! created by git2.

use chrono::Utc;
use git2::{Repository, Signature};
use openrfd::annotation::*;
use openrfd::config::Config;
use openrfd::refs::{self, TreeEntry};

/// Create a temporary git repo with an initial commit.
fn init_repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    {
        let sig = Signature::now("test", "test@test.com").unwrap();
        let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }

    (dir, repo)
}

fn make_annotation(rfd: u32, creator: &str, body: &str) -> Annotation {
    Annotation {
        context: "http://www.w3.org/ns/anno.jsonld".into(),
        annotation_type: "Annotation".into(),
        id: Annotation::new_id(rfd, 4),
        creator: Creator {
            creator_type: "Person".into(),
            name: creator.into(),
            email: None,
            url: None,
        },
        created: Utc::now(),
        motivation: Motivation::Commenting,
        body: AnnotationBody {
            body_type: "TextualBody".into(),
            value: body.into(),
            format: "text/markdown".into(),
        },
        target: AnnotationTarget::Resource(SpecificResource {
            resource_type: "SpecificResource".into(),
            source: format!("rfd/{:04}/README.md", rfd),
            state: None,
            selector: vec![Selector::TextQuoteSelector {
                exact: "some text".into(),
                prefix: None,
                suffix: None,
            }],
        }),
        modified: None,
        resolved: None,
        origin: None,
    }
}

// -------------------------------------------------------------------------
// Annotation lifecycle
// -------------------------------------------------------------------------

#[test]
fn test_annotation_save_and_load_roundtrip() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0042";

    // Create the ref with an initial content snapshot
    refs::create_ref(
        &repo,
        ref_name,
        "Reserve RFD 0042\n\nState: prediscussion",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# RFD 0042 Test".to_vec(),
        }],
    )
    .unwrap();

    // Save an annotation collection
    let mut collection = AnnotationCollection::new("CLI Annotations");
    collection.items.push(make_annotation(42, "alice", "Looks good"));
    collection
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "Add annotation")
        .unwrap();

    // Load it back
    let loaded = AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
        .unwrap()
        .expect("collection should exist");

    assert_eq!(loaded.label, "CLI Annotations");
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].creator.name, "alice");
    assert_eq!(loaded.items[0].body.value, "Looks good");

    // Verify the README is still intact
    let readme = refs::read_file(&repo, ref_name, "README.md")
        .unwrap()
        .unwrap();
    assert_eq!(readme, b"# RFD 0042 Test");
}

#[test]
fn test_annotation_accumulation() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"content".to_vec(),
        }],
    )
    .unwrap();

    // First annotation
    let mut collection = AnnotationCollection::new("CLI Annotations");
    collection.items.push(make_annotation(1, "alice", "first"));
    collection
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "first annotation")
        .unwrap();

    // Load, add second, save
    let mut collection =
        AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
            .unwrap()
            .unwrap();
    collection.items.push(make_annotation(1, "bob", "second"));
    collection
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "second annotation")
        .unwrap();

    // Verify both are there
    let loaded = AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.items.len(), 2);
    assert_eq!(loaded.items[0].creator.name, "alice");
    assert_eq!(loaded.items[1].creator.name, "bob");
}

// -------------------------------------------------------------------------
// Multiple annotation sources
// -------------------------------------------------------------------------

#[test]
fn test_multiple_annotation_collections() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"content".to_vec(),
        }],
    )
    .unwrap();

    // Save CLI annotations
    let mut cli = AnnotationCollection::new("CLI Annotations");
    cli.items.push(make_annotation(1, "alice", "cli comment"));
    cli.save_to_ref(&repo, ref_name, "annotations/cli.json", "cli annotation")
        .unwrap();

    // Save PR annotations
    let mut pr = AnnotationCollection::new("PR #7 Discussion");
    pr.items.push(make_annotation(1, "bob", "pr comment 1"));
    pr.items.push(make_annotation(1, "carol", "pr comment 2"));
    pr.save_to_ref(&repo, ref_name, "annotations/pr-7.json", "pr annotations")
        .unwrap();

    // load_all should return both
    let all = AnnotationCollection::load_all_from_ref(&repo, ref_name).unwrap();
    assert_eq!(all.len(), 2);

    let total_items: usize = all.iter().map(|(_, c)| c.items.len()).sum();
    assert_eq!(total_items, 3);

    // Check filenames
    let filenames: Vec<&str> = all.iter().map(|(name, _)| name.as_str()).collect();
    assert!(filenames.contains(&"cli.json"));
    assert!(filenames.contains(&"pr-7.json"));
}

// -------------------------------------------------------------------------
// Resolve annotation
// -------------------------------------------------------------------------

#[test]
fn test_resolve_annotation() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"content".to_vec(),
        }],
    )
    .unwrap();

    // Create two annotations
    let mut collection = AnnotationCollection::new("Test");
    let ann1 = make_annotation(1, "alice", "unresolved");
    let ann2 = make_annotation(1, "bob", "also unresolved");
    let ann1_id = ann1.id.clone();
    collection.items.push(ann1);
    collection.items.push(ann2);
    collection
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "add annotations")
        .unwrap();

    // Resolve the first one
    let mut loaded = AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
        .unwrap()
        .unwrap();
    for ann in &mut loaded.items {
        if ann.id == ann1_id {
            ann.resolved = Some(Utc::now());
        }
    }
    loaded
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "resolve annotation")
        .unwrap();

    // Verify
    let final_loaded =
        AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
            .unwrap()
            .unwrap();
    assert!(final_loaded.items[0].resolved.is_some());
    assert!(final_loaded.items[1].resolved.is_none());
}

// -------------------------------------------------------------------------
// State transitions (commit log as audit trail)
// -------------------------------------------------------------------------

#[test]
fn test_state_transition_log() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    // Reserve
    refs::create_ref(
        &repo,
        ref_name,
        "Reserve RFD 0001\n\nState: prediscussion",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# RFD 0001 v1\nstate: prediscussion".to_vec(),
        }],
    )
    .unwrap();

    // Transition to discussion
    refs::write_commit(
        &repo,
        ref_name,
        "State: discussion\n\nPrevious-State: prediscussion\nDiscussion: https://github.com/example/pr/1",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# RFD 0001 v2\nstate: discussion".to_vec(),
        }],
        &[],
    )
    .unwrap();

    // Transition to published
    refs::write_commit(
        &repo,
        ref_name,
        "State: published\n\nPrevious-State: discussion",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# RFD 0001 v3\nstate: published".to_vec(),
        }],
        &[],
    )
    .unwrap();

    // Read the full log
    let log = refs::read_log(&repo, ref_name, 10).unwrap();
    assert_eq!(log.len(), 3);

    // Most recent first
    assert_eq!(log[0].footer("State"), Some("published".into()));
    assert_eq!(
        log[0].footer("Previous-State"),
        Some("discussion".into())
    );

    assert_eq!(log[1].footer("State"), Some("discussion".into()));
    assert_eq!(
        log[1].footer("Previous-State"),
        Some("prediscussion".into())
    );
    assert_eq!(
        log[1].footer("Discussion"),
        Some("https://github.com/example/pr/1".into())
    );

    assert_eq!(log[2].footer("State"), Some("prediscussion".into()));

    // Verify tree at HEAD has the latest content
    let readme = refs::read_file(&repo, ref_name, "README.md")
        .unwrap()
        .unwrap();
    assert_eq!(
        String::from_utf8(readme).unwrap(),
        "# RFD 0001 v3\nstate: published"
    );
}

// -------------------------------------------------------------------------
// Content snapshot preservation across annotation writes
// -------------------------------------------------------------------------

#[test]
fn test_annotations_dont_clobber_content_snapshots() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    // Create ref with content snapshot
    refs::create_ref(
        &repo,
        ref_name,
        "Reserve RFD\n\nState: prediscussion",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# Important Content".to_vec(),
        }],
    )
    .unwrap();

    // Write annotation (no README.md in the entries)
    let mut collection = AnnotationCollection::new("Test");
    collection
        .items
        .push(make_annotation(1, "alice", "comment"));
    collection
        .save_to_ref(&repo, ref_name, "annotations/cli.json", "add annotation")
        .unwrap();

    // README.md should still be there
    let readme = refs::read_file(&repo, ref_name, "README.md")
        .unwrap()
        .unwrap();
    assert_eq!(readme, b"# Important Content");

    // Write a state transition with new content
    refs::write_commit(
        &repo,
        ref_name,
        "State: discussion",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"# Updated Content".to_vec(),
        }],
        &[],
    )
    .unwrap();

    // Both README and annotations should exist
    let readme = refs::read_file(&repo, ref_name, "README.md")
        .unwrap()
        .unwrap();
    assert_eq!(readme, b"# Updated Content");

    let ann = AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
        .unwrap()
        .unwrap();
    assert_eq!(ann.items.len(), 1);
}

// -------------------------------------------------------------------------
// Config helper integration
// -------------------------------------------------------------------------

#[test]
fn test_config_ref_name_used_with_refs() {
    let (_dir, repo) = init_repo();
    let config = Config::default();
    let ref_name = config.ref_name(42);

    assert_eq!(ref_name, "refs/rfd/0042");

    refs::create_ref(
        &repo,
        &ref_name,
        "test",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"test".to_vec(),
        }],
    )
    .unwrap();

    assert!(refs::ref_exists(&repo, &ref_name));
    assert!(!refs::ref_exists(&repo, "refs/rfd/9999"));
}

// -------------------------------------------------------------------------
// Edge cases
// -------------------------------------------------------------------------

#[test]
fn test_load_from_nonexistent_ref() {
    let (_dir, repo) = init_repo();

    // load_from_ref on a ref that doesn't exist returns None
    let result =
        AnnotationCollection::load_from_ref(&repo, "refs/rfd/9999", "annotations/cli.json")
            .unwrap();
    assert!(result.is_none());

    // load_all returns empty
    let all = AnnotationCollection::load_all_from_ref(&repo, "refs/rfd/9999").unwrap();
    assert!(all.is_empty());
}

#[test]
fn test_load_from_ref_without_annotations_dir() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    // Create ref with only README, no annotations/ tree
    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"content".to_vec(),
        }],
    )
    .unwrap();

    let all = AnnotationCollection::load_all_from_ref(&repo, ref_name).unwrap();
    assert!(all.is_empty());
}

#[test]
fn test_write_commit_creates_ref_if_missing() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    // write_commit on a ref that doesn't exist should create it
    refs::write_commit(
        &repo,
        ref_name,
        "auto-create",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"created".to_vec(),
        }],
        &[],
    )
    .unwrap();

    let content = refs::read_file(&repo, ref_name, "README.md")
        .unwrap()
        .unwrap();
    assert_eq!(content, b"created");
}

#[test]
fn test_create_ref_fails_if_exists() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "first",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"first".to_vec(),
        }],
    )
    .unwrap();

    // Second create should fail
    let result = refs::create_ref(
        &repo,
        ref_name,
        "second",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"second".to_vec(),
        }],
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_deeply_nested_tree_paths() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[
            TreeEntry {
                path: "a/b/c.txt".into(),
                content: b"deep".to_vec(),
            },
            TreeEntry {
                path: "a/d.txt".into(),
                content: b"shallow".to_vec(),
            },
        ],
    )
    .unwrap();

    let deep = refs::read_file(&repo, ref_name, "a/b/c.txt")
        .unwrap()
        .unwrap();
    assert_eq!(deep, b"deep");

    let shallow = refs::read_file(&repo, ref_name, "a/d.txt")
        .unwrap()
        .unwrap();
    assert_eq!(shallow, b"shallow");

    let a_contents = refs::list_dir(&repo, ref_name, "a").unwrap();
    assert!(a_contents.contains(&"b".to_string()));
    assert!(a_contents.contains(&"d.txt".to_string()));
}

#[test]
fn test_concurrent_annotation_sources_independent() {
    let (_dir, repo) = init_repo();
    let ref_name = "refs/rfd/0001";

    refs::create_ref(
        &repo,
        ref_name,
        "init",
        &[TreeEntry {
            path: "README.md".into(),
            content: b"content".to_vec(),
        }],
    )
    .unwrap();

    // Save CLI collection
    let mut cli = AnnotationCollection::new("CLI");
    cli.items.push(make_annotation(1, "alice", "cli 1"));
    cli.save_to_ref(&repo, ref_name, "annotations/cli.json", "cli")
        .unwrap();

    // Save PR collection (separate file — shouldn't affect cli.json)
    let mut pr = AnnotationCollection::new("PR");
    pr.items.push(make_annotation(1, "bob", "pr 1"));
    pr.save_to_ref(&repo, ref_name, "annotations/pr-1.json", "pr")
        .unwrap();

    // Update CLI collection (add second annotation)
    let mut cli =
        AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
            .unwrap()
            .unwrap();
    assert_eq!(cli.items.len(), 1); // PR save didn't corrupt CLI
    cli.items.push(make_annotation(1, "alice", "cli 2"));
    cli.save_to_ref(&repo, ref_name, "annotations/cli.json", "cli update")
        .unwrap();

    // Verify both are intact
    let cli =
        AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/cli.json")
            .unwrap()
            .unwrap();
    assert_eq!(cli.items.len(), 2);

    let pr =
        AnnotationCollection::load_from_ref(&repo, ref_name, "annotations/pr-1.json")
            .unwrap()
            .unwrap();
    assert_eq!(pr.items.len(), 1);

    // Log should have 4 commits: init + cli + pr + cli update
    let log = refs::read_log(&repo, ref_name, 10).unwrap();
    assert_eq!(log.len(), 4);
}
