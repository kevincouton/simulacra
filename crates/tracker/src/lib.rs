//! Simulacra research tracker — static site generator.
//!
//! Reads Markdown documents from `content/`, renders them through the
//! built-in [`markdown`] renderer, and emits a complete, SEO/AEO-optimized
//! static site into `site/dist/`: semantic HTML5, per-page meta
//! descriptions, OpenGraph/Twitter tags, canonical URLs, JSON-LD structured
//! data (WebSite, TechArticle, FAQPage, ItemList), `sitemap.xml`, and
//! `robots.txt`. Run with `cargo run -p simulacra-tracker`.

#![warn(missing_docs)]

pub mod markdown;

use std::fs;
use std::path::{Path, PathBuf};

use markdown::render_markdown;

/// Canonical public origin the site is served from.
pub const BASE_URL: &str = "https://simulation.lucanian.app";

/// Content last substantially modified, for structured-data dates.
pub const DATE_MODIFIED: &str = "2026-10-08";

/// One question/answer pair for the index FAQ (rendered as content and as
/// `FAQPage` JSON-LD — the strongest AEO surface on the site).
pub struct FaqEntry {
    /// Question text.
    pub q: &'static str,
    /// Answer text.
    pub a: &'static str,
}

/// The index FAQ. Keep in sync with the FAQ section of `content/index.md`.
pub const FAQ: &[FaqEntry] = &[
    FaqEntry {
        q: "What is synthetic human simulation?",
        a: "The trend of replacing each human role in the AI training loop — judge, data labeler, teacher, curriculum designer, researcher, environment builder, and research subject — with a model. Synthetic data, synthetic rubrics, AI researchers, and end-to-end RL environments are all instances of the same idea: simulation that is roughly 10% worse than the human original but 100x cheaper and 10,000x faster.",
    },
    FaqEntry {
        q: "What are the stages of the synthetic simulation stack?",
        a: "Eight stages, in the order they were automated: (1) the reward signal — RLHF reward models and LLM judges; (2) the training data — Phi-style synthetic corpora; (3) the teacher — model distillation; (4) the curriculum — self-instructing models; (5) the researcher — autonomous experiment loops like Karpathy's autoresearch; (6) the environment — synthesized RL worlds with synthesized verifiers; (7) the human subject — digital twins of real people for surveys and A/B tests; (8) the physical world — the one layer that resists full synthesis.",
    },
    FaqEntry {
        q: "Where does synthetic simulation break?",
        a: "Independent evaluations converge on one finding: simulated populations track average responses well but fail on variance, price sensitivity, and distribution tails, and they break on lived experience and emotional nuance. Simulation quality is a measured quantity — like the synthesized verifiers it depends on, a simulator must be stress-tested against ground truth before anyone acts on its output.",
    },
    FaqEntry {
        q: "What is Simulacra?",
        a: "Simulacra is an open-source Rust workspace that prototypes the pipeline: seeded environment synthesis, constraint-based binary verifier synthesis with oracle/no-op/unsolved stress tests, heuristic agent personas behind an LLM provider trait, a Synthesizer/Solver co-evolution loop, a gossip-based social layer, and distributional-fidelity experiments that check simulated populations against theory.",
    },
];

/// One source page: where it came from and where it lands in `site/dist`.
#[derive(Debug)]
struct Page {
    source: PathBuf,
    output: String,
    slug: String,
    nav_title: String,
    description: String,
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

    let mut pages = vec![load_page(index, "index".into(), "index.html".into())?];

    let mut stage_files: Vec<_> = fs::read_dir(&stages_dir)
        .map_err(|e| format!("reading {}: {e}", stages_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    stage_files.sort();

    for path in stage_files {
        let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or("stage").to_string();
        pages.push(load_page(path, slug.clone(), format!("{slug}.html"))?);
    }

    fs::create_dir_all(output_dir).map_err(|e| format!("creating {}: {e}", output_dir.display()))?;
    fs::write(output_dir.join("style.css"), STYLE_CSS)
        .map_err(|e| format!("writing style.css: {e}"))?;

    let mut written = vec![output_dir.join("style.css")];
    for i in 0..pages.len() {
        let body = render_markdown(&read(&pages[i].source)?);
        let is_index = pages[i].slug == "index";
        let html = wrap_page(&pages[i], &body, &pages, is_index);
        let out = output_dir.join(&pages[i].output);
        fs::write(&out, html).map_err(|e| format!("writing {}: {e}", out.display()))?;
        written.push(out);
    }

    let sitemap = render_sitemap(&pages);
    fs::write(output_dir.join("sitemap.xml"), sitemap).map_err(|e| format!("writing sitemap: {e}"))?;
    written.push(output_dir.join("sitemap.xml"));
    fs::write(output_dir.join("robots.txt"), render_robots())
        .map_err(|e| format!("writing robots.txt: {e}"))?;
    written.push(output_dir.join("robots.txt"));
    Ok(written)
}

/// Load a page's metadata: title from its first `# ` heading, description
/// from its first paragraph (Markdown stripped).
fn load_page(source: PathBuf, slug: String, output: String) -> Result<Page, String> {
    let src = read(&source)?;
    let mut title: Option<String> = None;
    let mut paragraph = String::new();
    let mut in_paragraph = false;
    for line in src.lines() {
        if title.is_none() {
            if let Some(rest) = line.strip_prefix("# ") {
                title = Some(rest.trim().to_string());
                continue;
            }
        }
        let t = line.trim();
        if t.is_empty() {
            if in_paragraph {
                break;
            }
            continue;
        }
        if title.is_some() && !t.starts_with('#') {
            in_paragraph = true;
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(t);
        }
    }
    let description = truncate_words(&strip_markdown(&paragraph), 200);
    let fallback = slug.clone();
    Ok(Page {
        source,
        output,
        slug,
        nav_title: title.unwrap_or(fallback),
        description,
    })
}

/// Truncate to `max` characters at a word boundary.
fn truncate_words(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max).collect();
    match truncated.rfind(' ') {
        Some(idx) => truncated[..idx].to_string(),
        None => truncated,
    }
}

/// Remove the Markdown inline syntax the tracker content actually uses.
fn strip_markdown(s: &str) -> String {
    s.replace("**", "").replace('*', "").replace('`', "")
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn canonical(slug: &str) -> String {
    if slug == "index" {
        format!("{BASE_URL}/")
    } else {
        format!("{BASE_URL}/{slug}.html")
    }
}

fn wrap_page(page: &Page, body: &str, pages: &[Page], is_index: bool) -> String {
    let nav = pages
        .iter()
        .map(|p| {
            let cls = if p.output == page.output { " class=\"active\" aria-current=\"page\"" } else { "" };
            format!("<li><a{cls} href=\"{}\">{}</a></li>", p.output, esc(&p.nav_title))
        })
        .collect::<String>();

    let title = format!("{} — Simulacra", page.nav_title);
    let canonical_url = canonical(&page.slug);
    let description = esc(&page.description);

    let json_ld = if is_index {
        let faq: String = FAQ
            .iter()
            .map(|f| {
                format!(
                    "{{\"@type\":\"Question\",\"name\":\"{}\",\"acceptedAnswer\":{{\"@type\":\"Answer\",\"text\":\"{}\"}}}}",
                    esc(f.q),
                    esc(f.a)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let items: String = pages
            .iter()
            .filter(|p| p.slug != "index")
            .map(|p| {
                format!(
                    "{{\"@type\":\"ListItem\",\"position\":{},\"name\":\"{}\",\"url\":\"{}\"}}",
                    pages.iter().position(|x| x.slug == p.slug).unwrap_or(1),
                    esc(&p.nav_title),
                    canonical(&p.slug)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            concat!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"WebSite\",\"name\":\"Simulacra\",",
                "\"url\":\"{base}/\",\"description\":\"{desc}\",\"inLanguage\":\"en\"}},",
                "{{\"@context\":\"https://schema.org\",\"@type\":\"FAQPage\",\"mainEntity\":[{faq}]}},",
                "{{\"@context\":\"https://schema.org\",\"@type\":\"ItemList\",\"itemListElement\":[{items}]}}"
            ),
            base = BASE_URL,
            desc = description,
            faq = faq,
            items = items
        )
    } else {
        format!(
            concat!(
                "{{\"@context\":\"https://schema.org\",\"@type\":\"TechArticle\",\"headline\":\"{h}\",",
                "\"description\":\"{desc}\",\"url\":\"{url}\",\"dateModified\":\"{date}\",",
                "\"author\":{{\"@type\":\"Organization\",\"name\":\"Simulacra\",\"url\":\"{base}/\"}},",
                "\"isPartOf\":{{\"@type\":\"WebSite\",\"name\":\"Simulacra\",\"url\":\"{base}/\"}},\"inLanguage\":\"en\"}}"
            ),
            h = esc(&page.nav_title),
            desc = description,
            url = canonical_url,
            date = DATE_MODIFIED,
            base = BASE_URL
        )
    };

    let article_tag = if is_index { "div" } else { "article" };
    format!(
        concat!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n",
            "<meta charset=\"utf-8\">\n",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
            "<title>{title}</title>\n",
            "<meta name=\"description\" content=\"{description}\">\n",
            "<link rel=\"canonical\" href=\"{canonical_url}\">\n",
            "<meta property=\"og:type\" content=\"{og_type}\">\n",
            "<meta property=\"og:site_name\" content=\"Simulacra\">\n",
            "<meta property=\"og:title\" content=\"{title}\">\n",
            "<meta property=\"og:description\" content=\"{description}\">\n",
            "<meta property=\"og:url\" content=\"{canonical_url}\">\n",
            "<meta name=\"twitter:card\" content=\"summary\">\n",
            "<meta name=\"twitter:title\" content=\"{title}\">\n",
            "<meta name=\"twitter:description\" content=\"{description}\">\n",
            "<script type=\"application/ld+json\">[{json_ld}]</script>\n",
            "<link rel=\"stylesheet\" href=\"style.css\">\n",
            "</head>\n<body>\n",
            "<header><nav aria-label=\"Sections\"><ul>{nav}</ul></nav></header>\n",
            "<main>\n<{article_tag}>\n{body}</{article_tag}>\n</main>\n",
            "<footer><p>Simulacra — a research tracker for synthetic human simulation. ",
            "<a href=\"/app/\">Try the playground</a>.</p></footer>\n",
            "</body>\n</html>\n"
        ),
        title = esc(&title),
        description = description,
        canonical_url = canonical_url,
        og_type = if is_index { "website" } else { "article" },
        json_ld = json_ld,
        nav = nav,
        article_tag = article_tag,
        body = body,
    )
}

fn render_sitemap(pages: &[Page]) -> String {
    let urls: String = pages
        .iter()
        .map(|p| {
            format!(
                "<url><loc>{}</loc><lastmod>{}</lastmod></url>",
                canonical(&p.slug),
                DATE_MODIFIED
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  {}\n</urlset>\n",
        urls
    )
}

fn render_robots() -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {BASE_URL}/sitemap.xml\n")
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
        fs::write(tmp.join("index.md"), "# Overview\n\nHello world.\n").unwrap();
        fs::write(tmp.join("stages/01-reward.md"), "# Stage 1\n\nReward text.\n").unwrap();
        fs::write(tmp.join("stages/02-data.md"), "# Stage 2\n\nData text.\n").unwrap();
    }

    #[test]
    fn builds_pages_with_seo_metadata() {
        let tmp = std::env::temp_dir().join("simulacra-tracker-test");
        let _ = fs::remove_dir_all(&tmp);
        fixture(&tmp);
        let out = tmp.join("dist");
        let written = build_site(&tmp, &out).expect("build failed");
        // style.css + index + 2 stages + sitemap + robots
        assert_eq!(written.len(), 6);

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("<h1>Overview</h1>"));
        assert!(index.contains("01-reward.html"));
        assert!(index.contains("<meta name=\"description\" content=\"Hello world.\">"));
        assert!(index.contains("<link rel=\"canonical\" href=\"https://simulation.lucanian.app/\">"));
        assert!(index.contains("og:type\" content=\"website"));
        assert!(index.contains("\"@type\":\"FAQPage\""));
        assert!(index.contains("\"@type\":\"WebSite\""));
        assert!(index.contains("\"@type\":\"ItemList\""));
        assert!(index.contains("<header><nav aria-label=\"Sections\">"));

        let stage = fs::read_to_string(out.join("01-reward.html")).unwrap();
        assert!(stage.contains("<h1>Stage 1</h1>"));
        assert!(stage.contains("class=\"active\" aria-current=\"page\""));
        assert!(stage.contains("<article>"));
        assert!(stage.contains("\"@type\":\"TechArticle\""));
        assert!(stage.contains("og:type\" content=\"article"));
        assert!(stage.contains("<meta name=\"description\" content=\"Reward text.\">"));
        assert!(stage.contains("<link rel=\"canonical\" href=\"https://simulation.lucanian.app/01-reward.html\">"));

        let sitemap = fs::read_to_string(out.join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("<loc>https://simulation.lucanian.app/</loc>"));
        assert!(sitemap.contains("01-reward.html"));
        let robots = fs::read_to_string(out.join("robots.txt")).unwrap();
        assert!(robots.contains("Sitemap: https://simulation.lucanian.app/sitemap.xml"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn descriptions_truncate_at_word_boundaries() {
        assert_eq!(truncate_words("short text", 200), "short text");
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen";
        let t = truncate_words(long, 40);
        assert!(t.chars().count() <= 40);
        assert!(!t.ends_with("eleve"));
        assert!(!long.starts_with(&t) || t.len() < long.len());
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
