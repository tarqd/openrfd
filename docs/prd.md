# OpenRFD — Product Requirements Document

> **See also:**
> [Annotation System](prd-annotations.md) ·
> [CLI, Site & Implementation](prd-cli-and-site.md)

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
