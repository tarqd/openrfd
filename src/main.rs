use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use colored::Colorize;

use openrfd::annotation::*;
use openrfd::config::Config;
use openrfd::diagnostic;
use openrfd::rfd::{find_rfd_file, next_rfd_number, Rfd};
use openrfd::state::{State, Visibility};
use openrfd::{build, import, index, refs, repo, search, selector, source_map, validate};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "rfd", version = VERSION, about = "Manage Requests for Discussion")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new RFD repository
    Init,

    /// Create a new RFD
    New {
        /// Title for the new RFD
        title: String,
    },

    /// List all RFDs
    #[command(alias = "ls")]
    List {
        /// Filter by state
        #[arg(short, long)]
        state: Option<String>,
        /// Filter by label
        #[arg(short, long)]
        label: Option<String>,
    },

    /// Display an RFD
    #[command(alias = "cat")]
    Show {
        /// RFD number
        number: u32,
    },

    /// Open an RFD in your editor
    Edit {
        /// RFD number
        number: u32,
    },

    /// Update an RFD's lifecycle state
    State {
        /// RFD number
        number: u32,
        /// New state
        state: String,
    },

    /// Move an RFD to discussion state
    Discuss {
        /// RFD number
        number: u32,
    },

    /// Publish an RFD (merge to main)
    Publish {
        /// RFD number
        number: u32,
    },

    /// Search across all RFDs
    #[command(alias = "grep")]
    Search {
        /// Search query
        query: String,
    },

    /// Validate RFD format and metadata
    #[command(alias = "check")]
    Validate {
        /// Specific RFD number (validates all if omitted)
        number: Option<u32>,
    },

    /// Regenerate the RFD index (rfd.csv)
    Index,

    /// Add an annotation to an RFD
    Annotate {
        /// RFD number
        number: u32,
        /// Quoted text to annotate
        #[arg(long)]
        quote: String,
        /// Comment body
        comment: String,
    },

    /// List or check annotations for an RFD
    Annotations {
        /// RFD number
        number: u32,
        /// Check annotation health
        #[arg(long)]
        check: bool,
        /// Re-anchor stale annotations
        #[arg(long)]
        reanchor: bool,
    },

    /// Import annotations from a GitHub PR
    ImportAnnotations {
        /// RFD number
        number: u32,
        /// PR number
        #[arg(long)]
        pr: u32,
    },

    /// Sync PR review comments into the annotation database
    ///
    /// Merges GitHub's current state with local annotations: new comments
    /// are added, edited comments are updated, and local state (resolved)
    /// is preserved. If --pr is omitted, discovers the PR automatically
    /// from the rfd/NNNN branch.
    Sync {
        /// RFD number
        number: u32,
        /// PR number (auto-discovered from branch if omitted)
        #[arg(long)]
        pr: Option<u32>,
    },

    /// Mark an annotation as resolved
    Resolve {
        /// RFD number
        number: u32,
        /// Annotation ID
        annotation_id: String,
    },

    /// Generate static site
    Build {
        /// Only include public RFDs
        #[arg(long)]
        public: bool,
        /// Only include internal RFDs
        #[arg(long)]
        internal: bool,
    },

    /// Start local dev server
    Serve {
        /// Port number
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Git pre-commit hook — enforces branch discipline
    ///
    /// On `rfd/NNNN` branches, only changes under `rfd/NNNN/` are allowed.
    #[command(name = "pre-commit")]
    PreCommit,

    /// Git post-commit hook — updates RFD metadata refs
    #[command(name = "post-commit")]
    PostCommit,

    /// Show or update configuration
    Config {
        /// Config key to set
        key: Option<String>,
        /// Value to set
        value: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    // Install miette's fancy graphical error handler
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .build(),
        )
    }))
    .ok();

    let cli = Cli::parse();

    let result: Result<()> = async {
        match cli.command {
            Commands::Init => cmd_init()?,
            Commands::Serve { port } => {
                let repo_root = repo::find_repo_root()?;
                build::serve(&repo_root, port).await?;
            }
            _ => {
                let repo_root = repo::find_repo_root()?;
                let config = Config::load(&repo_root)?;
                run_command(cli.command, &repo_root, &config)?;
            }
        }
        Ok(())
    }
    .await;

    if let Err(err) = result {
        // Try to render a rich miette diagnostic from the error chain
        if !diagnostic::try_render_diagnostic(&err) {
            // Fall back to standard anyhow display
            eprintln!("Error: {:#}", err);
        }
        std::process::exit(1);
    }
}

fn run_command(command: Commands, repo_root: &Path, config: &Config) -> Result<()> {
    match command {
        Commands::New { title } => cmd_new(repo_root, config, &title),
        Commands::List { state, label } => cmd_list(repo_root, config, state.as_deref(), label.as_deref()),
        Commands::Show { number } => cmd_show(repo_root, config, number),
        Commands::Edit { number } => cmd_edit(repo_root, config, number),
        Commands::State { number, state } => cmd_state(repo_root, config, number, &state),
        Commands::Discuss { number } => cmd_discuss(repo_root, config, number),
        Commands::Publish { number } => cmd_publish(repo_root, config, number),
        Commands::Search { query } => cmd_search(repo_root, config, &query),
        Commands::Validate { number } => cmd_validate(repo_root, config, number),
        Commands::Index => cmd_index(repo_root, config),
        Commands::Annotate { number, quote, comment } => {
            cmd_annotate(repo_root, config, number, &quote, &comment)
        }
        Commands::Annotations { number, check, reanchor } => {
            cmd_annotations(repo_root, config, number, check, reanchor)
        }
        Commands::ImportAnnotations { number, pr } => cmd_import_annotations(repo_root, config, number, pr),
        Commands::Sync { number, pr } => cmd_sync(repo_root, config, number, pr),
        Commands::Resolve { number, annotation_id } => {
            cmd_resolve(repo_root, config, number, &annotation_id)
        }
        Commands::Build { public, internal } => {
            let filter = if public {
                Some(Visibility::Public)
            } else if internal {
                Some(Visibility::Internal)
            } else {
                None
            };
            build::build_site(repo_root, config, filter)
        }
        Commands::PreCommit => cmd_pre_commit(repo_root, config),
        Commands::PostCommit => cmd_post_commit(repo_root, config),
        Commands::Config { key, value } => cmd_config(repo_root, config, key, value),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn cmd_init() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if cwd.join("rfd").exists() {
        eprintln!("RFD repository already initialized in {}", cwd.display());
        return Ok(());
    }

    // Create directories
    std::fs::create_dir_all(cwd.join("rfd"))?;
    std::fs::create_dir_all(cwd.join("templates"))?;
    std::fs::create_dir_all(cwd.join("static"))?;

    // Write rfd.toml
    let config = Config::default();
    config.save(&cwd)?;

    // Write markdown RFD template
    std::fs::write(
        cwd.join("templates/rfd.md"),
        r#"---
authors:
state: prediscussion
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
"#,
    )?;

    // Write HTML templates
    write_default_html_templates(&cwd)?;

    // Write default static assets
    write_default_static_assets(&cwd)?;

    // Install git hooks (pre-commit + post-commit)
    install_git_hooks(&cwd)?;

    eprintln!("Initialized RFD repository in {}", cwd.display());
    eprintln!("  Created rfd/, templates/, static/, and rfd.toml");
    eprintln!("  Installed git hooks (pre-commit, post-commit)");
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. git add -A && git commit -m \"Initialize RFD repository\"");
    eprintln!("  2. rfd new \"Your First RFD Title\"");

    Ok(())
}

fn cmd_new(repo_root: &Path, config: &Config, title: &str) -> Result<()> {
    // Refuse to create an RFD if there are uncommitted changes — switching
    // branches with a dirty tree carries those changes over and produces
    // confusing commits that include unrelated files.
    if !repo::is_clean(repo_root)? {
        anyhow::bail!(
            "working tree has uncommitted changes\n\n  \
             Commit or stash your changes first, then retry:\n    \
             git add -A && git commit -m \"save work\"\n    \
             # or\n    \
             git stash"
        );
    }

    let num = next_rfd_number(repo_root);
    let padded = config.pad_number(num);
    let branch = config.branch_name(num);
    let dir = config.rfd_path(repo_root, num);
    let ext = &config.rfd.default_format;
    let template_path = config.templates_dir(repo_root).join(format!("rfd.{}", ext));

    eprintln!("Creating RFD {}: {}", padded, title);

    // Create branch
    repo::create_branch(repo_root, &branch)?;

    // Create directory
    std::fs::create_dir_all(&dir)?;

    // Render template
    let file_path = dir.join(format!("README.{}", ext));
    if template_path.exists() {
        let template = std::fs::read_to_string(&template_path)?;
        let content = template
            .replace("{number}", &padded)
            .replace("{title}", title);
        std::fs::write(&file_path, content)?;
    } else {
        let content = format!(
            "---\nauthors:\nstate: prediscussion\n---\n\n# RFD {} {}\n",
            padded, title
        );
        std::fs::write(&file_path, content)?;
    }

    eprintln!();
    eprintln!("Created {}", file_path.display());
    eprintln!("Branch: {}", branch);
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. Edit {}", file_path.display());
    eprintln!("  2. git add -A && git commit -m \"RFD {}: {}\"", padded, title);
    eprintln!("  3. rfd discuss {}   # move to discussion", num);

    Ok(())
}

/// Git pre-commit hook: on `rfd/NNNN` branches, reject staged changes that
/// touch files outside the corresponding `rfd/NNNN/` directory or that modify
/// a different RFD.
fn cmd_pre_commit(repo_root: &Path, config: &Config) -> Result<()> {
    let branch = repo::current_branch(repo_root)?;

    // Only enforce on rfd/* branches
    let num = match parse_rfd_branch(&branch, config) {
        Some(n) => n,
        None => return Ok(()),
    };

    let padded = config.pad_number(num);
    let allowed_prefix = format!("rfd/{}/", padded);

    // Get staged files
    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--cached", "--name-only"])
        .output()
        .context("failed to list staged files")?;

    let staged = String::from_utf8(output.stdout)?;
    let mut violations: Vec<String> = Vec::new();

    for file in staged.lines() {
        let file = file.trim();
        if file.is_empty() {
            continue;
        }
        if !file.starts_with(&allowed_prefix) {
            violations.push(file.to_string());
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    let mut msg = format!(
        "branch `{}` should only contain changes under `{}`\n\n  \
         The following staged files are outside that directory:\n",
        branch, allowed_prefix
    );
    for v in &violations {
        msg.push_str(&format!("    {}\n", v));
    }
    msg.push_str(
        "\n  Unstage them with:\n    \
         git reset HEAD <file>...\n\n  \
         Or commit on the correct branch instead.",
    );

    anyhow::bail!("{}", msg);
}

/// Git post-commit hook: scan the last commit for changed RFD files and
/// create or update the corresponding NoteDb metadata refs.
fn cmd_post_commit(repo_root: &Path, config: &Config) -> Result<()> {
    // List files changed in HEAD commit, relative to repo root.
    let output = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["diff-tree", "--no-commit-id", "-r", "--name-only", "HEAD"])
        .output()
        .context("failed to list changed files in HEAD commit")?;

    if !output.status.success() {
        // Probably an initial commit or detached HEAD — nothing to do.
        return Ok(());
    }

    let changed = String::from_utf8(output.stdout)?;
    let rfd_dir = config.rfd_dir(repo_root);
    let repo = refs::open_repo(repo_root)?;

    for line in changed.lines() {
        let path = Path::new(line);

        // Match paths like  rfd/NNNN/README.md
        let components: Vec<_> = path.components().collect();
        if components.len() < 3 {
            continue;
        }
        let first = components[0].as_os_str().to_str().unwrap_or("");
        let second = components[1].as_os_str().to_str().unwrap_or("");
        if first != "rfd" {
            continue;
        }
        let num: u32 = match second.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let rfd_path = rfd_dir.join(second);
        let file = match find_rfd_file(&rfd_path) {
            Some(f) => f,
            None => continue,
        };

        let rfd = match Rfd::load(&file) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let padded = config.pad_number(num);
        let ref_name = config.ref_name(num);
        let content = std::fs::read(&file)?;
        let state = rfd.frontmatter.state;

        let entries = vec![refs::TreeEntry {
            path: "README.md".into(),
            content,
        }];

        if refs::ref_exists(&repo, &ref_name) {
            refs::write_commit(
                &repo,
                &ref_name,
                &format!("Snapshot RFD {}\n\nState: {}", padded, state),
                &entries,
                &[],
            )?;
        } else {
            refs::create_ref(
                &repo,
                &ref_name,
                &format!("Reserve RFD {}\n\nState: {}", padded, state),
                &entries,
            )?;
        }

        eprintln!("rfd post-commit: updated refs/rfd/{}", padded);
    }

    Ok(())
}

fn cmd_list(repo_root: &Path, config: &Config, state_filter: Option<&str>, label_filter: Option<&str>) -> Result<()> {
    let rfd_dir = config.rfd_dir(repo_root);

    let state_filter = state_filter
        .map(State::from_str)
        .transpose()?;

    println!(
        "{:<6}  {:<15}  {:<12}  {}",
        "RFD".bold(),
        "STATE".bold(),
        "VISIBILITY".bold(),
        "TITLE".bold()
    );
    println!("{:<6}  {:<15}  {:<12}  -----", "---", "-----", "----------");

    let mut entries: Vec<_> = std::fs::read_dir(&rfd_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut count = 0;
    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();

        if name_str.parse::<u32>().is_err() {
            continue;
        }

        let dir = entry.path();
        let file = match find_rfd_file(&dir) {
            Some(f) => f,
            None => continue,
        };

        let rfd = match Rfd::load(&file) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Apply filters
        if let Some(ref sf) = state_filter {
            if rfd.frontmatter.state != *sf {
                continue;
            }
        }

        if let Some(lf) = label_filter {
            if !rfd.frontmatter.labels.iter().any(|l| l == lf) {
                continue;
            }
        }

        let vis = rfd.frontmatter.visibility.unwrap_or_default();
        let state_str = colorize_state(&rfd.frontmatter.state);

        println!(
            "{:<6}  {:<15}  {:<12}  {}",
            name_str,
            state_str,
            vis,
            rfd.title()
        );
        count += 1;
    }

    if count == 0 {
        eprintln!("(no RFDs found)");
    }

    Ok(())
}

fn cmd_show(repo_root: &Path, config: &Config, number: u32) -> Result<()> {
    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", number))?;

    let content = std::fs::read_to_string(&file)?;
    print!("{}", content);

    Ok(())
}

fn cmd_edit(repo_root: &Path, config: &Config, number: u32) -> Result<()> {
    let branch = config.branch_name(number);

    // Switch to RFD branch if it exists
    let current = repo::current_branch(repo_root)?;
    if current != branch && repo::branch_exists(repo_root, &branch) {
        eprintln!("Switching to branch {}", branch);
        repo::checkout(repo_root, &branch)?;
    }

    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", number))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(&file)
        .status()
        .with_context(|| format!("failed to run editor '{}'", editor))?;

    if !status.success() {
        anyhow::bail!("editor exited with non-zero status");
    }

    Ok(())
}

fn cmd_state(repo_root: &Path, config: &Config, number: u32, new_state: &str) -> Result<()> {
    let new_state = State::from_str(new_state)?;
    let padded = config.pad_number(number);
    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", number))?;

    let old_state = Rfd::get_field(&file, "state")?.unwrap_or_default();
    Rfd::set_field(&file, "state", new_state.as_str())?;

    eprintln!("RFD {}: {} -> {}", padded, old_state, new_state);

    let rel_path = file.strip_prefix(repo_root)?.to_string_lossy().to_string();
    repo::commit(
        repo_root,
        &[&rel_path],
        &format!("rfd: update RFD {} state to {}", padded, new_state),
    )?;

    // Record state transition on the ref
    let repo = refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(number);
    let content = std::fs::read(&file)?;
    refs::write_commit(
        &repo,
        &ref_name,
        &format!("State: {}\n\nPrevious-State: {}", new_state, old_state),
        &[refs::TreeEntry {
            path: "README.md".into(),
            content,
        }],
        &[],
    )?;

    Ok(())
}

fn cmd_discuss(repo_root: &Path, config: &Config, number: u32) -> Result<()> {
    let padded = config.pad_number(number);
    let branch = config.branch_name(number);

    // Must be on the RFD branch
    let current = repo::current_branch(repo_root)?;
    if current != branch {
        anyhow::bail!(
            "not on branch {}\n\n  \
             Switch to the RFD branch first:\n    \
             git checkout {}",
            branch,
            branch
        );
    }

    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found on branch {}", number, branch))?;

    let rfd = Rfd::load(&file)?;

    if rfd.frontmatter.state == State::Discussion {
        eprintln!("RFD {} is already in discussion state", padded);
        return Ok(());
    }

    Rfd::set_field(&file, "state", "discussion")?;
    let rel_path = file.strip_prefix(repo_root)?.to_string_lossy().to_string();
    repo::commit(
        repo_root,
        &[&rel_path],
        &format!("rfd: move RFD {} to discussion", padded),
    )?;

    eprintln!("RFD {} is now in discussion state", padded);
    Ok(())
}

fn cmd_publish(repo_root: &Path, config: &Config, number: u32) -> Result<()> {
    let padded = config.pad_number(number);
    let branch = config.branch_name(number);

    // Switch to branch
    repo::checkout(repo_root, &branch)?;

    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", number))?;

    // Update state
    Rfd::set_field(&file, "state", "published")?;
    let rel_path = file.strip_prefix(repo_root)?.to_string_lossy().to_string();
    repo::commit(
        repo_root,
        &[&rel_path],
        &format!("rfd: publish RFD {}", padded),
    )?;
    repo::push(repo_root, &branch)?;

    // Merge to main
    repo::checkout(repo_root, &config.rfd.main_branch)?;
    repo::merge(
        repo_root,
        &branch,
        &format!("Merge RFD {} (published)", padded),
    )?;

    // Record publish + revision snapshot on the ref
    let repo = refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(number);
    let content = std::fs::read(&file)?;
    refs::write_commit(
        &repo,
        &ref_name,
        "State: published",
        &[refs::TreeEntry {
            path: "README.md".into(),
            content,
        }],
        &[],
    )?;

    eprintln!("RFD {} published and merged to {}", padded, config.rfd.main_branch);
    eprintln!();
    eprintln!("Don't forget to push: git push origin {}", config.rfd.main_branch);

    Ok(())
}

fn cmd_search(repo_root: &Path, config: &Config, query: &str) -> Result<()> {
    eprintln!("Searching RFDs for: {}", query);
    eprintln!();

    let results = search::search(repo_root, config, query)?;

    if results.is_empty() {
        eprintln!("(no matches)");
        return Ok(());
    }

    for result in &results {
        println!(
            "{}:{} [{}] {}",
            format!("RFD {}", result.number).cyan(),
            result.line,
            result.state,
            result.context
        );
    }

    eprintln!();
    eprintln!("{} match(es)", results.len());

    Ok(())
}

fn cmd_validate(repo_root: &Path, config: &Config, number: Option<u32>) -> Result<()> {
    if let Some(num) = number {
        let padded = config.pad_number(num);
        let dir = config.rfd_path(repo_root, num);
        let file = find_rfd_file(&dir)
            .ok_or_else(|| anyhow::anyhow!("RFD {} not found", num))?;

        eprintln!("Validating RFD {}...", padded);
        let result = validate::validate_rfd(&file);

        // Prefer rich diagnostics
        if !result.diagnostics.is_empty() {
            for diag in &result.diagnostics {
                eprintln!("{:?}", miette::Report::new_boxed(Box::new(diag.clone())));
            }
        } else {
            for err in &result.errors {
                eprintln!("  {} {}", "ERROR:".red().bold(), err);
            }
            for warn in &result.warnings {
                eprintln!("  {} {}", "WARNING:".yellow().bold(), warn);
            }
        }

        if result.is_ok() {
            eprintln!("{}", "Valid".green());
        } else {
            std::process::exit(1);
        }
    } else {
        let ok = validate::validate_all(repo_root, config)?;
        if !ok {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_index(repo_root: &Path, config: &Config) -> Result<()> {
    index::generate_csv_index(repo_root, config)
}

fn cmd_annotate(
    repo_root: &Path,
    config: &Config,
    number: u32,
    quote: &str,
    comment: &str,
) -> Result<()> {
    let padded = config.pad_number(number);
    let dir = config.rfd_path(repo_root, number);
    let file = find_rfd_file(&dir)
        .ok_or_else(|| anyhow::anyhow!("RFD {} not found", number))?;

    let rfd = Rfd::load(&file)?;
    let source = format!("rfd/{}/README.md", padded);

    // Find the quoted text in the source
    let match_result = selector::match_text_quote(&rfd.body, quote, None, None);
    if match_result.range.is_none() {
        anyhow::bail!("quoted text not found in RFD {}", padded);
    }
    let range = match_result.range.unwrap();

    // Build selectors
    let prefix = if range.start > 20 {
        Some(rfd.body[range.start - 20..range.start].to_string())
    } else if range.start > 0 {
        Some(rfd.body[..range.start].to_string())
    } else {
        None
    };

    let suffix = if range.end + 20 < rfd.body.len() {
        Some(rfd.body[range.end..range.end + 20].to_string())
    } else if range.end < rfd.body.len() {
        Some(rfd.body[range.end..].to_string())
    } else {
        None
    };

    let line = source_map::line_number_at(&rfd.body, range.start);

    let commit = repo::head_sha(repo_root).unwrap_or_default();
    let branch_name = repo::current_branch(repo_root).ok();

    let annotation = Annotation {
        context: "http://www.w3.org/ns/anno.jsonld".into(),
        annotation_type: "Annotation".into(),
        id: Annotation::new_id(number, config.rfd.pad_width),
        creator: Creator {
            creator_type: "Person".into(),
            name: whoami().unwrap_or_else(|| "anonymous".into()),
            email: git_email(repo_root),
            url: None,
        },
        created: Utc::now(),
        motivation: Motivation::Commenting,
        body: AnnotationBody {
            body_type: "TextualBody".into(),
            value: comment.to_string(),
            format: "text/markdown".into(),
        },
        target: AnnotationTarget::Resource(SpecificResource {
            resource_type: "SpecificResource".into(),
            source,
            state: Some(GitState {
                state_type: "GitState".into(),
                commit,
                r#ref: branch_name,
            }),
            selector: vec![
                Selector::TextQuoteSelector {
                    exact: quote.to_string(),
                    prefix,
                    suffix,
                },
                Selector::FragmentSelector {
                    value: format!("line={},{}", line, line),
                    conforms_to: "http://tools.ietf.org/rfc/rfc5147".into(),
                },
                Selector::TextPositionSelector {
                    start: range.start,
                    end: range.end,
                },
            ],
        }),
        modified: None,
        resolved: None,
        origin: None,
    };

    // Store annotation on the RFD's custom ref (never touches working tree)
    let repo = refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(number);

    let mut collection = AnnotationCollection::load_from_ref(&repo, &ref_name, "annotations/cli.json")?
        .unwrap_or_else(|| AnnotationCollection::new("CLI Annotations"));

    eprintln!("Added annotation: {}", annotation.id);
    collection.items.push(annotation);
    collection.save_to_ref(
        &repo,
        &ref_name,
        "annotations/cli.json",
        &format!("Add annotation to RFD {}", padded),
    )?;

    Ok(())
}

fn cmd_annotations(
    repo_root: &Path,
    config: &Config,
    number: u32,
    check: bool,
    _reanchor: bool,
) -> Result<()> {
    if check {
        return validate::check_annotations(repo_root, config, number);
    }

    let padded = config.pad_number(number);
    let repo = refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(number);

    let collections = AnnotationCollection::load_all_from_ref(&repo, &ref_name)?;

    let mut total = 0;
    for (source_name, collection) in &collections {
        let source_stem = source_name.trim_end_matches(".json");
        if !collection.items.is_empty() {
            println!("{} ({})", collection.label.bold(), source_stem);
            for annotation in &collection.items {
                let creator = &annotation.creator.name;
                let date = annotation.created.format("%Y-%m-%d");
                let body = &annotation.body.value;
                let resolved = if annotation.resolved.is_some() {
                    " [resolved]".dimmed().to_string()
                } else {
                    String::new()
                };

                println!("  {} {} @ {}{}", "•".cyan(), creator, date, resolved);

                // Show quoted text if available
                if let AnnotationTarget::Resource(ref res) = annotation.target {
                    for sel in &res.selector {
                        if let Selector::TextQuoteSelector { exact, .. } = sel {
                            let truncated = if exact.len() > 60 {
                                format!("{}...", &exact[..60])
                            } else {
                                exact.clone()
                            };
                            println!("    > {}", truncated.dimmed());
                            break;
                        }
                    }
                }

                // Show comment
                for line in body.lines() {
                    println!("    {}", line);
                }
                println!();

                total += 1;
            }
        }
    }

    if total == 0 {
        eprintln!("No annotations for RFD {}", padded);
    } else {
        eprintln!("{} annotation(s) total", total);
    }

    Ok(())
}

fn cmd_import_annotations(
    repo_root: &Path,
    config: &Config,
    number: u32,
    pr: u32,
) -> Result<()> {
    // Legacy one-shot import — prefer `rfd sync` for incremental updates.
    // Delegates to sync so existing invocations get the merge behavior.
    import::sync_pr_annotations(repo_root, config, number, pr)?;
    Ok(())
}

fn cmd_sync(
    repo_root: &Path,
    config: &Config,
    number: u32,
    pr: Option<u32>,
) -> Result<()> {
    let pr_number = match pr {
        Some(n) => n,
        None => {
            eprintln!("Discovering PR for RFD {}...", config.pad_number(number));
            import::find_pr_for_rfd(repo_root, config, number)?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no open PR found with head branch {}\n\n  \
                         Specify the PR number explicitly:\n    \
                         rfd sync {} --pr <number>",
                        config.branch_name(number),
                        number
                    )
                })?
        }
    };

    import::sync_pr_annotations(repo_root, config, number, pr_number)?;
    Ok(())
}

fn cmd_resolve(
    repo_root: &Path,
    config: &Config,
    number: u32,
    annotation_id: &str,
) -> Result<()> {
    let padded = config.pad_number(number);
    let repo = refs::open_repo(repo_root)?;
    let ref_name = config.ref_name(number);

    let collections = AnnotationCollection::load_all_from_ref(&repo, &ref_name)?;

    for (filename, mut collection) in collections {
        let mut found = false;
        for annotation in &mut collection.items {
            if annotation.id == annotation_id {
                annotation.resolved = Some(Utc::now());
                found = true;
                break;
            }
        }

        if found {
            let path = format!("annotations/{}", filename);
            collection.save_to_ref(
                &repo,
                &ref_name,
                &path,
                &format!("Resolve annotation in RFD {}", padded),
            )?;
            eprintln!("Resolved annotation: {}", annotation_id);
            return Ok(());
        }
    }

    anyhow::bail!("annotation {} not found in RFD {}", annotation_id, padded);
}

fn cmd_config(
    repo_root: &Path,
    config: &Config,
    key: Option<String>,
    value: Option<String>,
) -> Result<()> {
    match (key, value) {
        (None, _) => {
            // Show current config
            let toml_str = toml::to_string_pretty(config)?;
            println!("{}", toml_str);
        }
        (Some(key), Some(value)) => {
            let mut config = config.clone();
            match key.as_str() {
                "rfd.default_format" => config.rfd.default_format = value,
                "rfd.main_branch" => config.rfd.main_branch = value,
                "rfd.pad_width" => config.rfd.pad_width = value.parse()?,
                "github.app_client_id" => config.github.app_client_id = value,
                "github.redirect_uri" => config.github.redirect_uri = Some(value),
                other => anyhow::bail!("unknown config key: {}", other),
            }
            config.save(repo_root)?;
            eprintln!("Updated {}", key);
        }
        (Some(key), None) => {
            // Show a specific key
            match key.as_str() {
                "rfd.default_format" => println!("{}", config.rfd.default_format),
                "rfd.main_branch" => println!("{}", config.rfd.main_branch),
                "rfd.pad_width" => println!("{}", config.rfd.pad_width),
                "github.app_client_id" => println!("{}", config.github.app_client_id),
                "github.redirect_uri" => {
                    println!("{}", config.github.redirect_uri.as_deref().unwrap_or(""))
                }
                other => anyhow::bail!("unknown config key: {}", other),
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an `rfd/NNNN` branch name into the RFD number.
/// Returns `None` if the branch doesn't match the pattern.
fn parse_rfd_branch(branch: &str, config: &Config) -> Option<u32> {
    let rest = branch.strip_prefix("rfd/")?;
    let num: u32 = rest.parse().ok()?;
    // Sanity-check that it round-trips to the expected padded form
    if rest != config.pad_number(num) {
        return None;
    }
    Some(num)
}

/// Install pre-commit and post-commit hooks into `.git/hooks/`.
/// Existing hooks are preserved by appending rather than overwriting.
fn install_git_hooks(repo_root: &Path) -> Result<()> {
    let hooks_dir = repo_root.join(".git/hooks");
    if !hooks_dir.exists() {
        std::fs::create_dir_all(&hooks_dir)?;
    }

    let hooks: &[(&str, &str)] = &[
        (
            "pre-commit",
            "#!/bin/sh\n# rfd: enforce branch discipline on rfd/* branches\nrfd pre-commit\n",
        ),
        (
            "post-commit",
            "#!/bin/sh\n# rfd: update metadata refs for committed RFDs\nrfd post-commit\n",
        ),
    ];

    // Check for conflicts before writing anything
    for (name, _) in hooks {
        let path = hooks_dir.join(name);
        if path.exists() {
            // .sample files ship with git init — ignore those
            if name.ends_with(".sample") {
                continue;
            }
            let existing = std::fs::read_to_string(&path)?;
            if existing.contains("rfd pre-commit") || existing.contains("rfd post-commit") {
                continue; // already ours
            }
            anyhow::bail!(
                ".git/hooks/{name} already exists\n\n  \
                 Add the following to your existing hook:\n    \
                 rfd {name}\n\n  \
                 Or remove the hook and re-run `rfd init`."
            );
        }
    }

    for (name, content) in hooks {
        let path = hooks_dir.join(name);
        if path.exists() {
            let existing = std::fs::read_to_string(&path)?;
            if existing.contains("rfd pre-commit") || existing.contains("rfd post-commit") {
                continue; // already installed
            }
        }

        std::fs::write(&path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&path, perms)?;
        }
    }

    Ok(())
}

fn colorize_state(state: &State) -> String {
    match state {
        State::Prediscussion => state.to_string().dimmed().to_string(),
        State::Ideation => state.to_string().blue().to_string(),
        State::Discussion => state.to_string().yellow().to_string(),
        State::Published => state.to_string().green().to_string(),
        State::Committed => state.to_string().green().bold().to_string(),
        State::Abandoned => state.to_string().red().to_string(),
    }
}

fn whoami() -> Option<String> {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn git_email(repo_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["config", "user.email"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let email = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if email.is_empty() {
                    None
                } else {
                    Some(email)
                }
            } else {
                None
            }
        })
}

fn write_default_html_templates(root: &std::path::Path) -> Result<()> {
    let templates_dir = root.join("templates");
    std::fs::create_dir_all(&templates_dir)?;

    std::fs::write(
        templates_dir.join("base.html"),
        include_str!("../templates/base.html.default"),
    )?;
    std::fs::write(
        templates_dir.join("index.html"),
        include_str!("../templates/index.html.default"),
    )?;
    std::fs::write(
        templates_dir.join("rfd.html"),
        include_str!("../templates/rfd.html.default"),
    )?;

    Ok(())
}

fn write_default_static_assets(root: &std::path::Path) -> Result<()> {
    let static_dir = root.join("static");
    std::fs::create_dir_all(&static_dir)?;

    std::fs::write(
        static_dir.join("style.css"),
        include_str!("../static/style.css.default"),
    )?;
    std::fs::write(
        static_dir.join("rfd.js"),
        include_str!("../static/rfd.js.default"),
    )?;

    Ok(())
}
