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
plus `list_stages` and `get_stage` for the research documents.

Both the legacy `2025-06-18` handshake and the current `2026-07-28`
per-request metadata model are supported. Modern clients can call
`server/discover`; `tools/list` responses include public cache metadata.
The modern transport requires `MCP-Protocol-Version` and `Mcp-Method` on
every request, plus `Mcp-Name` for tool calls, and rejects header/body
mismatches.

- Endpoint: `https://simulation.lucanian.app/mcp`
- Legacy initialize: send `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"you","version":"0"}}}`
- Legacy list tools: send `{"jsonrpc":"2.0","id":2,"method":"tools/list"}`
- Legacy curl: `curl -s https://simulation.lucanian.app/mcp -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_gossip","arguments":{"seed":42}}}'`

A modern `tools/call` body places protocol metadata under `params._meta`:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "run_gossip",
    "arguments": {"seed": 42},
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientCapabilities": {},
      "io.modelcontextprotocol/clientInfo": {"name": "you", "version": "1"}
    }
  }
}
```

Send it with headers `MCP-Protocol-Version: 2026-07-28`,
`Mcp-Method: tools/call`, and `Mcp-Name: run_gossip`.

## A2A (Agent2Agent)

The same experiments are also an A2A agent. Send a natural-language
`message/send` — "run gossip seed 7" — and it replies with a completed Task
whose artifacts carry both a human-readable summary and the raw JSON result.
`tasks/get` retrieves a task by id; tasks persist across restarts in a
bounded JSONL log. `SIMULACRA_A2A_MAX_TASKS` sets the maximum task count
(default 1000), and `SIMULACRA_A2A_MAX_LOG_BYTES` sets the target log size
(default 10 MiB). When either limit is exceeded, the newest tasks are
retained in their original order through same-directory atomic compaction.
Task IDs continue increasing after compaction and restart. At least the
newest valid task is always retained, so the byte limit is a soft cap if
that task alone is larger. A failed compaction leaves the original log and
all in-memory tasks intact.

A2A 0.3 remains available at `/a2a/` with its original card and task shape.
A2A 1.0 is served in parallel at `/a2a/v1`; send `A2A-Version: 1.0` and use
`SendMessage`/`GetTask` (the legacy method names remain accepted), uppercase
roles and task states, MIME media modes, and wrapper-only text or data parts.
`SendMessage` returns the standard `result.task` envelope; `GetTask` returns
a task directly. The server converts persisted legacy tasks at the boundary,
so either protocol version can retrieve them.

- Legacy agent card: `https://simulation.lucanian.app/.well-known/agent.json`
- A2A 1.0 agent card: `https://simulation.lucanian.app/.well-known/agent-card.json`
- Legacy endpoint: `https://simulation.lucanian.app/a2a/`
- A2A 1.0 endpoint: `https://simulation.lucanian.app/a2a/v1`
- Legacy curl: `curl -s https://simulation.lucanian.app/a2a/ -H 'content-type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"message/send","params":{"message":{"role":"user","parts":[{"type":"text","text":"run variance seed 42"}]}}}'`
- A2A 1.0 curl: `curl -s https://simulation.lucanian.app/a2a/v1 -H 'content-type: application/json' -H 'A2A-Version: 1.0' -d '{"jsonrpc":"2.0","id":1,"method":"SendMessage","params":{"message":{"role":"ROLE_USER","messageId":"request-1","parts":[{"text":"run variance seed 42"}]}}}'`

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
