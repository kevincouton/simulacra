//! Minimal Markdown subset renderer for the tracker site.
//!
//! Supports what the research content uses: ATX headings (`#`–`###`),
//! paragraphs, unordered (`-`) and ordered (`1.`) lists, `> ` blockquotes,
//! `---` rules, inline `**bold**`, `*emphasis*`, `` `code` ``, and
//! `[text](url)` links. Anything else passes through escaped.

#![warn(missing_docs)]

/// Render a Markdown document to an HTML fragment (no `<html>` wrapper).
pub fn render_markdown(src: &str) -> String {
    let mut html = String::new();
    let mut list: Option<ListKind> = None;

    for line in src.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            close_list(&mut html, &mut list);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            close_list(&mut html, &mut list);
            html.push_str(&format!("<h3>{}</h3>\n", inline(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            close_list(&mut html, &mut list);
            html.push_str(&format!("<h2>{}</h2>\n", inline(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            close_list(&mut html, &mut list);
            html.push_str(&format!("<h1>{}</h1>\n", inline(rest)));
        } else if trimmed == "---" {
            close_list(&mut html, &mut list);
            html.push_str("<hr>\n");
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            close_list(&mut html, &mut list);
            html.push_str(&format!("<blockquote>{}</blockquote>\n", inline(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            if list != Some(ListKind::Unordered) {
                close_list(&mut html, &mut list);
                html.push_str("<ul>\n");
                list = Some(ListKind::Unordered);
            }
            html.push_str(&format!("<li>{}</li>\n", inline(rest)));
        } else if is_ordered_item(trimmed) {
            if list != Some(ListKind::Ordered) {
                close_list(&mut html, &mut list);
                html.push_str("<ol>\n");
                list = Some(ListKind::Ordered);
            }
            let rest = &trimmed[trimmed.find('.').unwrap() + 1..].trim_start();
            html.push_str(&format!("<li>{}</li>\n", inline(rest)));
        } else {
            close_list(&mut html, &mut list);
            html.push_str(&format!("<p>{}</p>\n", inline(trimmed)));
        }
    }
    close_list(&mut html, &mut list);
    html
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Unordered,
    Ordered,
}

fn close_list(html: &mut String, list: &mut Option<ListKind>) {
    match list.take() {
        Some(ListKind::Unordered) => html.push_str("</ul>\n"),
        Some(ListKind::Ordered) => html.push_str("</ol>\n"),
        None => {}
    }
}

fn is_ordered_item(line: &str) -> bool {
    let Some(dot) = line.find('.') else { return false };
    let (num, rest) = (&line[..dot], &line[dot + 1..]);
    !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) && rest.starts_with(' ')
}

/// Render inline Markdown: code, bold, emphasis, links, then HTML-escape
/// whatever structure did not match.
fn inline(src: &str) -> String {
    // Tokenize on `code` spans first so their content is never re-processed.
    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find('`') {
        out.push_str(&inline_no_code(&rest[..start]));
        match rest[start + 1..].find('`') {
            Some(end) => {
                let code = &rest[start + 1..start + 1 + end];
                out.push_str(&format!("<code>{}</code>", escape_html(code)));
                rest = &rest[start + 1 + end + 1..];
            }
            None => {
                out.push_str(&inline_no_code(&rest[start..]));
                rest = "";
            }
        }
    }
    out.push_str(&inline_no_code(rest));
    out
}

fn inline_no_code(src: &str) -> String {
    let mut s = escape_html(src);
    s = replace_pairs(&s, "**", "<strong>", "</strong>");
    s = replace_pairs(&s, "*", "<em>", "</em>");
    replace_links(&s)
}

/// Replace the first balanced pair of `delim` with open/close tags, repeat
/// until no pair remains. Delimiters may not span HTML tags introduced by
/// earlier replacements because `**` is processed before `*`.
fn replace_pairs(s: &str, delim: &str, open: &str, close: &str) -> String {
    let mut s = s.to_string();
    while let Some(start) = s.find(delim) {
        let after = &s[start + delim.len()..];
        let Some(rel_end) = after.find(delim) else { break };
        let end = start + delim.len() + rel_end;
        let inner = &s[start + delim.len()..end];
        let replacement = format!("{open}{inner}{close}");
        s = format!("{}{}{}", &s[..start], replacement, &s[end + delim.len()..]);
    }
    s
}

fn replace_links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(close) => {
                let text = &after[..close];
                let tail = &after[close + 1..];
                if let Some(url_part) = tail.strip_prefix("(") {
                    if let Some(end) = url_part.find(')') {
                        let url = &url_part[..end];
                        out.push_str(&format!("<a href=\"{url}\">{text}</a>"));
                        rest = &url_part[end + 1..];
                        continue;
                    }
                }
                out.push('[');
                rest = after;
            }
            None => {
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_and_paragraph() {
        let html = render_markdown("# Title\n\nSome text.\n");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Some text.</p>"));
    }

    #[test]
    fn unordered_and_ordered_lists() {
        let html = render_markdown("- a\n- b\n\n1. one\n2. two\n");
        assert!(html.contains("<ul>\n<li>a</li>\n<li>b</li>\n</ul>"));
        assert!(html.contains("<ol>\n<li>one</li>\n<li>two</li>\n</ol>"));
    }

    #[test]
    fn inline_formatting() {
        let html = render_markdown("a **bold** and *em* and `code x` end");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>em</em>"));
        assert!(html.contains("<code>code x</code>"));
    }

    #[test]
    fn link_rendering() {
        let html = render_markdown("see [Phi](https://example.com/phi) paper");
        assert!(html.contains("<a href=\"https://example.com/phi\">Phi</a>"));
    }

    #[test]
    fn html_is_escaped() {
        let html = render_markdown("a < b & c");
        assert!(html.contains("a &lt; b &amp; c"));
    }

    #[test]
    fn code_spans_escape_content() {
        let html = render_markdown("`<b>x</b>`");
        assert!(html.contains("<code>&lt;b&gt;x&lt;/b&gt;</code>"));
    }

    #[test]
    fn blockquote_and_rule() {
        let html = render_markdown("> quoted\n\n---\n");
        assert!(html.contains("<blockquote>quoted</blockquote>"));
        assert!(html.contains("<hr>"));
    }
}
