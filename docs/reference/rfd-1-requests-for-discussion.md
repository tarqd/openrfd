# RFD 1: Requests for Discussion

> Source: https://oxide.computer/blog/rfd-1-requests-for-discussion

## Overview

Oxide Computer's RFD (Request for Discussion) system is a formal process for
proposing and discussing ideas, inspired by IETF's RFC model. The system
emphasizes that "notes are encouraged to be timely rather than polished,"
allowing rough ideas to be shared and refined through community feedback.

## When to Use RFDs

RFDs apply broadly to:

- Company process changes
- Architectural and design decisions (hardware/software)
- API or tool modifications affecting customers or internal teams
- Testing designs
- Any organizational improvements

## RFD States (Lifecycle)

1. **Prediscussion** - Placeholder stage with active iteration
2. **Ideation** - Topic outline without expectation of immediate revision
3. **Discussion** - Active feedback phase via pull request
4. **Published** - Discussion converged; merged to master (remains updatable)
5. **Committed** - Fully implemented and stable
6. **Abandoned** - Deliberately not pursued

## RFD Metadata

Each RFD includes:

- **Authors** - Listed with names and email addresses
- **State** - Current lifecycle position
- **Discussion** - Link to PR for ongoing conversation

## Process Workflow

1. Reserve sequential numbering (with leading zeros)
2. Create git branch named after RFD number
3. Use provided templates (Markdown or AsciiDoc)
4. Iterate locally, then push branch
5. Open pull request for community discussion
6. Merge and update state to "published"
7. Continue accepting changes post-publication

## Supporting Infrastructure

- Automated CSV tracking of all RFDs
- Short URL access (e.g., 12.rfd.oxide.computer)
- Chat bot integration for quick reference
- Public rendering site for sharing with external parties
