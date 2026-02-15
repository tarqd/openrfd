# Annotation System & Source Mapping

> **Part of the [OpenRFD PRD](prd.md).**
> See also: [CLI, Site & Implementation](prd-cli-and-site.md)

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
