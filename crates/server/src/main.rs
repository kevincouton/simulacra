//! HTTP server for the Simulacra playground and research tracker.
//!
//! Route map:
//!
//! - `/` and `/*.html`, `sitemap.xml`, `robots.txt`, `style.css` — the
//!   SEO/AEO research tracker (rebuilt from `content/` at startup)
//! - `/api/town?seed=42` — one town run with the demo persona cast
//! - `/api/variance?seed=42` — the distributional-fidelity experiment
//! - `/api/coevolution?seed=42` — rounds of the Synthesizer/Solver game
//! - `/api/gossip?seed=42` — hidden-location runs with and without gossip
//! - `POST /mcp` — MCP (Model Context Protocol) JSON-RPC endpoint exposing
//!   the experiments as tools (stateless streamable HTTP)
//! - `GET /.well-known/agent.json`, `GET /a2a/agent.json` — the A2A agent
//!   card; `POST /a2a/` — A2A (Agent2Agent) JSON-RPC endpoint
//! - `/app/*` — the Nuxt playground UI from `web/.output/public`
//!
//! Single-threaded and std-only; it is a playground server, not a
//! production one. Default bind: `127.0.0.1:31011` (override with
//! `SIMULACRA_BIND`).

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

mod a2a;
mod json;
mod mcp;
mod sim;

fn main() {
    let bind = std::env::var("SIMULACRA_BIND").unwrap_or_else(|_| "127.0.0.1:31011".into());
    let root = workspace_root();
    // Rebuild the research tracker from content at startup so the served
    // HTML is always current.
    if let Err(e) = simulacra_tracker::build_site(&root.join("content"), &root.join("site").join("dist")) {
        eprintln!("tracker rebuild failed: {e}");
        std::process::exit(1);
    }
    // A2A tasks persist in an append-only JSONL log under data/ so they
    // survive restarts; fall back to memory if the log cannot be created.
    let data_dir = root.join("data");
    let store = match std::fs::create_dir_all(&data_dir) {
        Ok(()) => a2a::Store::open(&data_dir.join("a2a-tasks.jsonl")),
        Err(e) => {
            eprintln!("cannot create {}, tasks will not persist: {e}", data_dir.display());
            a2a::Store::new()
        }
    };
    let listener = match TcpListener::bind(&bind) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {bind}: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "simulacra server on http://{bind} (tracker + /app/ UI + /api/ + /mcp + /a2a/)"
    );
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle(s, &root, &store) {
                    eprintln!("request error: {e}");
                }
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}

/// Locate the workspace root. Submodules and tests use this to find
/// `content/` regardless of the current directory.
pub fn workspace_root() -> PathBuf {
    if let Ok(root) = std::env::var("SIMULACRA_ROOT") {
        return PathBuf::from(root);
    }
    // Walk up from the executable looking for the workspace Cargo.toml,
    // so installed binaries (/usr/local/bin) work when run from the repo.
    let exe = std::env::current_exe().expect("cannot locate executable");
    for anc in exe.ancestors() {
        if anc.join("Cargo.toml").is_file() {
            return anc.to_path_buf();
        }
    }
    panic!("cannot locate workspace root — set SIMULACRA_ROOT");
}

/// Escape a string for embedding inside a JSON string literal (no quotes).
pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn handle(mut stream: TcpStream, root: &std::path::Path, store: &RefCell<a2a::Store>) -> std::io::Result<()> {
    let Some((method, target, body)) = read_request(&mut stream)? else {
        return respond(&mut stream, 400, "text/plain", "bad request", "");
    };

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target.as_str(), ""),
    };
    let seed = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("seed="))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(sim::DEFAULT_SEED);

    const CORS: &str = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type, mcp-session-id\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\n";
    match (method.as_str(), path) {
        ("GET", "/api/town") => respond(&mut stream, 200, "application/json", &sim::run_town(seed), ""),
        ("GET", "/api/variance") => respond(&mut stream, 200, "application/json", &sim::run_variance(seed), ""),
        ("GET", "/api/coevolution") => respond(&mut stream, 200, "application/json", &sim::run_coevolution(seed), ""),
        ("GET", "/api/gossip") => respond(&mut stream, 200, "application/json", &sim::run_gossip(seed), ""),
        ("POST", "/mcp") => respond(&mut stream, 200, "application/json", &mcp::handle(root, &body), CORS),
        ("POST", "/a2a") | ("POST", "/a2a/") => {
            respond(&mut stream, 200, "application/json", &a2a::handle(root, store, &body), CORS)
        }
        ("OPTIONS", "/mcp") | ("OPTIONS", "/a2a") | ("OPTIONS", "/a2a/") => {
            respond(&mut stream, 204, "text/plain", "", CORS)
        }
        ("GET", "/.well-known/agent.json") | ("GET", "/a2a/agent.json") => {
            respond(&mut stream, 200, "application/json", &a2a::agent_card(), CORS)
        }
        ("GET", "/healthz") => respond(&mut stream, 200, "text/plain", "ok", ""),
        ("GET", _) => serve_static(&mut stream, root, path),
        _ => respond(&mut stream, 405, "text/plain", "method not allowed", ""),
    }
}

/// Read one HTTP request: headers up to `\r\n\r\n`, then the body per
/// Content-Length. Returns `Ok(None)` on a malformed or over-long request.
fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<(String, String, String)>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > 65_536 {
            return Ok(None);
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("").to_string();
    let content_length = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < header_end + content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 1_000_000 {
            return Ok(None);
        }
    }
    let end = (header_end + content_length).min(buf.len());
    let body = String::from_utf8_lossy(&buf[header_end..end]).into_owned();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    Ok(Some((method, target, body)))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
    extra_headers: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())
}

/// Map a URL path to a file under either the web UI (`web/.output/public`,
/// under `/app/`) or the research tracker (`site/dist`, the default),
/// with a traversal guard.
fn serve_static(stream: &mut TcpStream, root: &std::path::Path, url_path: &str) -> std::io::Result<()> {
    let (base, rel) = if let Some(rest) = url_path.strip_prefix("/app") {
        (root.join("web").join(".output").join("public"), rest.trim_start_matches('/'))
    } else {
        (root.join("site").join("dist"), url_path.trim_start_matches('/'))
    };
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let file = base.join(rel);
    // Guard against path traversal.
    let canonical = file.canonicalize();
    let canonical = match canonical {
        Ok(c) if c.starts_with(&base) => c,
        _ => return respond(stream, 404, "text/plain", "not found", ""),
    };
    match std::fs::read(&canonical) {
        Ok(bytes) => {
            let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");
            let ctype = match ext {
                "css" => "text/css",
                "js" => "application/javascript",
                "xml" => "application/xml",
                "txt" => "text/plain",
                "svg" => "image/svg+xml",
                _ => "text/html",
            };
            let body = String::from_utf8_lossy(&bytes).into_owned();
            respond(stream, 200, ctype, &body, "")
        }
        Err(_) => respond(stream, 404, "text/plain", "not found", ""),
    }
}
