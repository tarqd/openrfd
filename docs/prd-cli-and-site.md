# CLI, Static Site & Implementation

> **Part of the [OpenRFD PRD](prd.md).**
> See also: [Annotation System](prd-annotations.md)

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

## Web Frontend

The static site becomes interactive when a user signs in with GitHub. Without
login, the site is a read-only rendered view of all RFDs and their committed
annotations. With login, users can browse live PR discussions, add annotations,
and propose edits — all from the browser, all backed by the GitHub API. No
additional server infrastructure is required.

### GitHub OAuth (Client-Side)

The site uses the GitHub OAuth [device flow][device-flow] or a GitHub App
installation to authenticate users directly from the browser. The OAuth app is
configured in `.rfdconfig`:

```toml
[github]
app_client_id = "Iv1.abc123"    # GitHub OAuth App or GitHub App client ID
```

On login the frontend receives a token scoped to the repository. All subsequent
API calls (reading PRs, creating commits, posting reviews) use this token
client-side via the GitHub REST/GraphQL API.

[device-flow]: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow

### Branch Naming Convention

Each RFD is edited on exactly one branch at a time. The canonical branch name
is:

```
rfd/NNNN
```

Examples: `rfd/0001`, `rfd/0042`, `rfd/0100`.

This convention is enforced by the CLI (`rfd new`, `rfd edit`, `rfd discuss`)
and assumed by the web frontend. It provides a predictable mapping between RFD
number and branch/PR, enabling the frontend to:

- Look up the active PR for any RFD by branch name
- Determine whether the current user has an in-progress edit
- Link directly to the correct branch for annotation commits

A branch `rfd/NNNN` can represent either a new RFD or a revision to an
existing published one. Only one open PR per RFD is allowed at a time.

### Authenticated Features

#### Live PR Discussion

When viewing an RFD in `discussion` state, the frontend fetches the associated
PR's review comments via the GitHub API and renders them alongside the committed
annotations. This gives readers a unified view:

- **Committed annotations** — loaded from the static build (always available)
- **PR review comments** — fetched live from GitHub (requires login)

PR comments appear in the same annotation sidebar, visually distinguished as
"from PR review." They are not yet persisted as W3C annotations — that happens
at merge time via `rfd import-annotations`.

#### Active Revision Indicator

For any RFD, the frontend checks whether a branch `rfd/NNNN` exists with an
open PR. If so, the RFD page shows a banner:

```
┌─────────────────────────────────────────────────────────┐
│  This RFD has an active revision: PR #63 by @jane       │
│  [View changes]  [View on GitHub]                       │
└─────────────────────────────────────────────────────────┘
```

This makes it visible that discussion is happening, even when reading the
published (main branch) version.

#### Adding Annotations

Authenticated users can select text and add annotations directly from the
browser. The frontend:

1. Builds the W3C annotation object (selectors, GitState, creator from GitHub
   identity)
2. Commits the annotation JSON file to the `rfd/NNNN` branch via the GitHub
   Contents API (or creates the branch if one doesn't exist)
3. If no PR exists for that branch, offers to open one

This means annotations added from the web are immediately visible to other
authenticated users and will be included when the PR is merged.

#### Editing an RFD

Authenticated users can propose edits to an RFD from the browser:

- If the user already has branch `rfd/NNNN` with an open PR: edits commit to
  that branch
- If no branch exists: the frontend forks a new `rfd/NNNN` branch from `main`,
  commits the edit, and offers to open a PR

The edit UI is intentionally minimal — a textarea with the raw markdown, not a
WYSIWYG editor. This keeps the implementation simple and the markdown source
canonical.

### Unauthenticated Experience

Without login, the site is fully functional as a read-only resource:

- Browse and search all RFDs
- Read committed annotations
- View the annotation sidebar with threading
- See that a revision is in progress (the banner links to the PR on GitHub)

The login prompt appears only when a user tries to annotate, edit, or view live
PR comments.

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
- **RFD dependency graph** — Visualize relationships between RFDs.
- **RSS/Atom feed** — Subscribe to new and updated RFDs.
- **PDF export** — For offline reading and formal distribution.
- **Inter-repo RFD references** — Link to RFDs in other repositories.
- **Annotation permissions** — Control who can annotate (public RFDs may want
  moderated comments).

---

## Open Questions

1. **Annotation conflict resolution** — When two people annotate the same
   text concurrently on different branches, the JSON files are in separate
   directories and rarely conflict. But annotation IDs must be globally unique
   (UUIDs solve this).

2. **Large RFD repositories** — At hundreds or thousands of RFDs, should we
   shard the search index? Pre-build per-label or per-state indices?
