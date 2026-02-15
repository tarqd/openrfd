# OpenRFD — Product Requirements Document

## Overview

OpenRFD is an open source system for managing Requests for Discussion (RFDs) —
a structured process for proposing ideas, documenting decisions, and having
persistent, asynchronous discussions backed by Git.

It is inspired by [Oxide Computer Company's RFD process][oxide-rfd] and is
designed to be adopted by any organization that values written reasoning,
transparent decision-making, and durable institutional memory.

OpenRFD is a single Rust binary that serves as both the CLI management tool and
the static site generator.

[oxide-rfd]: https://oxide.computer/blog/rfd-1-requests-for-discussion

---

## Goals

1. **Make written reasoning the default** — Lower the barrier to writing down
   ideas, proposals, and decisions so teams default to written discussion over
   hallway conversations.

2. **Durable, portable discussion** — Comments and annotations travel with the
   document in the repository. Fork the repo, you get the full discussion
   history. No external service required.

3. **Standards-based annotation** — Use the W3C Web Annotation data model for
   all comments. Interoperable with existing tools, extensible, and future-proof.

4. **One binary, zero infrastructure** — A single `openrfd` binary handles
   everything: creating RFDs, managing lifecycle, building a static site,
   managing annotations. Deploys to GitHub Pages with no additional services.

5. **Editor-native workflow** — Authors work in their editor of choice. The
   annotation format supports editor plugins for inline display and creation of
   comments without leaving the writing flow.

---

## Non-Goals

- Real-time collaborative editing (Google Docs-style)
- Replacing GitHub Issues or project management tools
- Supporting non-Git version control systems (initially)
- Server-side rendering or dynamic backends
- WYSIWYG editing

---

## Architecture

### Single Binary

```
openrfd (Rust binary)
├── CLI: rfd new / list / show / edit / state / discuss / publish
├── Annotations: rfd annotate / annotations / import-annotations / resolve
├── Site Generator: rfd build / serve
└── Validation: rfd validate / check-annotations
```

### Repository Structure

```
project/
├── rfd/
│   ├── 0001/
│   │   ├── README.md              # The RFD document
│   │   └── annotations/
│   │       ├── pr-47.json         # Imported from GitHub PR
│   │       ├── review-2026-02.json
│   │       └── slack-thread.json
│   ├── 0002/
│   │   ├── README.md
│   │   └── annotations/
│   └── ...
├── templates/
│   ├── rfd.md                     # New RFD template
│   ├── base.html                  # Site layout
│   ├── index.html                 # RFD listing page
│   └── rfd.html                   # Individual RFD page
├── static/
│   ├── style.css
│   └── rfd.js                     # Minimal vanilla JS
├── .rfdconfig                     # Repository configuration
├── .github/
│   └── workflows/
│       └── rfd.yml                # CI: validate + build + deploy
└── README.md
```

### Build Output

```
_site/
├── index.html                     # RFD listing
├── rfd/0001/index.html            # Rendered RFD with annotation sidebar
├── search-index.json              # Pre-built for client-side search
├── style.css
└── rfd.js
```

---

## RFD Document Format

### Markdown with YAML Frontmatter

```markdown
---
authors: Jane Doe <jane@example.com>, John Smith <john@example.com>
state: discussion
discussion: https://github.com/org/repo/pull/47
visibility: public
labels:
  - architecture
  - api
---

# RFD 0042 Service Mesh Design

## Introduction
...
```

### Frontmatter Fields

| Field        | Required | Type       | Description                              |
|------------- |--------- |----------- |----------------------------------------- |
| `authors`    | yes      | string     | Comma-separated names with optional email |
| `state`      | yes      | enum       | Lifecycle state (see below)              |
| `discussion` | no       | url        | Link to the pull request                 |
| `visibility` | no       | enum       | `public`, `internal`, `confidential` (default: `internal`) |
| `labels`     | no       | list       | Free-form tags for filtering             |

### AsciiDoc Support

AsciiDoc is supported as an alternative format. The renderer detects the file
extension (`.md` or `.adoc`) and uses the appropriate parser.

---

## Lifecycle States

| State            | Description                                                |
|----------------- |----------------------------------------------------------- |
| `prediscussion`  | Initial placeholder. Author is actively iterating on a branch. Not ready for broad feedback. |
| `ideation`       | Topic outlined. May not be actively revised, but the idea is captured. |
| `discussion`     | PR open. Community is invited to review and comment.       |
| `published`      | Discussion converged. Merged to main. Open to future updates. |
| `committed`      | Fully implemented. Serves as historical record.            |
| `abandoned`      | Deliberately not pursued. Reasoning preserved.             |

State transitions are not strictly linear. An RFD may move backward (e.g.,
`discussion` → `prediscussion` for rework) or skip states (e.g., `ideation` →
`discussion`).

---

## Annotation System

### W3C Web Annotation Data Model

All comments and annotations use the [W3C Web Annotation Data Model][w3c-anno].
This provides a standard format for anchoring commentary to documents with full
provenance.

[w3c-anno]: https://www.w3.org/TR/annotation-model/

### Annotation Format

```json
{
  "@context": "http://www.w3.org/ns/anno.jsonld",
  "type": "Annotation",
  "id": "urn:openrfd:0042:a1b2c3",
  "creator": {
    "type": "Person",
    "name": "Jane Doe",
    "email": "jane@example.com"
  },
  "created": "2026-02-15T14:30:00Z",
  "motivation": "commenting",
  "body": {
    "type": "TextualBody",
    "value": "Have we considered Connect? Better browser support.",
    "format": "text/markdown"
  },
  "target": {
    "type": "SpecificResource",
    "source": "rfd/0042/README.md",
    "state": {
      "type": "GitState",
      "commit": "a1b2c3d4e5f6",
      "ref": "rfd/0042"
    },
    "selector": [
      {
        "type": "TextQuoteSelector",
        "exact": "gRPC for the service mesh",
        "prefix": "We should use ",
        "suffix": ". The API will"
      },
      {
        "type": "FragmentSelector",
        "value": "line=42,42",
        "conformsTo": "http://tools.ietf.org/rfc/rfc5147"
      },
      {
        "type": "TextPositionSelector",
        "start": 1245,
        "end": 1270
      }
    ]
  }
}
```

### Selector Hierarchy

Multiple selectors are stored per annotation, each serving a different purpose:

| Selector              | Purpose                              | Resilience to edits |
|---------------------- |------------------------------------- |-------------------- |
| `TextQuoteSelector`   | Primary live matching. Finds the annotated text in the current document using quoted text with prefix/suffix context. | High — survives edits that don't change the quoted text. |
| `FragmentSelector`    | Source line numbers (RFC 5147). Fast jump target for editors. | Low — goes stale on any edit above the target line. Treated as an optimization hint. |
| `TextPositionSelector`| Character byte offsets. Precise for tooling. | Low — goes stale on any edit before the target. Also an optimization hint. |

### GitState

The `GitState` on the target records the exact commit the annotation was created
against. This serves as ground truth: if all selectors fail to match the current
document, the tooling can check out that commit to show exactly what the
annotation was referencing.

```json
{
  "type": "GitState",
  "commit": "a1b2c3d4e5f6",
  "ref": "rfd/0042"
}
```

`GitState` is a custom state type extending the W3C model. The `ref` field
records the branch the annotation was created on (useful for annotations made
during the `discussion` phase before merge).

### Annotation Resolution Algorithm

When placing an annotation on a document:

1. **TextQuoteSelector on current source** — Match found? Place it. This is the
   common case.
2. **Fuzzy match** — No exact match? Try fuzzy matching (Levenshtein distance,
   longest common substring) on the `exact` field. Close enough? Place it, flag
   as "approximate."
3. **Diff-based re-anchoring** — Still no match? Check out the source at the
   annotation's commit. Find the match there. Diff the old and new source.
   Track where that text moved or was deleted.
4. **Orphaned** — Text was deleted or changed beyond recognition. Mark the
   annotation as orphaned. Display it with the original quoted text and commit
   so a human can decide what to do.

### Annotation Storage

Annotations are stored in JSON files within each RFD's `annotations/` directory.
Each file is a W3C `AnnotationCollection`:

```json
{
  "@context": "http://www.w3.org/ns/anno.jsonld",
  "type": "AnnotationCollection",
  "label": "PR #47 Discussion",
  "generator": {
    "type": "Software",
    "name": "openrfd",
    "homepage": "https://github.com/openrfd/openrfd"
  },
  "items": [ ]
}
```

Annotations are grouped by source (one file per PR import, review session,
etc.) to keep diffs clean and allow selective inclusion.

### Annotation Threading

Annotations can reply to other annotations by targeting them:

```json
{
  "type": "Annotation",
  "motivation": "replying",
  "body": { "value": "Good point, I'll switch to Connect." },
  "target": "urn:openrfd:0042:a1b2c3"
}
```

The frontend renders these as threaded conversations in the sidebar.

---

## Source Mapping

### Why

When a user highlights text in the rendered web view and adds a comment, we need
to map the DOM selection back to the markdown source accurately. Source mapping
eliminates ambiguity (e.g., same phrase appearing multiple times).

### How

`pulldown-cmark` provides `into_offset_iter()`, yielding `(Event, Range<usize>)`
pairs — every AST node tagged with its byte range in the markdown source.

During `rfd build`, source byte ranges are emitted as `data-source` attributes
on block-level HTML elements:

```html
<h2 data-source="245-267">Proposal</h2>
<p data-source="269-342">
  We should use <strong>gRPC</strong> for the service mesh.
  The API will expose three endpoints.
</p>
```

### Annotation Creation from Web View

1. User selects text in the browser.
2. JavaScript gets the DOM Range.
3. Walk up to the nearest element with `data-source`.
4. Read the source byte range from the attribute.
5. Calculate text offset within the block's `textContent`.
6. Build the annotation with all three selector types plus the current commit.
7. POST to a small endpoint or write to a JSON file via GitHub API.

### Annotation Validation at Build Time

During `rfd build`, the renderer produces a `RenderedDocument` with a source map.
The annotation validator uses this to check every annotation:

- TextQuoteSelector matches current source → valid
- No match → attempt diff-based re-anchor against the annotation's commit
- Report stale, approximate, and orphaned annotations

---

## Visibility and Access Control

### Visibility Field

Each RFD declares its visibility in frontmatter:

```yaml
visibility: public       # visible on the public site
visibility: internal     # visible only on internal site (default)
visibility: confidential # excluded from all generated sites
```

### Build Filtering

```bash
rfd build                 # all non-confidential RFDs
rfd build --public        # only visibility: public
rfd build --internal      # only visibility: internal
```

### Public Mirror (Optional)

For organizations that need strong separation (no private content in git
history), the CLI supports publishing public RFDs to a separate repository:

```bash
rfd publish --mirror public-repo 42
```

This copies the RFD and its public annotations to the mirror repo. The mirror
never contains internal or confidential content, even in git history.

---

## CLI Reference

### RFD Management

| Command                      | Description                                          |
|----------------------------- |----------------------------------------------------- |
| `rfd init`                   | Initialize a new RFD repository                      |
| `rfd new <title>`            | Reserve number, create branch, scaffold from template |
| `rfd list [--state S] [--label L]` | List RFDs, optionally filtered                 |
| `rfd show <number>`          | Display an RFD                                       |
| `rfd edit <number>`          | Open in `$EDITOR`                                    |
| `rfd state <number> <state>` | Update lifecycle state                               |
| `rfd discuss <number>`       | Push branch, open GitHub PR, set state to discussion  |
| `rfd publish <number>`       | Merge to main, set state to published                |
| `rfd search <query>`         | Full-text search across all RFDs                     |
| `rfd validate [number]`      | Validate format, frontmatter, state                  |

### Annotations

| Command                              | Description                                |
|-------------------------------------- |------------------------------------------- |
| `rfd annotate <num> --quote "..." "comment"` | Add an annotation                  |
| `rfd annotations <num> [--source S]` | List annotations, optionally by source     |
| `rfd annotations <num> --check`      | Check annotation health (live/stale/orphaned) |
| `rfd annotations <num> --reanchor`   | Update stale position selectors            |
| `rfd import-annotations <num> --pr N`| Import GitHub PR comments as annotations   |
| `rfd resolve <num> <annotation-id>`  | Mark annotation as resolved                |

### Site Generation

| Command                       | Description                                    |
|------------------------------ |----------------------------------------------- |
| `rfd build [--public]`        | Generate static site                           |
| `rfd serve [--port N]`        | Local dev server with live reload              |

### Configuration

| Command           | Description                  |
|------------------ |----------------------------- |
| `rfd config`      | Show current configuration   |
| `rfd config set <key> <value>` | Update configuration |

---

## Static Site

### Technology

- **Template engine**: [Tera][tera] (Jinja2-like, Rust-native)
- **Markdown rendering**: [pulldown-cmark][pulldown] with source span tracking
- **Styling**: Single CSS file, no framework
- **Client-side JS**: Minimal vanilla JavaScript (~5KB) for:
  - Annotation sidebar toggle and highlight linking
  - Comment creation via text selection
  - Jump-to-RFD menu (Cmd+K)
  - Client-side search against pre-built JSON index
- **Search**: Pre-built JSON index at build time, [minisearch][minisearch]
  (~6KB) on the client

[tera]: https://keats.github.io/tera/
[pulldown]: https://github.com/raphlinus/pulldown-cmark
[minisearch]: https://lucaong.github.io/minisearch/

### Pages

| Page              | Description                                            |
|------------------ |------------------------------------------------------- |
| `index.html`      | Sortable, filterable table of all RFDs (number, title, state, authors, last updated, labels) |
| `rfd/NNNN/`       | Rendered RFD with annotation sidebar, metadata header, and inter-RFD link previews |

### RFD Page Layout

```
┌──────────────────────────────────────────────────────────────┐
│  RFD 0042 · Service Mesh Design                             │
│  State: discussion · Authors: Jane, Bob · PR: #47           │
├─────────────────────────────────────┬────────────────────────┤
│                                     │ Annotations (5)        │
│  ## Proposal                        │                        │
│                                     │ ┌────────────────────┐ │
│  We should use [gRPC] for the  ◄────│─│ Jane · Feb 15      │ │
│  service mesh.                      │ │ via PR #47         │ │
│                                     │ │                    │ │
│                                     │ │ Have we considered │ │
│                                     │ │ Connect?           │ │
│                                     │ └────────────────────┘ │
│  The API will expose three          │                        │
│  endpoints...                       │ ┌────────────────────┐ │
│                                     │ │ Bob · Feb 16       │ │
│                                     │─│ via Slack           │ │
│                                     │ │                    │ │
│                                     │ │ Do we need batch   │ │
│                                     │ │ at launch?         │ │
│                                     │ └────────────────────┘ │
│                                     │                        │
│                                     │ [+ Add annotation]     │
├─────────────────────────────────────┴────────────────────────┤
│  Generated by OpenRFD · a1b2c3d · 2026-02-15                │
└──────────────────────────────────────────────────────────────┘
```

### GitHub Pages Deployment

The included GitHub Actions workflow builds the site and deploys to GitHub Pages
on push to main:

```yaml
- rfd build --public
- deploy to gh-pages branch
```

---

## Rust Crate Structure

```
openrfd/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entrypoint (clap)
│   ├── config.rs            # .rfdconfig parsing
│   ├── rfd.rs               # RFD struct: frontmatter parsing, serialization
│   ├── state.rs             # Lifecycle state enum and transitions
│   ├── repo.rs              # Git operations (git2), branch/PR workflow
│   ├── render.rs            # Markdown → HTML with source mapping
│   ├── annotation.rs        # W3C Web Annotation types and serialization
│   ├── selector.rs          # TextQuoteSelector matching, fuzzy matching
│   ├── source_map.rs        # Source span tracking, RenderedDocument
│   ├── import.rs            # Import annotations from GitHub PRs, etc.
│   ├── build.rs             # Static site generation (Tera)
│   ├── search.rs            # Search index generation
│   ├── validate.rs          # RFD and annotation validation
│   └── index.rs             # CSV/JSON index generation
├── templates/
│   ├── base.html            # Tera: site layout
│   ├── index.html           # Tera: RFD listing
│   └── rfd.html             # Tera: individual RFD page
├── static/
│   ├── style.css
│   └── rfd.js
└── tests/
    ├── rfd_test.rs
    ├── selector_test.rs
    ├── render_test.rs
    └── annotation_test.rs
```

### Key Dependencies

| Crate          | Purpose                                    |
|--------------- |------------------------------------------- |
| `clap`         | CLI argument parsing                       |
| `serde`        | Serialization for YAML frontmatter, JSON annotations |
| `pulldown-cmark` | Markdown parsing with source spans       |
| `tera`         | HTML template rendering                    |
| `git2`         | Git operations (libgit2 bindings)          |
| `octocrab`     | GitHub API (PR comments import)            |
| `minijinja`    | Alternative to Tera if lighter weight needed |
| `axum`         | Local dev server for `rfd serve`           |
| `similar`      | Diff algorithm for annotation re-anchoring |

### Core Types

```rust
/// A parsed RFD document.
struct Rfd {
    number: u32,
    frontmatter: Frontmatter,
    body: String,            // raw markdown after frontmatter
    path: PathBuf,
}

struct Frontmatter {
    authors: String,
    state: State,
    discussion: Option<String>,
    visibility: Visibility,
    labels: Vec<String>,
}

enum State {
    Prediscussion,
    Ideation,
    Discussion,
    Published,
    Committed,
    Abandoned,
}

enum Visibility {
    Public,
    Internal,
    Confidential,
}

/// Rendered output with source mapping.
struct RenderedDocument {
    html: String,
    source_map: Vec<SourceSpan>,
}

struct SourceSpan {
    source_range: Range<usize>,  // byte range in markdown
    output_range: Range<usize>,  // byte range in HTML
    line_range: Range<u32>,      // source line numbers
}

/// W3C Web Annotation (simplified).
struct Annotation {
    id: String,
    creator: Creator,
    created: DateTime<Utc>,
    motivation: Motivation,
    body: AnnotationBody,
    target: AnnotationTarget,
}

struct AnnotationTarget {
    source: String,
    state: Option<GitState>,
    selector: Vec<Selector>,
}

struct GitState {
    commit: String,
    r#ref: Option<String>,
}

enum Selector {
    TextQuote { exact: String, prefix: Option<String>, suffix: Option<String> },
    Fragment { value: String, conforms_to: String },
    TextPosition { start: usize, end: usize },
}
```

---

## GitHub Actions

### Validation + Build + Deploy

```yaml
name: RFD

on:
  push:
    branches: [main]
    paths: [rfd/**, templates/**, static/**]
  pull_request:
    paths: [rfd/**]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: openrfd/setup-openrfd@v1   # install the binary
      - run: rfd validate
      - run: rfd annotations --check-all

  build-deploy:
    needs: validate
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      pages: write
      id-token: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }       # full history for GitState
      - uses: openrfd/setup-openrfd@v1
      - run: rfd build --public
      - uses: actions/deploy-pages@v4
        with: { path: _site }
```

---

## Future Work

Items explicitly out of scope for v1 but worth tracking:

- **VS Code extension** — Render annotations inline, create annotations from
  editor selections, annotation panel sidebar.
- **Neovim/Emacs plugins** — Same functionality via LSP or native plugin.
- **Slack/Discord import** — `rfd import-annotations --source slack --url "..."`.
- **Annotation API server** — Optional small server for creating annotations
  from the web view without going through GitHub API.
- **RFD dependency graph** — Visualize relationships between RFDs.
- **RSS/Atom feed** — Subscribe to new and updated RFDs.
- **PDF export** — For offline reading and formal distribution.
- **Inter-repo RFD references** — Link to RFDs in other repositories.
- **Annotation permissions** — Control who can annotate (public RFDs may want
  moderated comments).

---

## Open Questions

1. **AsciiDoc parity** — Should v1 support AsciiDoc fully, or ship with
   Markdown only and add AsciiDoc later? AsciiDoc parsing in Rust is less
   mature than Markdown.

2. **Web-based annotation creation** — For `rfd build --public` sites on
   GitHub Pages, there's no server to receive new annotations. Options:
   - GitHub API: create a commit adding the annotation JSON via the API
   - GitHub Actions: trigger a workflow that adds the annotation
   - External service: small API that creates PRs with annotation files
   - Local only: annotations created via CLI or editor, not the web view

3. **Annotation conflict resolution** — When two people annotate the same
   text concurrently on different branches, the JSON files are in separate
   directories and rarely conflict. But annotation IDs must be globally unique
   (UUIDs solve this).

4. **Large RFD repositories** — At hundreds or thousands of RFDs, should we
   shard the search index? Pre-build per-label or per-state indices?
