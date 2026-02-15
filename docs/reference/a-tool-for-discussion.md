# A Tool for Discussion: Oxide's RFD Platform

> Source: https://oxide.computer/blog/a-tool-for-discussion

## Overview

Oxide uses RFDs (Requests for Discussion) as a central mechanism for
architectural and design decisions. The company developed an internal RFD site
to improve upon storing these documents in a GitHub repository.

## Core Philosophy

"Similar to RFCs, our philosophy of RFDs is to allow both timely discussion of
rough ideas, while still becoming a permanent repository for more established
ones."

The RFD workflow draws inspiration from Golang proposals, Joyent RFDs, Rust
RFCs, and Kubernetes processes.

## Technical Implementation

RFDs are AsciiDoc documents stored in GitHub. The internal platform addresses
repository limitations:

- Poor reading experience on GitHub
- Limited AsciiDoc rendering support
- Difficulty sharing externally

## Key Features

**RFD Directory**: Users browse documents sorted by last update, showing
actively worked-on RFDs.

**Full-Text Search**: A self-hosted Meilisearch instance powers search
functionality, automatically updating indices. Users access it via navigation
menu or CMD+K keyboard shortcut.

**Inline PR Discussion**: The system fetches GitHub pull request comments using
the API and displays them alongside relevant document sections using
`getLineNumber` functionality. Comments load asynchronously via Remix deferred
responses to maintain page performance.

**Inter-RFD Linking**: Hovering over RFD references displays previews including
title, authors, status, and last update date.

**Jump-to Menu**: CMD+/ opens a keyboard-navigable menu for filtering and
selecting RFDs by title or number.

## Future Directions

Planned improvements include better discoverability through filtering and
tagging, document collections for onboarding, and a web-based editor eliminating
the need for Git repository cloning and command-line operations. The team aims to
make RFD creation accessible beyond engineering roles.
