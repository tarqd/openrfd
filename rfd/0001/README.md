---
authors: OpenRFD Contributors
state: published
discussion:
---

# RFD 0001 Requests for Discussion

## Introduction

This RFD describes the RFD (Request for Discussion) process itself — the
mechanism by which ideas, proposals, and decisions are documented, discussed,
and resolved within this repository. It is the foundational document that
establishes how all subsequent RFDs operate.

## Background

Organizations struggle with decision-making that is transparent, inclusive, and
durable. Common failure modes include:

- **Decisions made in hallway conversations** that are never recorded and cannot
  be referenced later.
- **Knowledge trapped in individuals' heads**, making onboarding difficult and
  creating single points of failure.
- **Lack of written reasoning**, so future team members can't understand *why*
  a decision was made — only *what* was decided.
- **Inconsistent processes** where some decisions go through formal review and
  others don't.

The RFD process addresses these problems by making **written reasoning** the
center of how we think and decide together.

This process is inspired by [Oxide Computer's RFD system][oxide-rfd], which
itself draws from IETF RFCs, Golang proposals, Joyent RFDs, Rust RFCs, and
Kubernetes Enhancement Proposals.

## The RFD Process

### When to Write an RFD

You should write an RFD for anything that would benefit from written discussion
and a durable record of reasoning. This includes but is not limited to:

- Architectural or design decisions
- New features or significant changes
- API designs or modifications
- Process or workflow changes
- Tooling decisions
- Standards or conventions
- Post-mortems or retrospectives

The bar for creating an RFD is intentionally low. Notes are encouraged to be
**timely rather than polished**. A rough idea written down today is more
valuable than a perfect document that never gets written.

### RFD Lifecycle

Each RFD progresses through a series of states:

| State | Description |
|---|---|
| `prediscussion` | Initial placeholder. The author is actively iterating on the content, possibly on a branch. Not yet ready for broad feedback. |
| `ideation` | A topic has been outlined. The author may not be actively revising, but the idea is captured for future development. |
| `discussion` | The RFD is ready for feedback. A pull request is open and the community is invited to comment. |
| `published` | Discussion has converged. The RFD is merged to the main branch. It remains open to future updates if needed. |
| `committed` | The proposal has been fully implemented. The RFD serves as a historical record. |
| `abandoned` | The proposal was deliberately not pursued. The reasoning is preserved for posterity. |

State transitions are not strictly linear. An RFD might move from `discussion`
back to `prediscussion` if significant rework is needed. An `ideation` RFD
might jump straight to `discussion` once it's fleshed out. The states are guides,
not gatekeepers.

### Creating a New RFD

1. **Reserve a number.** RFDs are numbered sequentially (0001, 0002, ...). Use
   `rfd new "Your Title"` to automatically reserve the next number.

2. **Create a branch.** Each RFD gets its own Git branch named `rfd/NNNN`. The
   CLI tool creates this automatically.

3. **Write your RFD.** Use the provided template. Focus on:
   - A clear problem statement
   - Background and context
   - Your proposed approach
   - Alternatives you considered
   - Open questions

4. **Iterate locally.** Commit to your branch as you develop the idea. The RFD
   starts in `prediscussion` state.

5. **Open for discussion.** When ready, use `rfd discuss NNNN` to open a pull
   request. This moves the state to `discussion`.

6. **Gather feedback.** Discussion happens on the pull request. Line-by-line
   comments provide focused, attributable feedback that becomes part of the
   permanent record.

7. **Publish.** When discussion has converged, use `rfd publish NNNN` to merge
   the RFD to the main branch with state `published`.

### RFD Format

RFDs are Markdown files with YAML frontmatter stored in numbered directories:

```
rfd/
  0001/
    README.md
  0002/
    README.md
```

Each RFD file contains:

```markdown
---
authors: Jane Doe <jane@example.com>, John Smith <john@example.com>
state: prediscussion
discussion: https://github.com/org/repo/pull/42
---

# RFD 0002 Title of the Proposal

## Introduction
...
```

**Required metadata:**

- `authors` — Who wrote this RFD (name and optional email)
- `state` — Current lifecycle state

**Optional metadata:**

- `discussion` — Link to the pull request for this RFD

### Discussion Protocol

- Discussion happens on the **pull request**, not in issues or external tools.
- Line-by-line comments are preferred for specific feedback.
- General comments on the PR are for broader feedback.
- The pull request serves as the permanent discussion record.
- Authors should respond to all substantive feedback, even if just to
  acknowledge it.

### After Publication

Published RFDs are not frozen. They can and should be updated when:

- Implementation reveals that the original proposal needs adjustment
- New information changes the calculus
- Clarification is needed based on questions that arise

Updates to published RFDs go through the normal PR process.

## Principles

1. **Write things down.** If it's worth discussing, it's worth writing down.
2. **Timeliness over polish.** A rough draft today beats a perfect document
   never written.
3. **Reasoning over conclusions.** Document *why*, not just *what*.
4. **Persistent discussion.** Keep conversation attached to the document so
   future readers understand the context.
5. **Low barrier to entry.** Anyone can write an RFD. The process should feel
   lightweight, not burdensome.
6. **Transparency.** RFDs create a shared, searchable record of how we think
   and decide.

## References

- [Oxide Computer: RFD 1 — Requests for Discussion][oxide-rfd]
- [Oxide Computer: A Tool for Discussion][oxide-tool]
- [IETF RFC Process](https://www.ietf.org/standards/rfcs/)
- [Rust RFC Process](https://github.com/rust-lang/rfcs)
- [Golang Proposal Process](https://github.com/golang/proposal)

[oxide-rfd]: https://oxide.computer/blog/rfd-1-requests-for-discussion
[oxide-tool]: https://oxide.computer/blog/a-tool-for-discussion
