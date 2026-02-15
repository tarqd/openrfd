use std::path::Path;

use anyhow::{Context, Result};
use tera::Tera;

use crate::annotation::AnnotationCollection;
use crate::config::Config;
use crate::render::render_markdown;
use crate::rfd::{find_rfd_file, Rfd};
use crate::search::build_search_index;
use crate::state::Visibility;

/// Build the static site into `_site/`.
pub fn build_site(repo_root: &Path, config: &Config, visibility_filter: Option<Visibility>) -> Result<()> {
    let output_dir = repo_root.join("_site");
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)?;
    }
    std::fs::create_dir_all(&output_dir)?;

    // Load Tera templates
    let templates_dir = config.templates_dir(repo_root);
    let templates_glob = format!("{}/**/*.html", templates_dir.display());
    let tera = Tera::new(&templates_glob)
        .with_context(|| format!("loading templates from {}", templates_dir.display()))?;

    // Collect all RFDs
    let rfd_dir = config.rfd_dir(repo_root);
    let mut rfds = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&rfd_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

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
            Err(e) => {
                eprintln!("warning: skipping RFD {}: {}", name_str, e);
                continue;
            }
        };

        // Filter by visibility
        let vis = rfd.frontmatter.visibility.unwrap_or_default();
        if vis == Visibility::Confidential {
            continue;
        }
        if let Some(filter) = visibility_filter {
            if vis != filter {
                continue;
            }
        }

        rfds.push(rfd);
    }

    // Build index page
    build_index_page(&tera, &rfds, &output_dir, config)?;

    // Build individual RFD pages
    for rfd in &rfds {
        build_rfd_page(&tera, rfd, repo_root, &output_dir, config)?;
    }

    // Build search index
    let search_index = build_search_index(repo_root, config)?;
    let search_json = serde_json::to_string(&search_index)?;
    std::fs::write(output_dir.join("search-index.json"), search_json)?;

    // Copy static files
    let static_dir = config.static_dir(repo_root);
    if static_dir.exists() {
        copy_dir_recursive(&static_dir, &output_dir)?;
    }

    let rfd_count = rfds.len();
    eprintln!("Built site with {} RFDs in {}", rfd_count, output_dir.display());

    Ok(())
}

fn build_index_page(tera: &Tera, rfds: &[Rfd], output_dir: &Path, config: &Config) -> Result<()> {
    let mut context = tera::Context::new();

    let rfd_list: Vec<_> = rfds
        .iter()
        .map(|rfd| {
            let mut map = std::collections::BTreeMap::new();
            map.insert("number".to_string(), config.pad_number(rfd.number));
            map.insert("title".to_string(), rfd.title());
            map.insert("state".to_string(), rfd.frontmatter.state.to_string());
            map.insert(
                "authors".to_string(),
                rfd.frontmatter.authors.clone().unwrap_or_default(),
            );
            map.insert(
                "labels".to_string(),
                rfd.frontmatter.labels.join(", "),
            );
            map
        })
        .collect();

    context.insert("rfds", &rfd_list);
    context.insert("title", "RFD Index");

    let html = tera.render("index.html", &context).context("rendering index.html")?;
    std::fs::write(output_dir.join("index.html"), html)?;

    Ok(())
}

fn build_rfd_page(
    tera: &Tera,
    rfd: &Rfd,
    _repo_root: &Path,
    output_dir: &Path,
    config: &Config,
) -> Result<()> {
    let padded = config.pad_number(rfd.number);
    let rfd_output_dir = output_dir.join("rfd").join(&padded);
    std::fs::create_dir_all(&rfd_output_dir)?;

    // Render markdown to HTML with source mapping
    let rendered = render_markdown(&rfd.body);

    // Load annotations
    let annotations_dir = rfd.path.parent().unwrap().join("annotations");
    let mut all_annotations = Vec::new();
    if annotations_dir.exists() {
        for entry in std::fs::read_dir(&annotations_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(collection) = AnnotationCollection::load(&path) {
                    all_annotations.extend(collection.items);
                }
            }
        }
    }

    let annotations_json = serde_json::to_string(&all_annotations)?;

    let mut context = tera::Context::new();
    context.insert("number", &padded);
    context.insert("title", &rfd.title());
    context.insert("state", &rfd.frontmatter.state.to_string());
    context.insert("authors", &rfd.frontmatter.authors.clone().unwrap_or_default());
    context.insert("discussion", &rfd.frontmatter.discussion.clone().unwrap_or_default());
    context.insert("labels", &rfd.frontmatter.labels);
    context.insert("content", &rendered.html);
    context.insert("annotations_json", &annotations_json);
    context.insert("github_client_id", &config.github.app_client_id);

    let html = tera.render("rfd.html", &context).context("rendering rfd.html")?;
    std::fs::write(rfd_output_dir.join("index.html"), html)?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Start a local dev server on the given port.
pub async fn serve(repo_root: &Path, port: u16) -> Result<()> {
    let site_dir = repo_root.join("_site");

    if !site_dir.exists() {
        anyhow::bail!("_site/ not found. Run `rfd build` first.");
    }

    use axum::Router;
    use tower_http::services::ServeDir;

    let app = Router::new().fallback_service(ServeDir::new(&site_dir));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("Serving site at http://{}", addr);
    eprintln!("Press Ctrl+C to stop.");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
