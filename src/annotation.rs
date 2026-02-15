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
    /// If this annotation is resolved/closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<DateTime<Utc>>,
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
        let collection: Self = serde_json::from_slice(data)?;
        Ok(collection)
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
