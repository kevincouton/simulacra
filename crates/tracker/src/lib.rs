//! Simulacra research tracker — static site generator.
//!
//! Reads Markdown documents from `content/`, renders them through the
//! built-in [`markdown`] renderer, and emits a complete static site into
//! `site/dist/`. Run with `cargo run -p simulacra-tracker`.

#![warn(missing_docs)]

pub mod markdown;

use std::fs;
use std::path::{Path, PathBuf};

use markdown::render_markdown;

/// One source page: where it came from and where it lands in `site/dist`.
#[derive(Debug)]
struct Page {
    source: PathBuf,
    output: String,
    nav_title: String,
}

/// Build the whole tracker site from `content_dir` into `output_dir`.
///
/// Returns the list of files written. Fails (with `Err`) if any content
/// document is missing or unreadable.
pub fn build_site(content_dir: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let index = content_dir.join("index.md");
    let stages_dir = content_dir.join("stages");
    if !index.is_file() {
        return Err(format!("missing {}", index.display()));
    }
    if !stages_dir.is_dir() {
        return Err(format!("missing stages directory {}", stages_dir.display()));
    }

    let mut pages = vec![Page {
        source: index,
        output: "index.html".into(),
        nav_title: "Overview".into(),
    }];

    let mut stage_files: Vec<_> = fs::read_dir(&stages_dir)
        .map_err(|e| format!("reading {}: {e}", stages_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    stage_files.sort();

    for path in stage_files {
        let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or("stage").to_string();
        let title = stage_title(&path)?;
        pages.push(Page { source: path, output: format!("{slug}.html"), nav_title: title });
    }

    fs::create_dir_all(output_dir).map_err(|e| format!("creating {}: {e}", output_dir.display()))?;
    fs::write(output_dir.join("style.css"), STYLE_CSS)
        .map_err(|e| format!("writing style.css: {e}"))?;

    let mut written = vec![output_dir.join("style.css")];
    for page in &pages {
        let body = render_markdown(&read(&page.source)?);
        let html = wrap_page(&page.nav_title, &body, &pages);
        let out = output_dir.join(&page.output);
        fs::write(&out, html).map_err(|e| format!("writing {}: {e}", out.display()))?;
        written.push(out);
    }
    Ok(written)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

/// Extract a page title: first `# ` heading if present, else the file stem.
fn stage_title(path: &Path) -> Result<String, String> {
    let src = read(path)?;
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return Ok(rest.trim().to_string());
        }
    }
    Ok(path.file_stem().and_then(|s| s.to_str()).unwrap_or("Stage").to_string())
}

fn wrap_page(title: &str, body: &str, pages: &[Page]) -> String {
    let nav = pages
        .iter()
        .map(|p| {
            let cls = if p.nav_title == title { " class=\"active\"" } else { "" };
            format!("<li><a{cls} href=\"{}\">{}</a></li>", p.output, p.nav_title)
        })
        .collect::<String>();
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title} — Simulacra Tracker</title>\n<link rel=\"stylesheet\" href=\"style.css\">\n\
</head>\n<body>\n<nav><ul>{nav}</ul></nav>\n<main>\n{body}</main>\n\
<footer><p>Simulacra — a research tracker for synthetic human simulation.</p></footer>\n\
</body>\n</html>\n"
    )
}

const STYLE_CSS: &str = "body { font-family: system-ui, sans-serif; max-width: 44rem; \
margin: 2rem auto; padding: 0 1rem; line-height: 1.6; color: #1a1a2e; }
nav ul { display: flex; flex-wrap: wrap; gap: .5rem; list-style: none; padding: 0; }
nav a { text-decoration: none; color: #456; padding: .2rem .5rem; border-radius: .3rem; }
nav a.active { background: #1a1a2e; color: #fff; }
h1 { border-bottom: 2px solid #1a1a2e; padding-bottom: .3rem; }
code { background: #f0f0f4; padding: .1rem .3rem; border-radius: .25rem; }
blockquote { border-left: 3px solid #ccc; margin-left: 0; padding-left: 1rem; color: #555; }
footer { margin-top: 3rem; font-size: .8rem; color: #888; }
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(tmp: &Path) {
        fs::create_dir_all(tmp.join("stages")).unwrap();
        fs::write(tmp.join("index.md"), "# Overview\n\nHello.\n").unwrap();
        fs::write(tmp.join("stages/01-reward.md"), "# Stage 1\n\nText.\n").unwrap();
        fs::write(tmp.join("stages/02-data.md"), "# Stage 2\n\nText.\n").unwrap();
    }

    #[test]
    fn builds_index_and_stage_pages() {
        let tmp = std::env::temp_dir().join("simulacra-tracker-test");
        let _ = fs::remove_dir_all(&tmp);
        fixture(&tmp);
        let out = tmp.join("dist");
        let written = build_site(&tmp, &out).expect("build failed");
        assert!(written.len() >= 4); // style.css + index + 2 stages
        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("<h1>Overview</h1>"));
        assert!(index.contains("01-reward.html"));
        let stage = fs::read_to_string(out.join("01-reward.html")).unwrap();
        assert!(stage.contains("<h1>Stage 1</h1>"));
        assert!(stage.contains("class=\"active\""));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_content_is_an_error() {
        let tmp = std::env::temp_dir().join("simulacra-tracker-empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        assert!(build_site(&tmp, &tmp.join("dist")).is_err());
        let _ = fs::remove_dir_all(&tmp);
    }
}
