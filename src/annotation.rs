use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// W3C Web Annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    #[serde(rename = "@context", default = "default_context")]
    pub context: String,
    #[serde(rename = "type", default = "default_annotation_type")]
    pub annotation_type: String,
    pub id: String,
    pub creator: Creator,
    pub created: DateTime<Utc>,
    #[serde(default = "default_motivation")]
    pub motivation: Motivation,
    pub body: AnnotationBody,
    pub target: AnnotationTarget,
    /// W3C `modified` — set when the annotation body is updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
    /// If this annotation is resolved/closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<DateTime<Utc>>,
    /// Provenance: links this annotation to an external source (e.g. GitHub comment ID).
    /// Format: `github:<comment_id>` for PR review comments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

/// W3C AnnotationCollection — groups annotations by source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationCollection {
    #[serde(rename = "@context", default = "default_context")]
    pub context: String,
    #[serde(rename = "type", default = "default_collection_type")]
    pub collection_type: String,
    pub label: String,
    #[serde(default)]
    pub generator: Option<Generator>,
    #[serde(default)]
    pub items: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generator {
    #[serde(rename = "type")]
    pub generator_type: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    #[serde(rename = "type")]
    pub creator_type: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motivation {
    Commenting,
    Replying,
    Questioning,
    Suggesting,
    Editing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationBody {
    #[serde(rename = "type", default = "default_textual_body")]
    pub body_type: String,
    pub value: String,
    #[serde(default = "default_format")]
    pub format: String,
}

/// The target of an annotation — a specific resource with selectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnnotationTarget {
    /// A reply targeting another annotation by ID.
    Reference(String),
    /// A specific resource with selectors.
    Resource(SpecificResource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificResource {
    #[serde(rename = "type", default = "default_specific_resource")]
    pub resource_type: String,
    /// Path to the source file (e.g., "rfd/0042/README.md").
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<GitState>,
    #[serde(default)]
    pub selector: Vec<Selector>,
}

/// Custom state type extending W3C model with Git provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    #[serde(rename = "type", default = "default_git_state")]
    pub state_type: String,
    /// The commit SHA the annotation was created against.
    pub commit: String,
    /// The branch the annotation was created on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Selector {
    TextQuoteSelector {
        exact: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        suffix: Option<String>,
    },
    FragmentSelector {
        value: String,
        #[serde(rename = "conformsTo")]
        conforms_to: String,
    },
    TextPositionSelector {
        start: usize,
        end: usize,
    },
}

// Default value helpers for serde
fn default_context() -> String {
    "http://www.w3.org/ns/anno.jsonld".into()
}

fn default_annotation_type() -> String {
    "Annotation".into()
}

fn default_collection_type() -> String {
    "AnnotationCollection".into()
}

fn default_motivation() -> Motivation {
    Motivation::Commenting
}

fn default_textual_body() -> String {
    "TextualBody".into()
}

fn default_format() -> String {
    "text/markdown".into()
}

fn default_specific_resource() -> String {
    "SpecificResource".into()
}

fn default_git_state() -> String {
    "GitState".into()
}

impl Annotation {
    /// Generate a new annotation ID in the format `urn:openrfd:NNNN:<uuid>`.
    pub fn new_id(rfd_number: u32, pad_width: u32) -> String {
        let padded = format!("{:0>width$}", rfd_number, width = pad_width as usize);
        format!("urn:openrfd:{}:{}", padded, Uuid::new_v4())
    }

    /// Generate a stable annotation ID derived from a GitHub comment ID.
    /// Format: `urn:openrfd:NNNN:gh-<comment_id>`.
    pub fn github_id(rfd_number: u32, pad_width: u32, comment_id: u64) -> String {
        let padded = format!("{:0>width$}", rfd_number, width = pad_width as usize);
        format!("urn:openrfd:{}:gh-{}", padded, comment_id)
    }

    /// Returns true if this annotation was imported from GitHub.
    pub fn is_from_github(&self) -> bool {
        self.origin
            .as_ref()
            .map_or(false, |o| o.starts_with("github:"))
    }

    /// Check if this annotation is a reply to another.
    pub fn is_reply(&self) -> bool {
        matches!(self.target, AnnotationTarget::Reference(_))
    }

    /// Get the annotation's reply target ID, if it's a reply.
    pub fn reply_to(&self) -> Option<&str> {
        match &self.target {
            AnnotationTarget::Reference(id) => Some(id),
            _ => None,
        }
    }
}

impl AnnotationCollection {
    /// Create a new empty collection.
    pub fn new(label: &str) -> Self {
        Self {
            context: default_context(),
            collection_type: default_collection_type(),
            label: label.to_string(),
            generator: Some(Generator {
                generator_type: "Software".into(),
                name: "openrfd".into(),
                homepage: Some("https://github.com/openrfd/openrfd".into()),
            }),
            items: vec![],
        }
    }

    /// Parse from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        serde_json::from_slice(data).map_err(|e| {
            // Try to provide a diagnostic with the JSON source
            let source = String::from_utf8_lossy(data);
            let offset = e.column().saturating_sub(1);
            let diag = crate::diagnostic::InvalidAnnotationJson {
                src: miette::NamedSource::new("annotations.json", source.into_owned()),
                span: (offset, 1).into(),
                reason: e.to_string(),
                advice: Some("check the JSON structure matches the W3C annotation format".into()),
            };
            anyhow::Error::new(diag)
        })
    }

    /// Serialize to pretty JSON bytes.
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let content = serde_json::to_string_pretty(self)?;
        Ok(content.into_bytes())
    }

    /// Load from a ref's tree (e.g., `refs/rfd/0042`, path `annotations/cli.json`).
    pub fn load_from_ref(
        repo: &git2::Repository,
        ref_name: &str,
        path: &str,
    ) -> anyhow::Result<Option<Self>> {
        match crate::refs::read_file(repo, ref_name, path)? {
            Some(data) => Ok(Some(Self::from_bytes(&data)?)),
            None => Ok(None),
        }
    }

    /// Save to a ref's tree, creating a commit.
    pub fn save_to_ref(
        &self,
        repo: &git2::Repository,
        ref_name: &str,
        path: &str,
        message: &str,
    ) -> anyhow::Result<()> {
        let data = self.to_bytes()?;
        let entry = crate::refs::TreeEntry {
            path: path.to_string(),
            content: data,
        };
        crate::refs::write_commit(repo, ref_name, message, &[entry], &[])?;
        Ok(())
    }

    /// Load all annotation collections from a ref's annotations/ directory.
    pub fn load_all_from_ref(
        repo: &git2::Repository,
        ref_name: &str,
    ) -> anyhow::Result<Vec<(String, Self)>> {
        let files = crate::refs::list_dir(repo, ref_name, "annotations")?;
        let mut collections = Vec::new();

        for file in files {
            if file.ends_with(".json") {
                let path = format!("annotations/{}", file);
                if let Some(data) = crate::refs::read_file(repo, ref_name, &path)? {
                    let collection = Self::from_bytes(&data)?;
                    collections.push((file, collection));
                }
            }
        }

        Ok(collections)
    }
}

/// Health status of an annotation's anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorHealth {
    /// TextQuoteSelector matches current source exactly.
    Live,
    /// TextQuoteSelector matches approximately (fuzzy).
    Approximate,
    /// No selector matches; annotation needs re-anchoring.
    Stale,
    /// Text was deleted or changed beyond recognition.
    Orphaned,
}

impl std::fmt::Display for AnchorHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorHealth::Live => write!(f, "live"),
            AnchorHealth::Approximate => write!(f, "approximate"),
            AnchorHealth::Stale => write!(f, "stale"),
            AnchorHealth::Orphaned => write!(f, "orphaned"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_id_format() {
        let id = Annotation::new_id(42, 4);
        assert!(id.starts_with("urn:openrfd:0042:"));
        // UUID portion is 36 chars (8-4-4-4-12)
        let uuid_part = id.strip_prefix("urn:openrfd:0042:").unwrap();
        assert_eq!(uuid_part.len(), 36);
    }

    #[test]
    fn test_new_id_custom_pad_width() {
        let id = Annotation::new_id(1, 6);
        assert!(id.starts_with("urn:openrfd:000001:"));
    }

    #[test]
    fn test_new_id_uniqueness() {
        let id1 = Annotation::new_id(1, 4);
        let id2 = Annotation::new_id(1, 4);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_is_reply() {
        let annotation = Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: "urn:openrfd:0001:test".into(),
            creator: Creator {
                creator_type: "Person".into(),
                name: "test".into(),
                email: None,
                url: None,
            },
            created: chrono::Utc::now(),
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "reply".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Reference("urn:openrfd:0001:parent".into()),
            modified: None,
            resolved: None,
            origin: None,
        };
        assert!(annotation.is_reply());
        assert_eq!(annotation.reply_to(), Some("urn:openrfd:0001:parent"));
    }

    #[test]
    fn test_not_reply() {
        let annotation = Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: "urn:openrfd:0001:test".into(),
            creator: Creator {
                creator_type: "Person".into(),
                name: "test".into(),
                email: None,
                url: None,
            },
            created: chrono::Utc::now(),
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "a comment".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: default_specific_resource(),
                source: "rfd/0001/README.md".into(),
                state: None,
                selector: vec![],
            }),
            modified: None,
            resolved: None,
            origin: None,
        };
        assert!(!annotation.is_reply());
        assert_eq!(annotation.reply_to(), None);
    }

    #[test]
    fn test_collection_to_bytes_from_bytes_roundtrip() {
        let mut collection = AnnotationCollection::new("Test Collection");
        collection.items.push(Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: "urn:openrfd:0001:abc".into(),
            creator: Creator {
                creator_type: "Person".into(),
                name: "alice".into(),
                email: Some("alice@example.com".into()),
                url: None,
            },
            created: chrono::Utc::now(),
            motivation: Motivation::Suggesting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "Consider rephrasing".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: default_specific_resource(),
                source: "rfd/0001/README.md".into(),
                state: Some(GitState {
                    state_type: "GitState".into(),
                    commit: "abc123".into(),
                    r#ref: Some("rfd/0001".into()),
                }),
                selector: vec![
                    Selector::TextQuoteSelector {
                        exact: "some text".into(),
                        prefix: Some("before ".into()),
                        suffix: Some(" after".into()),
                    },
                    Selector::TextPositionSelector { start: 10, end: 19 },
                ],
            }),
            modified: None,
            resolved: None,
            origin: None,
        });

        let bytes = collection.to_bytes().unwrap();
        let parsed = AnnotationCollection::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.label, "Test Collection");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].id, "urn:openrfd:0001:abc");
        assert_eq!(parsed.items[0].creator.name, "alice");
        assert_eq!(parsed.items[0].body.value, "Consider rephrasing");

        // Check selector roundtrip
        if let AnnotationTarget::Resource(ref res) = parsed.items[0].target {
            assert_eq!(res.source, "rfd/0001/README.md");
            assert_eq!(res.selector.len(), 2);
            if let Selector::TextQuoteSelector { exact, prefix, suffix } = &res.selector[0] {
                assert_eq!(exact, "some text");
                assert_eq!(prefix.as_deref(), Some("before "));
                assert_eq!(suffix.as_deref(), Some(" after"));
            } else {
                panic!("expected TextQuoteSelector");
            }
        } else {
            panic!("expected Resource target");
        }
    }

    #[test]
    fn test_collection_new_has_generator() {
        let collection = AnnotationCollection::new("My Annotations");
        assert_eq!(collection.label, "My Annotations");
        assert!(collection.items.is_empty());
        assert!(collection.generator.is_some());
        assert_eq!(collection.generator.unwrap().name, "openrfd");
    }

    #[test]
    fn test_anchor_health_display() {
        assert_eq!(AnchorHealth::Live.to_string(), "live");
        assert_eq!(AnchorHealth::Approximate.to_string(), "approximate");
        assert_eq!(AnchorHealth::Stale.to_string(), "stale");
        assert_eq!(AnchorHealth::Orphaned.to_string(), "orphaned");
    }

    #[test]
    fn test_github_id_format() {
        let id = Annotation::github_id(42, 4, 987654);
        assert_eq!(id, "urn:openrfd:0042:gh-987654");
    }

    #[test]
    fn test_github_id_stable() {
        let id1 = Annotation::github_id(42, 4, 100);
        let id2 = Annotation::github_id(42, 4, 100);
        assert_eq!(id1, id2, "same comment ID should produce same annotation ID");
    }

    #[test]
    fn test_github_id_distinct() {
        let id1 = Annotation::github_id(42, 4, 100);
        let id2 = Annotation::github_id(42, 4, 101);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_is_from_github() {
        let ann = Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: "urn:openrfd:0042:gh-100".into(),
            creator: Creator {
                creator_type: "Person".into(),
                name: "alice".into(),
                email: None,
                url: None,
            },
            created: chrono::Utc::now(),
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "test".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: default_specific_resource(),
                source: "rfd/0042/README.md".into(),
                state: None,
                selector: vec![],
            }),
            modified: None,
            resolved: None,
            origin: Some("github:100".into()),
        };
        assert!(ann.is_from_github());
    }

    #[test]
    fn test_not_from_github() {
        let ann = Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: Annotation::new_id(1, 4),
            creator: Creator {
                creator_type: "Person".into(),
                name: "bob".into(),
                email: None,
                url: None,
            },
            created: chrono::Utc::now(),
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "local".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: default_specific_resource(),
                source: "rfd/0001/README.md".into(),
                state: None,
                selector: vec![],
            }),
            modified: None,
            resolved: None,
            origin: None,
        };
        assert!(!ann.is_from_github());
    }

    #[test]
    fn test_modified_roundtrip() {
        let now = chrono::Utc::now();
        let mut collection = AnnotationCollection::new("Test Modified");
        collection.items.push(Annotation {
            context: default_context(),
            annotation_type: default_annotation_type(),
            id: "urn:openrfd:0042:gh-200".into(),
            creator: Creator {
                creator_type: "Person".into(),
                name: "alice".into(),
                email: None,
                url: None,
            },
            created: now,
            motivation: Motivation::Commenting,
            body: AnnotationBody {
                body_type: default_textual_body(),
                value: "edited comment".into(),
                format: default_format(),
            },
            target: AnnotationTarget::Resource(SpecificResource {
                resource_type: default_specific_resource(),
                source: "rfd/0042/README.md".into(),
                state: None,
                selector: vec![],
            }),
            modified: Some(now),
            resolved: None,
            origin: Some("github:200".into()),
        });

        let bytes = collection.to_bytes().unwrap();
        let parsed = AnnotationCollection::from_bytes(&bytes).unwrap();

        assert!(parsed.items[0].modified.is_some());
        assert_eq!(parsed.items[0].origin.as_deref(), Some("github:200"));
    }
}
