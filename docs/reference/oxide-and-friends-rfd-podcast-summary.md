# Oxide and Friends: RFDs — The Backbone of Oxide

> Source: Podcast transcript summary from "Oxide and Friends" episode

## What RFDs Are

RFDs = Requests for Discussion. They're the core mechanism Oxide uses to
document ideas, explore alternatives, drive decisions, and share reasoning on
any subject that matters to the company, from technical architecture to hiring,
process, measurement, culture, tooling, and more. They're not merely design
docs — they're the shared locus of collective thinking and discussion.

Key characteristics:

- They encapsulate an idea, proposal, question, or problem in writing.
- They are designed to invite others to read, reflect, and respond.
- They become the durable source of truth for why a decision was made and how
  thinking evolved.
- They're vastly more than a decision record; they cultivate a writing-centric
  culture.

## Core Purpose

### RFDs Make Thinking Visible

Oxide uses RFDs to externalize reasoning — not just conclusions. Writing things
down forces clarity of thought and catches assumptions early in the process.
This dramatically improves idea quality and surfacing of edge cases that matter.

### RFDs Support Collective Collaboration

Instead of private decisions or hallway conversations, RFDs invite open,
asynchronous discussion that others can reference, challenge, and build on. They
transform decision making from individual assertions into collective exploration.

### RFDs Provide Historical Memory

Because they're stored in a repository with rich metadata and full text history,
RFDs serve as a living archive. Even older RFDs gain value over time as context
for decisions — sometimes years after they were created.

### RFDs Enable Transparency

Making RFDs public (one by one if necessary) allows outsiders to understand the
company's technical and cultural thinking. This amplifies external trust,
transparency, and shared understanding of motivations.

## How RFDs Work in Practice

### Basic Structure

An RFD typically includes:

- A clear problem statement or question
- Background on why it matters
- Proposed approaches or options
- Reasoning behind choices
- Open space for discussion and feedback

While the format was inspired by IETF RFCs, Oxide intentionally didn't call them
RFCs to avoid confusion and to differentiate them from the more formal standards
process.

### Cultural Context

Writing-intensive culture: Oxide treats writing as central to engineering
practice — not secondary to coding. RFDs incentivize people to think through
writing rather than rely on intuition or verbal arguments. That leads to improved
communication, better onboarding, and shared corporate memory.

### Iteration and Quality

RFDs don't need to be perfect. They can be:

- Draft ideas
- Non-polished proposals
- Exploratory reasoning
- Incomplete sketches of a solution

The emphasis is on timeliness and clarity, not polish. This encourages people to
write early and iterate, rather than waiting for a perfect product.

## Technical Process and Workflow

### Repository + GitHub

Oxide uses a version-controlled repository, where each RFD is a file (initially
Markdown, moving toward AsciiDoc). This allows:

- Version history of idea evolution
- Line-by-line comments
- Branching and GitHub pull requests for discussion
- State management (ideation → discussion → published)

Using Git and GitHub provides mechanical scaffolding that other tools (e.g.,
Google Docs) can't easily replicate, because discussions, diffs, and history are
explicit.

### Discussion Through Pull Requests

Rather than issues or external tools, RFDs are reviewed and discussed via GitHub
pull requests, allowing:

- Focused line-by-line commentary
- Attribution and discussion history
- Traceability over time

This trade-off sacrifices some real-time chat ease, but rewards persistent
asynchronous discussion.

### States and Life Cycle

RFDs can exist in different states:

1. **Ideation** — exploratory rough draft
2. **Discussion** — active community feedback
3. **Published** — considered settled or canonical

These states help teams manage where a thought is in its lifecycle. Flexibility
is key — not every RFD must "close."

### Conflict Resolution Through RFDs

RFDs aren't just for consensus — they also manage organizational conflict. They
provide a written record of arguments and reasoning, which:

- Helps spotlight unresolved tensions
- Encourages people to state assumptions explicitly
- Reduces reliance on informal verbal sparring
- Prevents decisions from being lost in memory or hierarchy

As one speaker joked: "Let organizational conflict be in writing."

## Additional Uses & Innovations

### Hiring and Onboarding

Oxide uses RFDs as part of the interview/onboarding process. Candidates are
given access to RFDs so they can:

- Understand how the company thinks
- See real technical depth
- Form an informed view on whether they want to join

This doubles as screening and cultural orientation.

### Graphing Dependencies

Teams have visualized interlocks among RFDs to show dependencies and help orient
contributors. This helps with cognitive mapping of idea networks.

## Foundational Principles for Your Own RFD Process

If you're building your own RFD pipeline or policy, these distilled principles
are essential:

1. **Make writing the center of thinking** — Encourage rapid written expression;
   don't wait for perfection.
2. **Use version control as the backbone** — Store all proposals in a repo-like
   system so history, discussion, and evolution are explicit.
3. **Prioritize persistent discussion** — Keep conversation attached to the
   document so future readers can learn why decisions were made.
4. **Document assumptions and reasoning** — Treat RFDs as thought artifacts, not
   just specs or decisions.
5. **Open when appropriate** — If external transparency is a value, RFDs are a
   powerful public signal of how you think and operate.
6. **Use lightweight phases** — Ideation → discussion → published is an
   effective life cycle — but remain adaptable.

## Final Takeaways

- RFDs are fundamentally about shared, written reasoning — not rigid governance.
- Their value compounds over time as an organizational memory.
- The social, cultural effects — transparency, consensus building, conflict
  resolution — are as important as technical documentation.
