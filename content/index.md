# The Synthetic Simulation Stack

Human simulation, stage by stage. If you squint, what we used to call
"synthetic data", "synthetic rubrics", "AI researcher", and "end-to-end RL
environments" is one trend: **increasingly ambitious human simulation** —
10% worse, but 100x cheaper and 10,000x faster.

Each stage below replaced one more human role in the training loop with a
model. The remaining human roles narrow as the loop closes on itself.

- [Stage 1 — The reward signal](01-reward.html) — the judge goes synthetic (2022)
- [Stage 2 — The training data](02-data.html) — the corpus goes synthetic (2023)
- [Stage 3 — The teacher](03-teacher.html) — distillation from models (2023)
- [Stage 4 — The curriculum](04-curriculum.html) — models choose what to learn (2024)
- [Stage 5 — The researcher](05-researcher.html) — agents run experiments (2026)
- [Stage 6 — The environment](06-environment.html) — RL worlds get synthesized (2026)
- [Stage 7 — The human subject](07-subject.html) — simulated people, simulated panels (2025)
- [Stage 8 — The physical world](08-physical.html) — the part that can't be synthesized (2026, in progress)

This tracker ships with **Simulacra**, a Rust workspace that prototypes the
pipeline: synthesized environments, synthesized binary verifiers with
oracle/no-op/unsolved stress tests, heuristic agent personas behind an LLM
provider trait, and a simulated-town playground demo.
