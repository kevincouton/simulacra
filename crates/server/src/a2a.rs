//! A2A (Agent2Agent) endpoint: a public agent at `/a2a/` plus its Agent Card
//! at `/.well-known/agent.json` (also mirrored at `/a2a/agent.json`).
//!
//! JSON-RPC 2.0 methods: `message/send` maps a natural-language request onto
//! one of the four experiments and returns a completed `Task` whose
//! artifacts carry both a human-readable summary and the raw JSON result;
//! `tasks/get` fetches a previously created task. Tasks persist across
//! restarts in an append-only JSONL log (see [`Store::open`]).
//! Streaming is not supported (the card advertises `streaming: false`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;

use crate::json::{self, Json};
use crate::sim;

pub const PROTOCOL_VERSION: &str = "0.3.0";
pub const V1_PROTOCOL_VERSION: &str = "1.0";

/// Task store backed by an append-only JSONL log. The server is
/// single-threaded, so a `RefCell` map behind the listener loop is enough
/// for the in-memory index; the log handle is append-only, one task per
/// line (the serializer escapes newlines, so lines never break).
pub struct Store {
    tasks: HashMap<String, Json>,
    next_id: u64,
    log: Option<std::fs::File>,
}

impl Store {
    /// In-memory store only (used by tests and as a fallback).
    pub fn new() -> RefCell<Self> {
        RefCell::new(Self {
            tasks: HashMap::new(),
            next_id: 0,
            log: None,
        })
    }

    /// Load tasks from `path` (created if missing) and append new ones to
    /// it. Unreadable or corrupt lines are skipped, and a log that fails
    /// to open degrades to in-memory-only operation.
    pub fn open(path: &Path) -> RefCell<Self> {
        let mut tasks = HashMap::new();
        let mut next_id = 0u64;
        if let Ok(contents) = std::fs::read_to_string(path) {
            for line in contents.lines() {
                let Ok(task) = json::parse(line) else {
                    continue;
                };
                let Some(id) = task.get("id").and_then(Json::as_str) else {
                    continue;
                };
                if let Some(n) = id
                    .strip_prefix("simulacra-task-")
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    next_id = next_id.max(n);
                }
                tasks.insert(id.to_string(), task);
            }
        }
        let log = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(f) => Some(f),
            Err(e) => {
                eprintln!(
                    "a2a task log {} not writable, tasks will not persist: {e}",
                    path.display()
                );
                None
            }
        };
        RefCell::new(Self {
            tasks,
            next_id,
            log,
        })
    }

    /// Store a task, inserting its freshly minted id as the first field.
    fn add(&mut self, mut task: Json) -> String {
        self.next_id += 1;
        let id = format!("simulacra-task-{}", self.next_id);
        if let Json::Obj(pairs) = &mut task {
            pairs.insert(0, ("id".into(), Json::Str(id.clone())));
        }
        if let Some(log) = &mut self.log {
            if let Err(e) = writeln!(log, "{}", task.to_string()) {
                eprintln!("a2a task log write failed: {e}");
            }
        }
        self.tasks.insert(id.clone(), task);
        id
    }

    fn get(&self, id: &str) -> Option<Json> {
        self.tasks.get(id).cloned()
    }
}

pub fn agent_card() -> String {
    let url = simulacra_tracker::BASE_URL;
    let skill = |id: &str, name: &str, desc: &str, tags: &[&str]| {
        Json::Obj(vec![
            ("id".into(), Json::str(id)),
            ("name".into(), Json::str(name)),
            ("description".into(), Json::str(desc)),
            (
                "tags".into(),
                Json::Arr(tags.iter().map(|t| Json::str(*t)).collect()),
            ),
        ])
    };
    Json::Obj(vec![
        ("protocolVersion".into(), Json::str(PROTOCOL_VERSION)),
        ("name".into(), Json::str("Simulacra Research Agent")),
        (
            "description".into(),
            Json::str(
                "Runs the Simulacra playground experiments: seeded town simulations with heuristic personas, the distributional-fidelity variance experiment, Synthesizer/Solver co-evolution rounds, and hidden-location runs with and without a gossip layer. Ask it to run an experiment by name, optionally with a seed.",
            ),
        ),
        ("url".into(), Json::str(format!("{url}/a2a/"))),
        (
            "provider".into(),
            Json::Obj(vec![("organization".into(), Json::str("Simulacra"))]),
        ),
        ("version".into(), Json::str(env!("CARGO_PKG_VERSION"))),
        (
            "capabilities".into(),
            Json::Obj(vec![
                ("streaming".into(), Json::Bool(false)),
                ("pushNotifications".into(), Json::Bool(false)),
            ]),
        ),
        (
            "authentication".into(),
            Json::Obj(vec![("schemes".into(), Json::Arr(vec![]))]),
        ),
        (
            "defaultInputModes".into(),
            Json::Arr(vec![Json::str("text")]),
        ),
        (
            "defaultOutputModes".into(),
            Json::Arr(vec![Json::str("text")]),
        ),
        (
            "skills".into(),
            Json::Arr(vec![
                skill("overview", "Overview", "What Simulacra is and which experiments are available.", &["simulation", "overview"]),
                skill("run-town", "Town simulation", "Run a seeded town: 5 worlds, the demo persona cast, per-agent rewards and violations.", &["simulation", "town", "personas"]),
                skill("run-variance", "Variance experiment", "30 populations of 20 agents; empirical mean/variance checked against binomial theory.", &["simulation", "statistics", "fidelity"]),
                skill("run-coevolution", "Co-evolution", "8 rounds of the Synthesizer/Solver game with per-round success rates.", &["simulation", "coevolution"]),
                skill("run-gossip", "Gossip experiment", "Hidden-location towns with and without gossip (6 explorer agents).", &["simulation", "gossip", "social"]),
            ]),
        ),
    ])
    .to_string()
}

/// A2A 1.0 discovery card. The 0.3 card remains available unchanged.
pub fn agent_card_v1() -> String {
    let mut card = json::parse(&agent_card()).unwrap_or(Json::Null);
    if let Json::Obj(pairs) = &mut card {
        pairs.retain(|(key, _)| {
            !matches!(key.as_str(), "protocolVersion" | "url" | "authentication")
        });
        for (key, value) in pairs.iter_mut() {
            if key == "defaultInputModes" || key == "defaultOutputModes" {
                *value = Json::Arr(vec![Json::str("text/plain")]);
            }
        }
        pairs.push((
            "supportedInterfaces".into(),
            Json::Arr(vec![Json::Obj(vec![
                (
                    "url".into(),
                    Json::str(format!("{}/a2a/v1", simulacra_tracker::BASE_URL)),
                ),
                ("protocolBinding".into(), Json::str("JSONRPC")),
                ("protocolVersion".into(), Json::str(V1_PROTOCOL_VERSION)),
            ])]),
        ));
    }
    card.to_string()
}

/// Handle one A2A JSON-RPC request body; always returns a response string.
pub fn handle(_root: &Path, store: &RefCell<Store>, body: &str) -> String {
    let req = match json::parse(body) {
        Ok(r) => r,
        Err(_) => return error(Json::Null, -32700, "Parse error").to_string(),
    };
    let id = req.get("id").cloned().unwrap_or(Json::Null);
    let method = req.get("method").and_then(Json::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Json::Null);

    match method {
        "message/send" => message_send(store, &id, &params).to_string(),
        "tasks/get" => match params.get("id").and_then(Json::as_str) {
            Some(tid) => match store.borrow().get(tid) {
                Some(task) => result(&id, task).to_string(),
                None => error(id, -32001, "Task not found").to_string(),
            },
            None => error(id, -32602, "Missing params.id").to_string(),
        },
        "" => error(id, -32600, "Invalid Request").to_string(),
        _ => error(id, -32601, &format!("Method not found: {method}")).to_string(),
    }
}

/// Select the representation from `A2A-Version`. Absence means 0.3, as
/// required by the specification; v1 tasks are converted at the boundary so
/// existing persisted tasks and legacy clients keep working.
pub fn handle_http(
    root: &Path,
    store: &RefCell<Store>,
    body: &str,
    version: Option<&str>,
) -> (u16, String) {
    match version.unwrap_or("0.3") {
        "0.3" => (200, handle(root, store, body)),
        V1_PROTOCOL_VERSION => {
            let response = handle(root, store, body);
            let mut parsed = json::parse(&response).unwrap_or(Json::Null);
            if let Json::Obj(pairs) = &mut parsed {
                if let Some((_, task)) = pairs.iter_mut().find(|(key, _)| key == "result") {
                    *task = task_v1(task.clone());
                }
            }
            (200, parsed.to_string())
        }
        requested => {
            let id = json::parse(body)
                .ok()
                .and_then(|req| req.get("id").cloned())
                .unwrap_or(Json::Null);
            let mut response = error(id, -32009, "A2A version not supported");
            if let Json::Obj(pairs) = &mut response {
                if let Some((_, Json::Obj(fields))) =
                    pairs.iter_mut().find(|(key, _)| key == "error")
                {
                    fields.push((
                        "data".into(),
                        Json::Obj(vec![
                            (
                                "supported".into(),
                                Json::Arr(vec![Json::str("0.3"), Json::str(V1_PROTOCOL_VERSION)]),
                            ),
                            ("requested".into(), Json::str(requested)),
                        ]),
                    ));
                }
            }
            (400, response.to_string())
        }
    }
}

fn task_v1(task: Json) -> Json {
    let Json::Obj(mut pairs) = task else {
        return task;
    };
    pairs.retain(|(key, _)| key != "kind");
    for (_, value) in &mut pairs {
        convert_task_field(value);
    }
    Json::Obj(pairs)
}

fn convert_task_field(value: &mut Json) {
    let Json::Obj(pairs) = value else {
        return;
    };
    for (key, field) in pairs {
        match key.as_str() {
            "state" => {
                if let Some(state) = field.as_str() {
                    *field = Json::str(match state {
                        "completed" => "TASK_STATE_COMPLETED",
                        "failed" => "TASK_STATE_FAILED",
                        "canceled" => "TASK_STATE_CANCELED",
                        "rejected" => "TASK_STATE_REJECTED",
                        "working" => "TASK_STATE_WORKING",
                        "submitted" => "TASK_STATE_SUBMITTED",
                        "input-required" => "TASK_STATE_INPUT_REQUIRED",
                        "auth-required" => "TASK_STATE_AUTH_REQUIRED",
                        other => other,
                    });
                }
            }
            "message" => convert_message(field),
            "history" => {
                if let Json::Arr(messages) = field {
                    for message in messages {
                        convert_message(message);
                    }
                }
            }
            "artifacts" => {
                if let Json::Arr(artifacts) = field {
                    for artifact in artifacts {
                        if let Some(parts) = artifact.get("parts").cloned() {
                            let converted = convert_parts(&parts);
                            if let Json::Obj(fields) = artifact {
                                if let Some((_, current)) =
                                    fields.iter_mut().find(|(key, _)| key == "parts")
                                {
                                    *current = converted;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn convert_message(message: &mut Json) {
    let Json::Obj(pairs) = message else {
        return;
    };
    if !pairs.iter().any(|(key, _)| key == "messageId") {
        pairs.push(("messageId".into(), Json::str("simulacra-request")));
    }
    for (key, field) in pairs {
        match key.as_str() {
            "role" => {
                if let Some(role) = field.as_str() {
                    *field = Json::str(match role {
                        "user" => "ROLE_USER",
                        "agent" => "ROLE_AGENT",
                        other => other,
                    });
                }
            }
            "parts" => *field = convert_parts(field),
            _ => {}
        }
    }
}

fn convert_parts(parts: &Json) -> Json {
    let Json::Arr(items) = parts else {
        return parts.clone();
    };
    Json::Arr(
        items
            .iter()
            .map(|part| {
                let Json::Obj(fields) = part else {
                    return part.clone();
                };
                Json::Obj(
                    fields
                        .iter()
                        .filter(|(key, _)| key != "type")
                        .cloned()
                        .collect(),
                )
            })
            .collect(),
    )
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

fn text_part(text: String) -> Json {
    Json::Obj(vec![
        ("type".into(), Json::str("text")),
        ("text".into(), Json::Str(text)),
    ])
}

fn data_part(data: Json) -> Json {
    Json::Obj(vec![
        ("type".into(), Json::str("data")),
        ("data".into(), data),
    ])
}

fn message_send(store: &RefCell<Store>, id: &Json, params: &Json) -> Json {
    let msg = match params.get("message") {
        Some(m) => m,
        None => return error(id.clone(), -32602, "Missing params.message"),
    };
    let text: String = msg
        .get("parts")
        .and_then(Json::as_arr)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Json::as_str))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        return error(id.clone(), -32602, "Message has no text parts");
    }

    let (summary, data) = run_intent(&text);
    let user_msg = msg.clone();
    let agent_msg = Json::Obj(vec![
        ("role".into(), Json::str("agent")),
        ("parts".into(), Json::Arr(vec![text_part(summary.clone())])),
        ("messageId".into(), Json::str("simulacra-reply")),
    ]);
    let task = Json::Obj(vec![
        ("kind".into(), Json::str("task")),
        ("contextId".into(), Json::str("simulacra-context")),
        (
            "status".into(),
            Json::Obj(vec![
                ("state".into(), Json::str("completed")),
                ("message".into(), agent_msg.clone()),
            ]),
        ),
        (
            "artifacts".into(),
            Json::Arr(vec![Json::Obj(vec![
                ("artifactId".into(), Json::str("simulacra-result")),
                ("name".into(), Json::str("experiment-result")),
                (
                    "parts".into(),
                    Json::Arr(vec![text_part(summary), data_part(data)]),
                ),
            ])]),
        ),
        ("history".into(), Json::Arr(vec![user_msg, agent_msg])),
        ("metadata".into(), Json::Obj(vec![])),
    ]);
    let task_id = store.borrow_mut().add(task);
    let task = store.borrow().get(&task_id).unwrap();
    result(id, task)
}

/// Map free text onto an experiment; unrecognized text gets the overview.
fn run_intent(text: &str) -> (String, Json) {
    let lower = text.to_lowercase();
    let seed = extract_seed(&lower).unwrap_or(sim::DEFAULT_SEED);
    if lower.contains("gossip") {
        let data = json::parse(&sim::run_gossip(seed)).unwrap_or(Json::Null);
        let a = data
            .get("no_gossip_rate")
            .and_then(Json::as_f64)
            .unwrap_or(f64::NAN);
        let b = data
            .get("gossip_rate")
            .and_then(Json::as_f64)
            .unwrap_or(f64::NAN);
        (format!("Gossip experiment at seed {seed}: hidden-location success is {a:.4} without gossip and {b:.4} with gossip."), data)
    } else if lower.contains("variance") {
        let data = json::parse(&sim::run_variance(seed)).unwrap_or(Json::Null);
        let m = data.path(&["mean_match"]) == Some(&Json::Bool(true));
        let v = data.path(&["variance_match"]) == Some(&Json::Bool(true));
        (format!("Variance experiment at seed {seed}: empirical mean/variance match theory: mean={m}, variance={v}."), data)
    } else if lower.contains("coevolution")
        || lower.contains("co-evolution")
        || lower.contains("synthesizer")
    {
        let data = json::parse(&sim::run_coevolution(seed)).unwrap_or(Json::Null);
        let n = data
            .get("rounds")
            .and_then(Json::as_arr)
            .map(|r| r.len())
            .unwrap_or(0);
        (format!("Co-evolution at seed {seed}: ran {n} Synthesizer/Solver rounds; see the artifact for per-round success rates."), data)
    } else if lower.contains("town") || lower.contains("shop") || lower.contains("persona") {
        let data = json::parse(&sim::run_town(seed)).unwrap_or(Json::Null);
        let rate = data
            .get("success_rate")
            .and_then(Json::as_f64)
            .unwrap_or(f64::NAN);
        (format!("Town simulation at seed {seed}: success rate {rate:.4} across 5 worlds; see the artifact for per-agent results."), data)
    } else {
        let data = Json::Obj(vec![("seed".into(), Json::Num(seed as f64))]);
        ("I can run four Simulacra experiments: a town simulation (\"run town\"), the distributional-fidelity variance experiment (\"run variance\"), Synthesizer/Solver co-evolution (\"run coevolution\"), and the gossip comparison (\"run gossip\"). Append a seed, e.g. \"run gossip seed 7\".".to_string(), data)
    }
}

fn extract_seed(lower: &str) -> Option<u64> {
    let pos = lower.find("seed")?;
    let digits: String = lower[pos + 4..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        crate::workspace_root()
    }

    #[test]
    fn card_is_valid_and_complete() {
        let card = json::parse(&agent_card()).unwrap();
        assert_eq!(
            card.get("name").unwrap().as_str(),
            Some("Simulacra Research Agent")
        );
        assert!(card
            .get("url")
            .unwrap()
            .as_str()
            .unwrap()
            .ends_with("/a2a/"));
        assert!(card.path(&["capabilities", "streaming"]) == Some(&Json::Bool(false)));
        assert!(card.get("skills").unwrap().as_arr().unwrap().len() >= 5);
    }

    #[test]
    fn message_send_runs_experiment() {
        let store = Store::new();
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"please run gossip with seed 42"}]}}}"#;
        let res = json::parse(&handle(&root(), &store, req)).unwrap();
        let task = res.get("result").unwrap();
        assert_eq!(
            task.path(&["status", "state"]).unwrap().as_str(),
            Some("completed")
        );
        let text = task
            .path(&["status", "message", "parts", "0", "text"])
            .unwrap()
            .as_str()
            .unwrap();
        assert!(
            text.contains("0.5000") || text.contains("gossip"),
            "unexpected summary: {text}"
        );
        assert!(task
            .path(&["artifacts", "0", "parts", "1", "data", "gossip_rate"])
            .is_some());
        let id = task.get("id").unwrap().as_str().unwrap().to_string();

        let res = json::parse(&handle(
            &root(),
            &store,
            &format!(r#"{{"jsonrpc":"2.0","id":2,"method":"tasks/get","params":{{"id":"{id}"}}}}"#),
        ))
        .unwrap();
        assert_eq!(
            res.get("result").unwrap().get("id").unwrap().as_str(),
            Some(id.as_str())
        );
    }

    #[test]
    fn unknown_skill_gets_overview() {
        let store = Store::new();
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"hello there"}]}}}"#;
        let res = json::parse(&handle(&root(), &store, req)).unwrap();
        let text = res
            .path(&["result", "status", "message", "parts", "0", "text"])
            .unwrap()
            .as_str()
            .unwrap();
        assert!(
            text.contains("variance"),
            "overview should list experiments: {text}"
        );
    }

    #[test]
    fn seed_extraction() {
        assert_eq!(extract_seed("run gossip seed 7"), Some(7));
        assert_eq!(extract_seed("seed=99 run town"), Some(99));
        assert_eq!(extract_seed("no seed here"), None);
    }

    #[test]
    fn tasks_persist_across_reopen() {
        let tmp = std::env::temp_dir().join("simulacra-a2a-persist-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let log = tmp.join("tasks.jsonl");

        let store = Store::open(&log);
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"run gossip seed 42"}]}}}"#;
        let res = json::parse(&handle(&root(), &store, req)).unwrap();
        let id = res
            .path(&["result", "id"])
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        drop(store);

        // Reopen from the log: the task must still be there, and new ids
        // must not collide with replayed ones.
        let store = Store::open(&log);
        let res = json::parse(&handle(
            &root(),
            &store,
            &format!(r#"{{"jsonrpc":"2.0","id":2,"method":"tasks/get","params":{{"id":"{id}"}}}}"#),
        ))
        .unwrap();
        assert_eq!(
            res.path(&["result", "id"]).unwrap().as_str(),
            Some(id.as_str())
        );

        let req = r#"{"jsonrpc":"2.0","id":3,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"run town"}]}}}"#;
        let res = json::parse(&handle(&root(), &store, req)).unwrap();
        let id2 = res
            .path(&["result", "id"])
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(id, id2);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn corrupt_log_lines_are_skipped() {
        let tmp = std::env::temp_dir().join("simulacra-a2a-corrupt-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let log = tmp.join("tasks.jsonl");
        std::fs::write(
            &log,
            "not json\n{\"id\":\"simulacra-task-5\",\"kind\":\"task\"}\n",
        )
        .unwrap();

        let store = Store::open(&log);
        let res = json::parse(&handle(
            &root(),
            &store,
            r#"{"jsonrpc":"2.0","id":1,"method":"tasks/get","params":{"id":"simulacra-task-5"}}"#,
        ))
        .unwrap();
        assert!(res.get("result").is_some());

        // New ids continue after the highest replayed id.
        let req = r#"{"jsonrpc":"2.0","id":2,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"hi"}]}}}"#;
        let res = json::parse(&handle(&root(), &store, req)).unwrap();
        assert_eq!(
            res.path(&["result", "id"]).unwrap().as_str(),
            Some("simulacra-task-6")
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn v1_card_uses_supported_interfaces_and_mime_modes() {
        let card = json::parse(&agent_card_v1()).unwrap();
        assert_eq!(
            card.path(&["supportedInterfaces", "0", "protocolVersion"])
                .and_then(Json::as_str),
            Some("1.0")
        );
        assert_eq!(
            card.path(&["supportedInterfaces", "0", "protocolBinding"])
                .and_then(Json::as_str),
            Some("JSONRPC")
        );
        assert!(card
            .path(&["supportedInterfaces", "0", "url"])
            .and_then(Json::as_str)
            .unwrap()
            .ends_with("/a2a/v1"));
        assert_eq!(
            card.path(&["defaultInputModes", "0"])
                .and_then(Json::as_str),
            Some("text/plain")
        );
        assert!(card.get("protocolVersion").is_none());
    }

    #[test]
    fn v1_messages_and_retrieved_legacy_tasks_use_v1_shapes() {
        let store = Store::new();
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"ROLE_USER","messageId":"request-1","parts":[{"text":"run gossip seed 42"}]}}}"#;
        let (status, response) = handle_http(&root(), &store, req, Some("1.0"));
        assert_eq!(status, 200);
        let res = json::parse(&response).unwrap();
        let task = res.get("result").unwrap();
        assert!(task.get("kind").is_none());
        assert_eq!(
            task.path(&["status", "state"]).and_then(Json::as_str),
            Some("TASK_STATE_COMPLETED")
        );
        assert_eq!(
            task.path(&["status", "message", "role"])
                .and_then(Json::as_str),
            Some("ROLE_AGENT")
        );
        assert!(task
            .path(&["status", "message", "parts", "0", "type"])
            .is_none());
        assert!(task
            .path(&["artifacts", "0", "parts", "1", "data", "gossip_rate"])
            .is_some());
        let id = task.get("id").and_then(Json::as_str).unwrap();

        // Create through the legacy API, then retrieve through v1.
        let legacy_req = r#"{"jsonrpc":"2.0","id":2,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"run town"}]}}}"#;
        let legacy = json::parse(&handle(&root(), &store, legacy_req)).unwrap();
        let legacy_id = legacy
            .path(&["result", "id"])
            .and_then(Json::as_str)
            .unwrap();
        assert_ne!(id, legacy_id);
        let get = format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"tasks/get","params":{{"id":"{legacy_id}"}}}}"#
        );
        let (_, response) = handle_http(&root(), &store, &get, Some("1.0"));
        let fetched = json::parse(&response).unwrap();
        assert_eq!(
            fetched
                .path(&["result", "status", "state"])
                .and_then(Json::as_str),
            Some("TASK_STATE_COMPLETED")
        );
        assert!(fetched.path(&["result", "kind"]).is_none());
    }

    #[test]
    fn unsupported_a2a_version_has_protocol_error() {
        let store = Store::new();
        let (status, response) = handle_http(
            &root(),
            &store,
            r#"{"jsonrpc":"2.0","id":9,"method":"tasks/get","params":{"id":"x"}}"#,
            Some("2.0"),
        );
        assert_eq!(status, 400);
        let res = json::parse(&response).unwrap();
        assert_eq!(
            res.path(&["error", "code"]).and_then(Json::as_f64),
            Some(-32009.0)
        );
        assert_eq!(
            res.path(&["error", "data", "requested"])
                .and_then(Json::as_str),
            Some("2.0")
        );
    }
}
