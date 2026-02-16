// The miette derive macros produce false-positive `unused_assignments`
// warnings when combined with manual Clone impls.
#![allow(unused_assignments)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

use crate::state::State;

// ---------------------------------------------------------------------------
// Frontmatter diagnostics
// ---------------------------------------------------------------------------

/// YAML frontmatter block is missing entirely.
#[derive(Error, Diagnostic, Debug)]
#[error("missing YAML frontmatter")]
#[diagnostic(
    code(rfd::frontmatter::missing),
    help("add frontmatter at the top of the file:\n\n  ---\n  authors: Your Name\n  state: prediscussion\n  ---")
)]
pub struct MissingFrontmatter {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("expected `---` here")]
    pub span: SourceSpan,
}

/// YAML inside frontmatter cannot be deserialized.
#[derive(Error, Diagnostic, Debug)]
#[error("invalid frontmatter")]
#[diagnostic(code(rfd::frontmatter::invalid))]
pub struct InvalidFrontmatter {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("{reason}")]
    pub span: SourceSpan,
    pub reason: String,
    #[help]
    pub advice: Option<String>,
}

// ---------------------------------------------------------------------------
// State / visibility diagnostics
// ---------------------------------------------------------------------------

/// Invalid lifecycle state value.
#[derive(Error, Diagnostic, Debug)]
#[error("invalid state `{value}`")]
#[diagnostic(code(rfd::state::invalid))]
pub struct InvalidState {
    pub value: String,
    #[help]
    pub advice: String,
}

impl InvalidState {
    pub fn new(value: &str) -> Self {
        let valid = State::ALL
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            value: value.to_string(),
            advice: format!("valid states are: {}", valid),
        }
    }
}

/// Invalid visibility value.
#[derive(Error, Diagnostic, Debug)]
#[error("invalid visibility `{value}`")]
#[diagnostic(code(rfd::visibility::invalid))]
pub struct InvalidVisibility {
    pub value: String,
    #[help]
    pub advice: String,
}

impl InvalidVisibility {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            advice: "valid visibilities are: public, internal, confidential".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation diagnostics
// ---------------------------------------------------------------------------

/// Annotation JSON is malformed.
#[derive(Error, Diagnostic, Debug)]
#[error("invalid annotation JSON")]
#[diagnostic(code(rfd::annotation::invalid_json))]
pub struct InvalidAnnotationJson {
    #[source_code]
    pub src: NamedSource<String>,
    #[label("{reason}")]
    pub span: SourceSpan,
    pub reason: String,
    #[help]
    pub advice: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation diagnostics
// ---------------------------------------------------------------------------

/// A validation finding (error or warning) with optional source context.
#[derive(Error, Diagnostic, Debug)]
#[error("{message}")]
pub struct ValidationDiagnostic {
    pub message: String,
    #[source_code]
    pub src: NamedSource<String>,
    #[label("{label}")]
    pub span: SourceSpan,
    pub label: String,
    #[help]
    pub advice: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk an `anyhow::Error` chain and try to render the first `miette::Diagnostic`
/// found. Returns `true` if a diagnostic was rendered, `false` otherwise.
pub fn try_render_diagnostic(err: &anyhow::Error) -> bool {
    // Check root — the outermost error itself
    if let Some(diag) = err.downcast_ref::<MissingFrontmatter>() {
        eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
        return true;
    }
    if let Some(diag) = err.downcast_ref::<InvalidFrontmatter>() {
        eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
        return true;
    }
    if let Some(diag) = err.downcast_ref::<InvalidState>() {
        eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
        return true;
    }
    if let Some(diag) = err.downcast_ref::<InvalidVisibility>() {
        eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
        return true;
    }
    if let Some(diag) = err.downcast_ref::<InvalidAnnotationJson>() {
        eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
        return true;
    }

    // Walk the chain of source errors
    for cause in err.chain().skip(1) {
        if let Some(diag) = cause.downcast_ref::<MissingFrontmatter>() {
            eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
            return true;
        }
        if let Some(diag) = cause.downcast_ref::<InvalidFrontmatter>() {
            eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
            return true;
        }
        if let Some(diag) = cause.downcast_ref::<InvalidState>() {
            eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
            return true;
        }
        if let Some(diag) = cause.downcast_ref::<InvalidVisibility>() {
            eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
            return true;
        }
    }

    false
}

// We need Clone for the downcast-and-rewrap pattern above.
impl Clone for MissingFrontmatter {
    fn clone(&self) -> Self {
        Self {
            src: NamedSource::new(self.src.name(), self.src.inner().clone()),
            span: self.span,
        }
    }
}

impl Clone for InvalidFrontmatter {
    fn clone(&self) -> Self {
        Self {
            src: NamedSource::new(self.src.name(), self.src.inner().clone()),
            span: self.span,
            reason: self.reason.clone(),
            advice: self.advice.clone(),
        }
    }
}

impl Clone for InvalidState {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            advice: self.advice.clone(),
        }
    }
}

impl Clone for InvalidVisibility {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            advice: self.advice.clone(),
        }
    }
}

impl Clone for InvalidAnnotationJson {
    fn clone(&self) -> Self {
        Self {
            src: NamedSource::new(self.src.name(), self.src.inner().clone()),
            span: self.span,
            reason: self.reason.clone(),
            advice: self.advice.clone(),
        }
    }
}

impl Clone for ValidationDiagnostic {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            src: NamedSource::new(self.src.name(), self.src.inner().clone()),
            span: self.span,
            label: self.label.clone(),
            advice: self.advice.clone(),
        }
    }
}
