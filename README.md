# Simulacra

**Live:** [simulation.lucanian.app](https://simulation.lucanian.app) — research tracker · [`/app/`](https://simulation.lucanian.app/app/) — playground UI

A Rust-first project dedicated to the new trend of **synthetic human
simulation** — the observation that synthetic data, synthetic rubrics, AI
researchers, and end-to-end RL environments are one thing: increasingly
ambitious human simulation, 10% worse but 100x cheaper and 10,000x faster.

## Parts

| Crate | What it does |
| --- | --- |
| `simulacra-agents` | Persona trait, heuristic-stub personas (shopper, explorer, oracle), gossip layer (`GossipBook` + `GossipingPersona`), and an `LlmProvider` seam — a feature-gated `llm` module adds a real OpenAI-compatible backend via curl (`--features llm`). Seeded, deterministic `Rng`. |
| `simulacra-env` | Environment synthesis (seed → town of shops with hidden stock, tunable `TownConfig`) and verifier synthesis — binary verifiers built from task constraints, never from a reference solution, with oracle/no-op/unsolved stress tests. |
| `simulacra-engine` | Simulation loop; the `VarianceExperiment` distributional-fidelity check (uniform and skewed population sampling); the `CoEvolution` Synthesizer/Solver game (difficulty ratchets against population success); hidden-location runs where gossip is the only way to learn where items are stocked. |
| `simulacra-demo` | Playground CLI: `town` (default), `variance`, `coevolution`, `gossip` subcommands. |
| `simulacra-server` | std-only HTTP server: JSON API (`/api/town`, `/api/variance`, `/api/coevolution`, `/api/gossip`) plus the built web UI. Binds `127.0.0.1:8787` by default. |
| `simulacra-tracker` | Static research-site generator over `content/` (the stages of the synthetic-simulation stack + limits + market pages). |
| `web/` | Nuxt 3 client-side UI for the playground; `npx nuxi build` emits `web/.output/public`, served by `simulacra-server`. |

## Quick start

```sh
cargo test --workspace          # all tests
cargo doc --workspace --no-deps # docs
cargo run -p simulacra-demo                 # town playground
cargo run -p simulacra-demo -- variance     # distributional-fidelity experiment
cargo run -p simulacra-demo -- coevolution  # Synthesizer/Solver game
cargo run -p simulacra-demo -- gossip       # information-diffusion experiment
cargo run -p simulacra-tracker              # builds research site into site/dist
cd web && npm install && npx nuxi build     # builds the web UI
cargo run -p simulacra-server               # serves UI + JSON API on :8787
```

The tracker and server binaries locate the workspace root from their own
executable path; set `SIMULACRA_ROOT` to override.

The default build has no external dependencies and never touches the
network: everything runs offline and replays from a seed. The optional
`llm` feature (`SIMULACRA_LLM_API_KEY`, `SIMULACRA_LLM_ENDPOINT`,
`SIMULACRA_LLM_MODEL`) enables real model calls.
