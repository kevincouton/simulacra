//! MCP (Model Context Protocol) endpoint: JSON-RPC 2.0 over HTTP at `/mcp`.
//!
//! Streamable-HTTP transport, stateless: every request is a single POST and
//! every response a single `application/json` body (no SSE stream, no
//! session ids — permitted for stateless servers). Implemented methods:
//! `initialize`, `ping`, `tools/list`, `tools/call`. The tools expose the
//! same four experiments as `/api/*` plus read access to the stage
//! documents behind the research tracker.

use std::path::Path;

use crate::json::{self, Json};
use crate::sim;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "simulacra";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handle one MCP request body; always returns a JSON-RPC response string.
pub fn handle(root: &Path, body: &str) -> String {
    let req = match json::parse(body) {
        Ok(r) => r,
        Err(_) => return error(Json::Null, -32700, "Parse error").to_string(),
    };
    let id = req.get("id").cloned().unwrap_or(Json::Null);
    let method = req.get("method").and_then(Json::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Null);

    match method {
        "initialize" => result(
            &id,
            Json::Obj(vec![
                ("protocolVersion".into(), Json::str(PROTOCOL_VERSION)),
                (
                    "capabilities".into(),
                    Json::Obj(vec![("tools".into(), Json::Obj(vec![]))]),
                ),
                (
                    "serverInfo".into(),
                    Json::Obj(vec![
                        ("name".into(), Json::str(SERVER_NAME)),
                        ("version".into(), Json::str(SERVER_VERSION)),
                    ]),
                ),
            ]),
        )
        .to_string(),
        "ping" => result(&id, Json::Obj(vec![])).to_string(),
        "tools/list" => result(&id, Json::Obj(vec![("tools".into(), tools(root))])).to_string(),
        "tools/call" => tools_call(root, &id, &params).to_string(),
        "" => error(id, -32600, "Invalid Request").to_string(),
        _ => error(id, -32601, &format!("Method not found: {method}")).to_string(),
    }
}

fn result(id: &Json, res: Json) -> Json {
    Json::Obj(vec![
        ("jsonrpc".into(), Json::str("2.0")),
        ("id".into(), id.clone()),
        ("result".into(), res),
    ])
}

fn error(id: Json, code: i64, msg: &str) -> Json {
    Json::Obj(vec![
        ("jsonrpc".into(), Json::str("2.0")),
        ("id".into(), id),
        (
            "error".into(),
            Json::Obj(vec![
                ("code".into(), Json::Num(code as f64)),
                ("message".into(), Json::str(msg)),
            ]),
        ),
    ])
}

fn seed_param(params: &Json) -> u64 {
    params
        .get("arguments")
        .and_then(|a| a.get("seed"))
        .and_then(Json::as_u64)
        .unwrap_or(sim::DEFAULT_SEED)
}

fn tool(name: &str, desc: &str, schema: Json) -> Json {
    Json::Obj(vec![
        ("name".into(), Json::str(name)),
        ("description".into(), Json::str(desc)),
        ("inputSchema".into(), schema),
    ])
}

fn seed_schema() -> Json {
    Json::Obj(vec![
        ("type".into(), Json::str("object")),
        (
            "properties".into(),
            Json::Obj(vec![(
                "seed".into(),
                Json::Obj(vec![
                    ("type".into(), Json::str("integer")),
                    ("description".into(), Json::str("random seed for the run")),
                ]),
            )]),
        ),
    ])
}

fn tools(root: &Path) -> Json {
    let mut list = vec![
        tool(
            "run_town",
            "Run one town simulation with the demo persona cast (3 shopper personas + 1 oracle) and per-agent rewards and constraint violations.",
            seed_schema(),
        ),
        tool(
            "run_variance",
            "Run the distributional-fidelity experiment: 30 populations of 20 agents, comparing empirical success-rate mean/variance against binomial theory.",
            seed_schema(),
        ),
        tool(
            "run_coevolution",
            "Run 8 rounds of the Synthesizer/Solver co-evolution game and report per-round success rates.",
            seed_schema(),
        ),
        tool(
            "run_gossip",
            "Compare hidden-location town success with and without the gossip social layer (6 explorer agents).",
            seed_schema(),
        ),
    ];
    list.push(tool(
        "list_stages",
        "List the research tracker's stage documents (slug and title).",
        Json::Obj(vec![("type".into(), Json::str("object"))]),
    ));
    list.push(tool(
        "get_stage",
        "Read one stage document as Markdown. Use \"index\" for the overview.",
        Json::Obj(vec![
            ("type".into(), Json::str("object")),
            (
                "properties".into(),
                Json::Obj(vec![(
                    "slug".into(),
                    Json::Obj(vec![
                        ("type".into(), Json::str("string")),
                        ("description".into(), Json::str("stage slug from list_stages, or \"index\"")),
                    ]),
                )]),
            ),
            ("required".into(), Json::Arr(vec![Json::str("slug")])),
        ]),
    ));
    let _ = root;
    Json::Arr(list)
}

fn call_ok(text: String) -> Json {
    Json::Obj(vec![
        (
            "content".into(),
            Json::Arr(vec![Json::Obj(vec![
                ("type".into(), Json::str("text")),
                ("text".into(), Json::Str(text)),
            ])]),
        ),
        ("isError".into(), Json::Bool(false)),
    ])
}

fn call_err(msg: &str) -> Json {
    Json::Obj(vec![
        (
            "content".into(),
            Json::Arr(vec![Json::Obj(vec![
                ("type".into(), Json::str("text")),
                ("text".into(), Json::str(msg)),
            ])]),
        ),
        ("isError".into(), Json::Bool(true)),
    ])
}

fn tools_call(root: &Path, id: &Json, params: &Json) -> Json {
    let name = match params.get("name").and_then(Json::as_str) {
        Some(n) => n,
        None => return error(id.clone(), -32602, "Missing params.name"),
    };
    let body = match name {
        "run_town" => call_ok(sim::run_town(seed_param(params))),
        "run_variance" => call_ok(sim::run_variance(seed_param(params))),
        "run_coevolution" => call_ok(sim::run_coevolution(seed_param(params))),
        "run_gossip" => call_ok(sim::run_gossip(seed_param(params))),
        "list_stages" => call_ok(list_stages(root)),
        "get_stage" => match params
            .get("arguments")
            .and_then(|a| a.get("slug"))
            .and_then(Json::as_str)
        {
            Some(slug) => match read_stage(root, slug) {
                Ok(md) => call_ok(md),
                Err(e) => call_err(&e),
            },
            None => return error(id.clone(), -32602, "Missing arguments.slug"),
        },
        _ => return error(id.clone(), -32602, &format!("Unknown tool: {name}")),
    };
    result(id, body)
}

/// Slugs safe to join onto `content/` — letters, digits, and dashes only.
fn safe_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn stage_titles(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let stages = root.join("content").join("stages");
    if let Ok(entries) = std::fs::read_dir(&stages) {
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for p in paths {
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
            if p.extension().and_then(|e| e.to_str()) != Some("md") || !safe_slug(stem) {
                continue;
            }
            let title = std::fs::read_to_string(&p)
                .ok()
                .and_then(|s| s.lines().find_map(|l| l.strip_prefix("# ")).map(|t| t.trim().to_string()))
                .unwrap_or_else(|| stem.to_string());
            out.push((stem.to_string(), title));
        }
    }
    out
}

fn list_stages(root: &Path) -> String {
    let items: Vec<String> = stage_titles(root)
        .into_iter()
        .map(|(slug, title)| format!("{{\"slug\":\"{slug}\",\"title\":\"{}\"}}", crate::json_escape(&title)))
        .collect();
    format!("[{}]", items.join(","))
}

fn read_stage(root: &Path, slug: &str) -> Result<String, String> {
    let path = if slug == "index" {
        root.join("content").join("index.md")
    } else if safe_slug(slug) {
        root.join("content").join("stages").join(format!("{slug}.md"))
    } else {
        return Err(format!("invalid slug: {slug}"));
    };
    std::fs::read_to_string(&path).map_err(|_| format!("unknown stage: {slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        crate::workspace_root()
    }

    #[test]
    fn initialize_negotiates() {
        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#)).unwrap();
        assert_eq!(res.path(&["result", "serverInfo", "name"]).unwrap().as_str(), Some("simulacra"));
        assert!(res.path(&["result", "capabilities", "tools"]).is_some());
    }

    #[test]
    fn lists_and_calls_tools() {
        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)).unwrap();
        let names: Vec<&str> = res
            .path(&["result", "tools"]).unwrap()
            .as_arr().unwrap()
            .iter()
            .filter_map(|t| t.get("name").and_then(Json::as_str))
            .collect();
        assert!(names.contains(&"run_gossip") && names.contains(&"get_stage"));

        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_gossip","arguments":{"seed":42}}}"#)).unwrap();
        assert_eq!(res.path(&["result", "isError"]), Some(&Json::Bool(false)));
        let text = res.path(&["result", "content", "0", "text"]).unwrap().as_str().unwrap();
        assert!(text.contains("\"gossip_rate\""));
    }

    #[test]
    fn reads_stage_documents() {
        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_stage","arguments":{"slug":"01-reward"}}}"#)).unwrap();
        let text = res.path(&["result", "content", "0", "text"]).unwrap().as_str().unwrap();
        assert!(text.starts_with("# Stage 1") || text.contains("Stage 1"));

        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_stage","arguments":{"slug":"../etc"}}}"#)).unwrap();
        assert_eq!(res.path(&["result", "isError"]), Some(&Json::Bool(true)));
    }

    #[test]
    fn errors_are_jsonrpc() {
        let res = json::parse(&handle(&root(), "{not json")).unwrap();
        assert_eq!(res.get("error").unwrap().get("code").unwrap().as_f64(), Some(-32700.0));
        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":9,"method":"bogus"}"#)).unwrap();
        assert_eq!(res.get("error").unwrap().get("code").unwrap().as_f64(), Some(-32601.0));
        let res = json::parse(&handle(&root(), r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"nope"}}"#)).unwrap();
        assert_eq!(res.get("error").unwrap().get("code").unwrap().as_f64(), Some(-32602.0));
    }
}
