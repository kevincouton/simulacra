# Simulacra

A Rust-first project dedicated to the new trend of **synthetic human
simulation** — the observation that synthetic data, synthetic rubrics, AI
researchers, and end-to-end RL environments are one thing: increasingly
ambitious human simulation, 10% worse but 100x cheaper and 10,000x faster.

## Parts

| Crate | What it does |
| --- | --- |
| `simulacra-agents` | Persona trait, heuristic-stub personas (shopper, oracle), and an `LlmProvider` seam for real model backends later. Seeded, deterministic `Rng`. |
| `simulacra-env` | Environment synthesis (seed → town of shops with hidden stock) and verifier synthesis — binary verifiers built from task constraints, never from a reference solution, with oracle/no-op/unsolved stress tests. |
| `simulacra-engine` | Simulation loop: personas attempt tasks, synthesized verifiers score. |
| `simulacra-demo` | The town playground: `cargo run -p simulacra-demo [seed]` — agents run errands, verifier rewards print, exit 0. |
| `simulacra-tracker` | Static research-site generator over `content/` (the 8 stages of the synthetic-simulation stack). |

## Quick start

```sh
cargo test --workspace          # all tests
cargo doc --workspace --no-deps # docs
cargo run -p simulacra-demo     # town playground
cargo run -p simulacra-demo -- variance  # distributional-fidelity experiment
cargo run -p simulacra-tracker  # builds site into site/dist
```

The tracker binary locates the workspace root from its own executable path;
set `SIMULACRA_ROOT` to override.

No external dependencies, no network calls: everything runs offline and
replays from a seed.
