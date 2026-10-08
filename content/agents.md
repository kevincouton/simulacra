# Agents & API: MCP and A2A Endpoints

Simulacra is not just a website: the same small Rust server that serves
this tracker exposes every playground experiment to other machines — as an
MCP tool server, as an A2A agent, and as plain JSON. All endpoints are
public, unauthenticated, rate-limited only by politeness, and seeded, so
every result is reproducible.

## MCP (Model Context Protocol)

`POST /mcp` speaks JSON-RPC 2.0 over stateless streamable HTTP. Six tools
are exposed: the four experiments — `run_town`, `run_variance`,
`run_coevolution`, `run_gossip`, each taking an optional integer `seed` —
plus `list_stages` and `get_stage` for the research documents. Point any
MCP client at the URL and it can run the playground itself.

- Endpoint: `https://simulation.lucanian.app/mcp`
- Initialize: send `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"you","version":"0"}}}`
- List tools: send `{"jsonrpc":"2.0","id":2,"method":"tools/list"}`
- Call a tool: send `{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_gossip","arguments":{"seed":42}}}`
- With curl: `curl -s https://simulation.lucanian.app/mcp -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_gossip","arguments":{"seed":42}}}'`

## A2A (Agent2Agent)

The same experiments are also an A2A agent. Its card is published at
`/.well-known/agent.json` (mirrored at `/a2a/agent.json`); send it a
natural-language `message/send` at `/a2a/` — "run gossip seed 7" — and it
replies with a completed Task whose artifacts carry both a human-readable
summary and the raw JSON result. `tasks/get` retrieves a task by id; tasks
persist across restarts in an append-only JSONL log.

- Agent card: `https://simulation.lucanian.app/.well-known/agent.json`
- Endpoint: `https://simulation.lucanian.app/a2a/`
- With curl: `curl -s https://simulation.lucanian.app/a2a/ -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"run variance seed 42"}]}}}'`

## Plain JSON API

For anything that does not speak agent protocols:

- `/api/town?seed=42` — one town run with the demo persona cast
- `/api/variance?seed=42` — the distributional-fidelity experiment
- `/api/coevolution?seed=42` — Synthesizer/Solver co-evolution rounds
- `/api/gossip?seed=42` — hidden-location runs with and without gossip

## Playground UI

Humans can drive the same four experiments in the browser:
[simulation.lucanian.app/app/](https://simulation.lucanian.app/app/)

## Source

The server is std-only Rust with a hand-rolled JSON layer — no
dependencies. `crates/server/src/mcp.rs` and `crates/server/src/a2a.rs` in
the [GitHub repository](https://github.com/kevincouton/simulacra) are the
whole implementation.
