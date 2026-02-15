# OpenRFD

An open source implementation of the [Request for Discussion][oxide-rfd] (RFD)
process, inspired by [Oxide Computer Company][oxide].

RFDs are a lightweight mechanism for proposing ideas, documenting decisions, and
having structured, asynchronous discussions — all backed by Git and GitHub pull
requests.

## Quick Start

```bash
# Clone this repo (or use it as a template)
git clone https://github.com/yourorg/openrfd.git
cd openrfd

# Create your first RFD
./tools/rfd.sh new "Add caching layer to API"

# Edit the generated file
$EDITOR rfd/0002/README.md

# Open a pull request for discussion
./tools/rfd.sh discuss 2
```

## Installation

### Option 1: Use as a GitHub template

Click "Use this template" on GitHub to create a new repository with the full
RFD infrastructure pre-configured.

### Option 2: Add to an existing repo

```bash
# Copy the tools and templates into your project
cp -r tools/ templates/ .rfdconfig /path/to/your/repo/
mkdir -p /path/to/your/repo/rfd
```

### Option 3: Symlink the CLI

```bash
# Make rfd available globally
ln -s "$(pwd)/tools/rfd.sh" /usr/local/bin/rfd
```

## CLI Reference

```
rfd <command> [arguments]
```

| Command | Description |
|---|---|
| `new <title>` | Create a new RFD (reserves number, creates branch, scaffolds from template) |
| `list [--state ST]` | List all RFDs, optionally filtered by state |
| `show <number>` | Display an RFD's contents |
| `edit <number>` | Open an RFD in your editor |
| `state <num> <state>` | Update an RFD's lifecycle state |
| `discuss <number>` | Push branch and open a GitHub pull request |
| `publish <number>` | Mark as published and merge to main |
| `search <query>` | Full-text search across all RFDs |
| `validate [number]` | Validate RFD format and metadata |
| `index` | Regenerate the RFD index CSV |
| `help` | Show help |

## Workflow

The typical RFD lifecycle:

```
                    ┌──────────────┐
                    │prediscussion │  Author iterating on branch
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  ideation    │  Idea captured, not actively revised
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  discussion  │  PR open, community feedback
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼───────┐   │     ┌──────▼───────┐
       │  published   │   │     │  abandoned   │
       └──────┬───────┘   │     └──────────────┘
              │            │
       ┌──────▼───────┐   │
       │  committed   │   │
       └──────────────┘   │
                          │
                   (back to prediscussion
                    if rework needed)
```

### Step by Step

1. **Create** — `rfd new "My Proposal"` reserves a number, creates a branch
   `rfd/NNNN`, and scaffolds the RFD from a template.

2. **Write** — Edit `rfd/NNNN/README.md`. Focus on the problem, your proposal,
   alternatives you considered, and open questions.

3. **Iterate** — Commit to your branch. The RFD starts in `prediscussion`.

4. **Discuss** — `rfd discuss NNNN` opens a pull request and sets the state to
   `discussion`. All feedback happens on the PR.

5. **Publish** — `rfd publish NNNN` merges the branch and sets the state to
   `published`.

6. **Update** — Published RFDs can still be updated via new PRs.

## RFD Format

Each RFD lives in its own numbered directory:

```
rfd/
├── 0001/
│   └── README.md
├── 0002/
│   └── README.md
└── ...
```

RFDs use Markdown (or AsciiDoc) with YAML frontmatter:

```markdown
---
authors: Jane Doe <jane@example.com>
state: prediscussion
discussion:
---

# RFD 0002 Add Caching Layer to API

## Introduction
...
```

### Metadata Fields

| Field | Required | Description |
|---|---|---|
| `authors` | Yes | Author name(s) and optional email(s) |
| `state` | Yes | Current lifecycle state |
| `discussion` | No | URL to the pull request |

### States

| State | Meaning |
|---|---|
| `prediscussion` | Placeholder, author is actively iterating |
| `ideation` | Idea outlined, not actively being revised |
| `discussion` | PR open, community is reviewing |
| `published` | Discussion converged, merged to main |
| `committed` | Fully implemented and stable |
| `abandoned` | Deliberately not pursued |

## Configuration

Repository-level settings live in `.rfdconfig`:

```bash
# Default format for new RFDs: md or adoc
RFD_DEFAULT_FORMAT=md

# Main branch name
RFD_MAIN_BRANCH=main

# Zero-pad width for RFD numbers
RFD_PAD_WIDTH=4
```

The `EDITOR` environment variable controls which editor `rfd edit` opens.

## CI / GitHub Actions

The included GitHub Actions workflow (`.github/workflows/rfd-validate.yml`):

- **Validates** all RFDs on push and PR (checks frontmatter, state values,
  directory naming)
- **Regenerates** `rfd.csv` index on push to main

## Repository Structure

```
openrfd/
├── rfd/                    # All RFDs, one directory per RFD
│   └── 0001/
│       └── README.md
├── tools/
│   └── rfd.sh              # CLI tool
├── templates/
│   ├── rfd.md              # Markdown template
│   └── rfd.adoc            # AsciiDoc template
├── docs/
│   └── reference/          # Reference materials
├── .github/
│   └── workflows/
│       └── rfd-validate.yml
├── .rfdconfig              # Repository configuration
├── rfd.csv                 # Auto-generated index
└── README.md
```

## Why RFDs?

- **Shared reasoning** — Document *why* decisions were made, not just what.
- **Asynchronous collaboration** — Anyone can read, reflect, and respond on
  their own time.
- **Historical memory** — RFDs compound in value as a searchable archive of
  organizational thinking.
- **Low barrier** — Rough ideas are welcome. Timeliness over polish.
- **Transparency** — Visible decision-making builds trust internally and
  externally.

For more on the philosophy, see [RFD 0001](rfd/0001/README.md) and the
[reference materials](docs/reference/).

## Credits

Inspired by [Oxide Computer Company's RFD process][oxide-rfd] and their
[RFD tooling][oxide-tool].

## License

MIT

[oxide]: https://oxide.computer
[oxide-rfd]: https://oxide.computer/blog/rfd-1-requests-for-discussion
[oxide-tool]: https://oxide.computer/blog/a-tool-for-discussion
