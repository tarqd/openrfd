#!/usr/bin/env bash
#
# rfd - A CLI tool for managing Requests for Discussion
#
# An open source implementation inspired by Oxide Computer's RFD process.
# See: https://oxide.computer/blog/rfd-1-requests-for-discussion
#
set -euo pipefail

VERSION="0.1.0"

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

# Resolve the repository root (the directory containing the rfd/ folder).
find_repo_root() {
    local dir="$PWD"
    while [[ "$dir" != "/" ]]; do
        if [[ -d "$dir/rfd" && -d "$dir/.git" ]]; then
            echo "$dir"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    return 1
}

REPO_ROOT=""
init_repo_root() {
    REPO_ROOT=$(find_repo_root) || {
        echo "error: not inside an RFD repository (no rfd/ directory found)" >&2
        exit 1
    }
}

RFD_DIR=""
TEMPLATE_DIR=""
CONFIG_FILE=""

load_config() {
    RFD_DIR="$REPO_ROOT/rfd"
    TEMPLATE_DIR="$REPO_ROOT/templates"
    CONFIG_FILE="$REPO_ROOT/.rfdconfig"

    # Source config file if present
    if [[ -f "$CONFIG_FILE" ]]; then
        # shellcheck source=/dev/null
        source "$CONFIG_FILE"
    fi
}

# Default settings (can be overridden in .rfdconfig)
: "${RFD_DEFAULT_FORMAT:=md}"          # md or adoc
: "${RFD_MAIN_BRANCH:=main}"
: "${RFD_EDITOR:=${EDITOR:-vi}}"
: "${RFD_PAD_WIDTH:=4}"               # zero-pad RFD numbers to this width

VALID_STATES=(prediscussion ideation discussion published committed abandoned)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

pad_number() {
    printf "%0${RFD_PAD_WIDTH}d" "$1"
}

is_valid_state() {
    local state="$1"
    for s in "${VALID_STATES[@]}"; do
        [[ "$s" == "$state" ]] && return 0
    done
    return 1
}

# Extract YAML frontmatter field from an RFD file.
get_field() {
    local file="$1" field="$2"
    sed -n '/^---$/,/^---$/p' "$file" | grep "^${field}:" | head -1 | sed "s/^${field}:[[:space:]]*//"
}

# Set a YAML frontmatter field in an RFD file.
set_field() {
    local file="$1" field="$2" value="$3"
    if grep -q "^${field}:" "$file" 2>/dev/null; then
        sed -i "s|^${field}:.*|${field}: ${value}|" "$file"
    else
        # Insert after first ---
        sed -i "0,/^---$/!{0,/^---$/s/^---$/${field}: ${value}\n---/}" "$file"
    fi
}

# Find the RFD file for a given number.
find_rfd_file() {
    local padded
    padded=$(pad_number "$1")
    local dir="$RFD_DIR/$padded"

    if [[ -f "$dir/README.md" ]]; then
        echo "$dir/README.md"
    elif [[ -f "$dir/README.adoc" ]]; then
        echo "$dir/README.adoc"
    else
        return 1
    fi
}

# Get the title from an RFD file.
get_title() {
    local file="$1"
    # Look for first markdown heading after frontmatter
    sed '1,/^---$/d' "$file" | sed '/^---$/d' | grep -m1 '^#' | sed 's/^#\+[[:space:]]*//'
}

# Get next available RFD number.
next_number() {
    local max=0
    if [[ -d "$RFD_DIR" ]]; then
        for d in "$RFD_DIR"/[0-9]*/; do
            [[ -d "$d" ]] || continue
            local num
            num=$(basename "$d" | sed 's/^0*//')
            num=${num:-0}
            if (( num > max )); then
                max=$num
            fi
        done
    fi
    echo $(( max + 1 ))
}

# Return the branch name for a given RFD number.
rfd_branch_name() {
    local padded
    padded=$(pad_number "$1")
    echo "rfd/$padded"
}

ensure_clean_working_tree() {
    if ! git -C "$REPO_ROOT" diff --quiet HEAD -- 2>/dev/null; then
        echo "warning: you have uncommitted changes" >&2
    fi
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

cmd_init() {
    if [[ -d "rfd" ]]; then
        echo "RFD repository already initialized in $PWD"
        return 0
    fi

    mkdir -p rfd templates

    # Create default config
    cat > .rfdconfig << 'CONF'
# OpenRFD Configuration
# See README.md for details.

# Default format for new RFDs: md or adoc
RFD_DEFAULT_FORMAT=md

# Main branch name (main or master)
RFD_MAIN_BRANCH=main

# Zero-pad width for RFD numbers
RFD_PAD_WIDTH=4
CONF

    # Create markdown template
    cat > templates/rfd.md << 'TMPL'
---
authors:
state: prediscussion
discussion:
---

# RFD {number} {title}

## Introduction

What is this RFD about? Briefly describe the problem or idea.

## Background

Why does this matter? What context does the reader need?

## Proposal

Describe your proposed approach, design, or solution.

## Alternatives

What other approaches were considered? Why were they not chosen?

## Open Questions

- What remains unresolved?

## References

- List any relevant links, prior art, or related RFDs.
TMPL

    # Create asciidoc template
    cat > templates/rfd.adoc << 'TMPL'
:authors:
:state: prediscussion
:discussion:

= RFD {number} {title}

== Introduction

What is this RFD about? Briefly describe the problem or idea.

== Background

Why does this matter? What context does the reader need?

== Proposal

Describe your proposed approach, design, or solution.

== Alternatives

What other approaches were considered? Why were they not chosen?

== Open Questions

* What remains unresolved?

== References

* List any relevant links, prior art, or related RFDs.
TMPL

    echo "Initialized RFD repository in $PWD"
    echo "  Created rfd/, templates/, and .rfdconfig"
    echo ""
    echo "Next steps:"
    echo "  rfd new \"Your First RFD Title\""
}

cmd_new() {
    local title="${1:-}"
    if [[ -z "$title" ]]; then
        echo "usage: rfd new <title>" >&2
        exit 1
    fi

    local num
    num=$(next_number)
    local padded
    padded=$(pad_number "$num")
    local branch
    branch=$(rfd_branch_name "$num")
    local dir="$RFD_DIR/$padded"
    local ext="$RFD_DEFAULT_FORMAT"
    local template_file="$TEMPLATE_DIR/rfd.$ext"

    # Create branch from main
    echo "Creating RFD $padded: $title"
    git -C "$REPO_ROOT" checkout -b "$branch" 2>/dev/null || git -C "$REPO_ROOT" checkout "$branch"

    mkdir -p "$dir"

    if [[ -f "$template_file" ]]; then
        sed -e "s/{number}/$padded/g" -e "s/{title}/$title/g" "$template_file" > "$dir/README.$ext"
    else
        # Fallback: generate minimal template
        cat > "$dir/README.md" << EOF
---
authors:
state: prediscussion
discussion:
---

# RFD $padded $title
EOF
        ext="md"
    fi

    # Initial commit on the branch
    git -C "$REPO_ROOT" add "$dir/README.$ext"
    git -C "$REPO_ROOT" commit -m "rfd: reserve RFD $padded — $title"

    echo ""
    echo "Created $dir/README.$ext"
    echo "Branch: $branch"
    echo ""
    echo "Next steps:"
    echo "  1. Edit $dir/README.$ext"
    echo "  2. git add && git commit"
    echo "  3. rfd discuss $num   # open a pull request"
}

cmd_list() {
    local filter_state=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --state|-s)
                filter_state="$2"
                shift 2
                ;;
            *)
                echo "usage: rfd list [--state STATE]" >&2
                exit 1
                ;;
        esac
    done

    # Collect RFDs from main branch and all rfd/* branches
    local found=0

    printf "%-6s  %-15s  %s\n" "RFD" "STATE" "TITLE"
    printf "%-6s  %-15s  %s\n" "---" "-----" "-----"

    for dir in "$RFD_DIR"/[0-9]*/; do
        [[ -d "$dir" ]] || continue
        local padded
        padded=$(basename "$dir")
        local file=""
        if [[ -f "$dir/README.md" ]]; then
            file="$dir/README.md"
        elif [[ -f "$dir/README.adoc" ]]; then
            file="$dir/README.adoc"
        else
            continue
        fi

        local state
        state=$(get_field "$file" "state")
        local title
        title=$(get_title "$file")

        if [[ -n "$filter_state" && "$state" != "$filter_state" ]]; then
            continue
        fi

        printf "%-6s  %-15s  %s\n" "$padded" "$state" "$title"
        found=1
    done

    # Also check rfd/* branches for RFDs not yet on current branch
    local current_branch
    current_branch=$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo "")
    for branch in $(git -C "$REPO_ROOT" branch --list 'rfd/*' --format='%(refname:short)' 2>/dev/null); do
        local padded="${branch#rfd/}"
        local dir="$RFD_DIR/$padded"
        # Skip if we already listed it from the working tree
        [[ -d "$dir" ]] && continue
        # Try to read from the branch
        local file_content=""
        file_content=$(git -C "$REPO_ROOT" show "$branch:rfd/$padded/README.md" 2>/dev/null) || \
        file_content=$(git -C "$REPO_ROOT" show "$branch:rfd/$padded/README.adoc" 2>/dev/null) || continue

        local state
        state=$(echo "$file_content" | sed -n '/^---$/,/^---$/p' | grep '^state:' | head -1 | sed 's/^state:[[:space:]]*//')
        local title
        title=$(echo "$file_content" | sed '1,/^---$/d' | sed '/^---$/d' | grep -m1 '^#' | sed 's/^#\+[[:space:]]*//')

        if [[ -n "$filter_state" && "$state" != "$filter_state" ]]; then
            continue
        fi

        printf "%-6s  %-15s  %s  (branch: %s)\n" "$padded" "$state" "$title" "$branch"
        found=1
    done

    if [[ "$found" -eq 0 ]]; then
        echo "(no RFDs found)"
    fi
}

cmd_show() {
    local num="${1:-}"
    if [[ -z "$num" ]]; then
        echo "usage: rfd show <number>" >&2
        exit 1
    fi

    # Strip leading zeros for numeric comparison, but use padded for lookup
    num=$(echo "$num" | sed 's/^0*//')
    num=${num:-0}

    local file
    file=$(find_rfd_file "$num") || {
        # Try to find it on a branch
        local padded
        padded=$(pad_number "$num")
        local branch="rfd/$padded"
        local content
        content=$(git -C "$REPO_ROOT" show "$branch:rfd/$padded/README.md" 2>/dev/null) || \
        content=$(git -C "$REPO_ROOT" show "$branch:rfd/$padded/README.adoc" 2>/dev/null) || {
            echo "error: RFD $num not found" >&2
            exit 1
        }
        echo "$content"
        return 0
    }
    cat "$file"
}

cmd_edit() {
    local num="${1:-}"
    if [[ -z "$num" ]]; then
        echo "usage: rfd edit <number>" >&2
        exit 1
    fi

    num=$(echo "$num" | sed 's/^0*//')
    num=${num:-0}

    local padded
    padded=$(pad_number "$num")
    local branch
    branch=$(rfd_branch_name "$num")

    # Switch to the RFD branch if it exists and we're not on it
    local current_branch
    current_branch=$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo "")
    if [[ "$current_branch" != "$branch" ]]; then
        if git -C "$REPO_ROOT" rev-parse --verify "$branch" >/dev/null 2>&1; then
            echo "Switching to branch $branch"
            git -C "$REPO_ROOT" checkout "$branch"
        fi
    fi

    local file
    file=$(find_rfd_file "$num") || {
        echo "error: RFD $num not found" >&2
        exit 1
    }

    $RFD_EDITOR "$file"
}

cmd_state() {
    local num="${1:-}"
    local new_state="${2:-}"

    if [[ -z "$num" || -z "$new_state" ]]; then
        echo "usage: rfd state <number> <state>" >&2
        echo "" >&2
        echo "Valid states: ${VALID_STATES[*]}" >&2
        exit 1
    fi

    num=$(echo "$num" | sed 's/^0*//')
    num=${num:-0}

    if ! is_valid_state "$new_state"; then
        echo "error: invalid state '$new_state'" >&2
        echo "Valid states: ${VALID_STATES[*]}" >&2
        exit 1
    fi

    local file
    file=$(find_rfd_file "$num") || {
        echo "error: RFD $num not found" >&2
        exit 1
    }

    local old_state
    old_state=$(get_field "$file" "state")
    set_field "$file" "state" "$new_state"

    local padded
    padded=$(pad_number "$num")
    echo "RFD $padded: $old_state -> $new_state"

    git -C "$REPO_ROOT" add "$file"
    git -C "$REPO_ROOT" commit -m "rfd: update RFD $padded state to $new_state"
}

cmd_discuss() {
    local num="${1:-}"
    if [[ -z "$num" ]]; then
        echo "usage: rfd discuss <number>" >&2
        exit 1
    fi

    num=$(echo "$num" | sed 's/^0*//')
    num=${num:-0}

    local padded
    padded=$(pad_number "$num")
    local branch
    branch=$(rfd_branch_name "$num")

    # Check if we're on the right branch
    local current_branch
    current_branch=$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo "")
    if [[ "$current_branch" != "$branch" ]]; then
        echo "Switching to branch $branch"
        git -C "$REPO_ROOT" checkout "$branch" 2>/dev/null || {
            echo "error: branch $branch does not exist" >&2
            exit 1
        }
    fi

    local file
    file=$(find_rfd_file "$num") || {
        echo "error: RFD $num not found on branch $branch" >&2
        exit 1
    }

    local title
    title=$(get_title "$file")

    # Update state to discussion
    local current_state
    current_state=$(get_field "$file" "state")
    if [[ "$current_state" != "discussion" ]]; then
        set_field "$file" "state" "discussion"
        git -C "$REPO_ROOT" add "$file"
        git -C "$REPO_ROOT" commit -m "rfd: move RFD $padded to discussion"
    fi

    # Push branch
    echo "Pushing branch $branch..."
    git -C "$REPO_ROOT" push -u origin "$branch"

    # Check if gh CLI is available
    if ! command -v gh &>/dev/null; then
        echo ""
        echo "GitHub CLI (gh) not found. Create a PR manually:"
        echo "  Branch: $branch -> $RFD_MAIN_BRANCH"
        echo "  Title: RFD $padded: $title"
        return 0
    fi

    # Check for existing PR
    local existing_pr
    existing_pr=$(gh pr list --head "$branch" --json url --jq '.[0].url' 2>/dev/null || echo "")
    if [[ -n "$existing_pr" ]]; then
        echo ""
        echo "PR already exists: $existing_pr"

        # Update discussion link in file
        set_field "$file" "discussion" "$existing_pr"
        git -C "$REPO_ROOT" add "$file"
        git -C "$REPO_ROOT" diff --cached --quiet || \
            git -C "$REPO_ROOT" commit -m "rfd: update RFD $padded discussion link"
        return 0
    fi

    # Create PR
    local pr_url
    pr_url=$(gh pr create \
        --base "$RFD_MAIN_BRANCH" \
        --head "$branch" \
        --title "RFD $padded: $title" \
        --body "Discussion for RFD $padded: **$title**

This pull request is for discussing RFD $padded. Please leave comments and feedback here.

**State**: discussion

---
*Created with [OpenRFD](https://github.com/openrfd/openrfd)*" 2>&1) || {
        echo "error: failed to create PR" >&2
        echo "$pr_url" >&2
        exit 1
    }

    echo ""
    echo "Created PR: $pr_url"

    # Update discussion link
    set_field "$file" "discussion" "$pr_url"
    git -C "$REPO_ROOT" add "$file"
    git -C "$REPO_ROOT" commit -m "rfd: add discussion link to RFD $padded"
    git -C "$REPO_ROOT" push
}

cmd_publish() {
    local num="${1:-}"
    if [[ -z "$num" ]]; then
        echo "usage: rfd publish <number>" >&2
        exit 1
    fi

    num=$(echo "$num" | sed 's/^0*//')
    num=${num:-0}

    local padded
    padded=$(pad_number "$num")
    local branch
    branch=$(rfd_branch_name "$num")

    # Switch to branch
    git -C "$REPO_ROOT" checkout "$branch" 2>/dev/null || {
        echo "error: branch $branch does not exist" >&2
        exit 1
    }

    local file
    file=$(find_rfd_file "$num") || {
        echo "error: RFD $num not found" >&2
        exit 1
    }

    # Update state
    set_field "$file" "state" "published"
    git -C "$REPO_ROOT" add "$file"
    git -C "$REPO_ROOT" commit -m "rfd: publish RFD $padded"
    git -C "$REPO_ROOT" push

    # Merge to main
    git -C "$REPO_ROOT" checkout "$RFD_MAIN_BRANCH"
    git -C "$REPO_ROOT" merge --no-ff "$branch" -m "Merge RFD $padded (published)"

    echo "RFD $padded published and merged to $RFD_MAIN_BRANCH"
    echo ""
    echo "Don't forget to push: git push origin $RFD_MAIN_BRANCH"
}

cmd_search() {
    local query="${1:-}"
    if [[ -z "$query" ]]; then
        echo "usage: rfd search <query>" >&2
        exit 1
    fi

    echo "Searching RFDs for: $query"
    echo ""

    local found=0
    # Search in working tree
    while IFS= read -r match; do
        found=1
        echo "$match"
    done < <(grep -rni --include="README.md" --include="README.adoc" "$query" "$RFD_DIR" 2>/dev/null || true)

    if [[ "$found" -eq 0 ]]; then
        echo "(no matches)"
    fi
}

cmd_validate() {
    local num="${1:-}"
    local errors=0

    validate_file() {
        local file="$1" padded="$2"
        local state authors

        # Check frontmatter exists
        if ! head -1 "$file" | grep -q '^---$'; then
            echo "  ERROR: missing YAML frontmatter" >&2
            ((errors++)) || true
            return
        fi

        # Check required fields
        state=$(get_field "$file" "state")
        if [[ -z "$state" ]]; then
            echo "  ERROR: missing 'state' field" >&2
            ((errors++)) || true
        elif ! is_valid_state "$state"; then
            echo "  ERROR: invalid state '$state'" >&2
            ((errors++)) || true
        fi

        authors=$(get_field "$file" "authors")
        if [[ -z "$authors" ]]; then
            echo "  WARNING: missing 'authors' field"
        fi

        # Check title
        local title
        title=$(get_title "$file")
        if [[ -z "$title" ]]; then
            echo "  WARNING: no title heading found"
        fi
    }

    if [[ -n "$num" ]]; then
        num=$(echo "$num" | sed 's/^0*//')
        num=${num:-0}
        local padded
        padded=$(pad_number "$num")
        local file
        file=$(find_rfd_file "$num") || {
            echo "error: RFD $num not found" >&2
            exit 1
        }
        echo "Validating RFD $padded..."
        validate_file "$file" "$padded"
    else
        echo "Validating all RFDs..."
        for dir in "$RFD_DIR"/[0-9]*/; do
            [[ -d "$dir" ]] || continue
            local padded
            padded=$(basename "$dir")
            local file=""
            if [[ -f "$dir/README.md" ]]; then
                file="$dir/README.md"
            elif [[ -f "$dir/README.adoc" ]]; then
                file="$dir/README.adoc"
            else
                echo "RFD $padded: no README found"
                ((errors++)) || true
                continue
            fi
            echo "RFD $padded..."
            validate_file "$file" "$padded"
        done
    fi

    echo ""
    if [[ "$errors" -gt 0 ]]; then
        echo "Validation failed with $errors error(s)"
        exit 1
    else
        echo "All RFDs valid"
    fi
}

cmd_index() {
    local csv_file="$REPO_ROOT/rfd.csv"
    echo "number,title,state,authors,discussion" > "$csv_file"

    for dir in "$RFD_DIR"/[0-9]*/; do
        [[ -d "$dir" ]] || continue
        local padded
        padded=$(basename "$dir")
        local file=""
        if [[ -f "$dir/README.md" ]]; then
            file="$dir/README.md"
        elif [[ -f "$dir/README.adoc" ]]; then
            file="$dir/README.adoc"
        else
            continue
        fi

        local title state authors discussion
        title=$(get_title "$file" | sed 's/,/;/g')
        state=$(get_field "$file" "state")
        authors=$(get_field "$file" "authors" | sed 's/,/;/g')
        discussion=$(get_field "$file" "discussion")

        echo "$padded,\"$title\",$state,\"$authors\",$discussion" >> "$csv_file"
    done

    echo "Updated $csv_file"
}

cmd_help() {
    cat << 'HELP'
rfd - Manage Requests for Discussion

USAGE
    rfd <command> [arguments]

COMMANDS
    init                Initialize a new RFD repository
    new <title>         Create a new RFD
    list [--state ST]   List all RFDs (optionally filter by state)
    show <number>       Display an RFD
    edit <number>       Open an RFD in your editor
    state <num> <state> Update an RFD's state
    discuss <number>    Open a pull request for discussion
    publish <number>    Publish an RFD (merge to main)
    search <query>      Search across all RFDs
    validate [number]   Validate RFD format
    index               Regenerate the RFD index (rfd.csv)
    help                Show this help message
    version             Show version

STATES
    prediscussion   Initial placeholder, active iteration
    ideation        Topic outlined, not actively being revised
    discussion      PR open, active community feedback
    published       Discussion converged, merged to main
    committed       Fully implemented and stable
    abandoned       Deliberately not pursued

EXAMPLES
    rfd init
    rfd new "Add caching layer to API"
    rfd list --state discussion
    rfd discuss 42
    rfd publish 42

ENVIRONMENT
    EDITOR              Preferred text editor (default: vi)

CONFIGURATION
    .rfdconfig          Repository-level settings (sourced as bash)
        RFD_DEFAULT_FORMAT  md or adoc (default: md)
        RFD_MAIN_BRANCH     main or master (default: main)
        RFD_PAD_WIDTH       Zero-pad width (default: 4)
HELP
}

cmd_version() {
    echo "rfd version $VERSION"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        init)
            cmd_init "$@"
            ;;
        new)
            init_repo_root
            load_config
            cmd_new "$@"
            ;;
        list|ls)
            init_repo_root
            load_config
            cmd_list "$@"
            ;;
        show|cat)
            init_repo_root
            load_config
            cmd_show "$@"
            ;;
        edit)
            init_repo_root
            load_config
            cmd_edit "$@"
            ;;
        state)
            init_repo_root
            load_config
            cmd_state "$@"
            ;;
        discuss|pr)
            init_repo_root
            load_config
            cmd_discuss "$@"
            ;;
        publish)
            init_repo_root
            load_config
            cmd_publish "$@"
            ;;
        search|grep)
            init_repo_root
            load_config
            cmd_search "$@"
            ;;
        validate|check)
            init_repo_root
            load_config
            cmd_validate "$@"
            ;;
        index)
            init_repo_root
            load_config
            cmd_index "$@"
            ;;
        help|--help|-h)
            cmd_help
            ;;
        version|--version|-v)
            cmd_version
            ;;
        *)
            echo "error: unknown command '$cmd'" >&2
            echo "Run 'rfd help' for usage." >&2
            exit 1
            ;;
    esac
}

main "$@"
