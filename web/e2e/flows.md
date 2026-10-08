# End-user flows under test

The e2e suite (`*.spec.ts` next to this file) simulates five personas
against one deployment. Default target is the live site
(`https://simulation.lucanian.app`); override with
`PLAYWRIGHT_BASE_URL=http://localhost:31011` for a local server.

## Persona 1 — The researcher (human, browser)

Uses the playground UI at `/app/` to run and share experiments.

| # | Flow | Covered by |
|---|------|-----------|
| 1.1 | Land on `/app/`: title, four sections, nav anchors scroll | `playground.spec.ts` › landing |
| 1.2 | Town run: type seed 42, run, read the per-agent table and summary | › town run |
| 1.3 | Variance run: 30 bars render, verdict badges and DISTRIBUTION banner | › variance run |
| 1.4 | Co-evolution run: 8 rounds with inline bars | › co-evolution run |
| 1.5 | Gossip run: two cards and the delta conclusion | › gossip run |
| 1.6 | Determinism: same seed twice → identical output | › determinism |
| 1.7 | Shareable link: `/app/?section=gossip&seed=7&run=1` auto-runs and matches a manual seed-7 run | › shareable deep link |
| 1.8 | Failure path: API down → the "start the server" placeholder appears instead of stale data | › api failure state |

## Persona 2 — The integrator (JSON API)

Embeds the raw results in their own tooling via `/api/*`.

| # | Flow | Covered by |
|---|------|-----------|
| 2.1 | Each endpoint returns 200 + expected fields; repeated calls with one seed are byte-identical | `api.spec.ts` |
| 2.2 | Unknown path → 404; wrong method → 405 | › error handling |

## Persona 3 — The agent builder (MCP)

Wires the experiments into an LLM client as MCP tools at `POST /mcp`.

| # | Flow | Covered by |
|---|------|-----------|
| 3.1 | `initialize` → protocol version + serverInfo | `mcp.spec.ts` |
| 3.2 | `tools/list` → six tools with input schemas | › tool discovery |
| 3.3 | `tools/call` every experiment tool; `get_stage`/`list_stages`; unknown tool → JSON-RPC error | › tool calls |
| 3.4 | Malformed body → `-32700`; unknown method → `-32601` | › protocol errors |

## Persona 4 — The orchestrator (A2A)

Discovers the agent card and delegates experiments over `POST /a2a/`.

| # | Flow | Covered by |
|---|------|-----------|
| 4.1 | `GET /.well-known/agent.json` → card with skills, no streaming | `a2a.spec.ts` |
| 4.2 | `message/send` "run gossip seed 42" → completed task, summary text + data artifact | › run experiment |
| 4.3 | `tasks/get` returns the created task by id | › task roundtrip |
| 4.4 | Unknown method → JSON-RPC error | › protocol errors |

## Persona 5 — The crawler (SEO/AEO)

A search engine or LLM agent indexing the tracker.

| # | Flow | Covered by |
|---|------|-----------|
| 5.1 | `/` has meta description, canonical, OG tags, and WebSite + FAQPage + ItemList JSON-LD | `seo.spec.ts` |
| 5.2 | Stage pages carry TechArticle JSON-LD and canonical URLs | › stage pages |
| 5.3 | `robots.txt` allows all and points at the sitemap; `sitemap.xml` lists every page; `llms.txt` lists the machine endpoints | › discovery files |
| 5.4 | `/.well-known/agent.json` is served as `application/json` | › agent discovery |
