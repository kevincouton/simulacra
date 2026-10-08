//! HTTP server for the Simulacra playground.
//!
//! Serves a small JSON API over the simulation engine and, when built,
//! the Nuxt web UI from `web/.output/public`:
//!
//! - `GET /api/town?seed=42` — one town run with the demo persona cast
//! - `GET /api/variance?seed=42` — the distributional-fidelity experiment
//! - `GET /api/coevolution?seed=42` — rounds of the Synthesizer/Solver game
//! - `GET /api/gossip?seed=42` — hidden-location runs with and without gossip
//! - anything else — static files from `web/.output/public` (404 if absent)
//!
//! Single-threaded and std-only; it is a playground server, not a
//! production one. Default bind: `127.0.0.1:8787` (override with
//! `SIMULACRA_BIND`).

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::rc::Rc;

use simulacra_agents::{ExplorerPersona, GossipBook, GossipingPersona, OraclePersona, Persona, ShopperPersona};
use simulacra_engine::{CoEvolution, Simulation, VarianceExperiment};

fn main() {
    let bind = std::env::var("SIMULACRA_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let root = workspace_root();
    let listener = match TcpListener::bind(&bind) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {bind}: {e}");
            std::process::exit(1);
        }
    };
    println!("simulacra server on http://{bind} (web root: {})", root.join("web/.output/public").display());
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle(s, &root) {
                    eprintln!("request error: {e}");
                }
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}

fn workspace_root() -> PathBuf {
    if let Ok(root) = std::env::var("SIMULACRA_ROOT") {
        return PathBuf::from(root);
    }
    let exe = std::env::current_exe().expect("cannot locate executable");
    exe.ancestors().nth(3).expect("cannot locate workspace root").to_path_buf()
}

fn handle(mut stream: TcpStream, root: &std::path::Path) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let Some(line) = request.lines().next() else {
        return respond(&mut stream, 400, "text/plain", "bad request");
    };
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    if method != "GET" {
        return respond(&mut stream, 405, "text/plain", "method not allowed");
    }

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };
    let seed = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("seed="))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);

    match path {
        "/api/town" => respond(&mut stream, 200, "application/json", &api_town(seed)),
        "/api/variance" => respond(&mut stream, 200, "application/json", &api_variance(seed)),
        "/api/coevolution" => respond(&mut stream, 200, "application/json", &api_coevolution(seed)),
        "/api/gossip" => respond(&mut stream, 200, "application/json", &api_gossip(seed)),
        "/healthz" => respond(&mut stream, 200, "text/plain", "ok"),
        other => serve_static(&mut stream, root, other),
    }
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())
}

fn serve_static(stream: &mut TcpStream, root: &std::path::Path, url_path: &str) -> std::io::Result<()> {
    let web_root = root.join("web").join(".output").join("public");
    let rel = url_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let file = web_root.join(rel);
    // Guard against path traversal.
    let canonical = file.canonicalize();
    let canonical = match canonical {
        Ok(c) if c.starts_with(&web_root) => c,
        _ => return respond(stream, 404, "text/plain", "not found"),
    };
    match std::fs::read(&canonical) {
        Ok(bytes) => {
            let ctype = if canonical.extension().is_some_and(|e| e == "css") {
                "text/css"
            } else if canonical.extension().is_some_and(|e| e == "js") {
                "application/javascript"
            } else {
                "text/html"
            };
            let body = String::from_utf8_lossy(&bytes).into_owned();
            respond(stream, 200, ctype, &body)
        }
        Err(_) => respond(stream, 404, "text/plain", "web UI not built — run `npx nuxi build` in web/ (or start the server after building)"),
    }
}

fn demo_cast() -> Vec<Box<dyn Persona>> {
    vec![
        Box::new(ShopperPersona::new("careful-clara", 0.95, 0.05)),
        Box::new(ShopperPersona::new("distracted-dan", 0.60, 0.20)),
        Box::new(ShopperPersona::new("lazy-lou", 0.50, 0.50)),
        Box::new(OraclePersona),
    ]
}

fn as_refs(cast: &[Box<dyn Persona>]) -> Vec<&dyn Persona> {
    cast.iter().map(|p| p.as_ref()).collect()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn api_town(seed: u64) -> String {
    let cast = demo_cast();
    let refs = as_refs(&cast);
    let report = Simulation::seeded(seed).run_town(5, &refs).unwrap();
    let results: String = report
        .results()
        .iter()
        .map(|r| {
            let violations: String = r
                .violations
                .iter()
                .map(|v| format!("\"{}\"", esc(v)))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"agent\":\"{}\",\"world_id\":{},\"task\":\"{}\",\"reward\":{},\"violations\":[{}]}}",
                esc(&r.agent),
                r.world_id,
                esc(&r.task),
                r.reward,
                violations
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"seed\":{seed},\"results\":[{}],\"success_rate\":{:.4},\"total_reward\":{:.1}}}",
        results,
        report.success_rate(),
        report.total_reward()
    )
}

fn api_variance(seed: u64) -> String {
    let report = VarianceExperiment::new(seed, 30, 20, 4).run().unwrap();
    let rates: String = report
        .per_run_rates
        .iter()
        .map(|r| format!("{r:.4}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"seed\":{seed},\"per_run_rates\":[{}],\"empirical_mean\":{:.4},\"theoretical_mean\":{:.4},\"empirical_variance\":{:.5},\"theoretical_variance\":{:.5},\"mean_match\":{},\"variance_match\":{}}}",
        rates,
        report.empirical_mean,
        report.theoretical_mean,
        report.empirical_variance,
        report.theoretical_variance,
        report.mean_match,
        report.variance_match
    )
}

fn api_coevolution(seed: u64) -> String {
    let personas = vec![
        ShopperPersona::new("careful-clara", 0.95, 0.05),
        ShopperPersona::new("distracted-dan", 0.60, 0.20),
        ShopperPersona::new("lazy-lou", 0.50, 0.50),
        ShopperPersona::new("steady-sue", 0.80, 0.10),
        ShopperPersona::new("hasty-hank", 0.70, 0.30),
        ShopperPersona::new("dreamy-dora", 0.55, 0.35),
    ];
    let refs: Vec<&dyn Persona> = personas.iter().map(|p| p as &dyn Persona).collect();
    let report = CoEvolution::seeded(seed).run(&refs, 8).unwrap();
    let rounds: String = report
        .rounds
        .iter()
        .map(|r| {
            format!(
                "{{\"round\":{},\"n_shops\":{},\"items_per_shop\":{},\"success_rate\":{:.4}}}",
                r.round, r.n_shops, r.items_per_shop, r.success_rate
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"seed\":{seed},\"rounds\":[{}]}}", rounds)
}

fn api_gossip(seed: u64) -> String {
    let mk = |trust: f64, book: Rc<RefCell<GossipBook>>| -> Vec<Box<dyn Persona>> {
        (0..6)
            .map(|i| {
                let base = ExplorerPersona::new(format!("explorer-{i}"), 0.1);
                if trust > 0.0 {
                    Box::new(GossipingPersona::new(base, book.clone(), trust)) as Box<dyn Persona>
                } else {
                    Box::new(base) as Box<dyn Persona>
                }
            })
            .collect()
    };
    let control_book = Rc::new(RefCell::new(GossipBook::default()));
    let plain = mk(0.0, control_book.clone());
    let plain_refs = as_refs(&plain);
    let without = Simulation::seeded(seed).run_town_hidden(4, &plain_refs, &control_book).unwrap();

    let book = Rc::new(RefCell::new(GossipBook::default()));
    let wired = mk(0.9, book.clone());
    let wired_refs = as_refs(&wired);
    let with = Simulation::seeded(seed).run_town_hidden(4, &wired_refs, &book).unwrap();

    format!(
        "{{\"seed\":{seed},\"no_gossip_rate\":{:.4},\"gossip_rate\":{:.4}}}",
        without.success_rate(),
        with.success_rate()
    )
}
