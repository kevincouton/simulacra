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
- [Where simulation breaks](09-limits.html) — the 2026 skeptical correction (variance, tails, lived experience)
- [From papers to procurement](10-market.html) — NIH grand challenges, buyer's guides, vendor directories (Sept 2026)

*Updated October 2026: stage 6 extended with co-evolution; new page on
institutionalization; the playground gained a Synthesizer/Solver
co-evolution mode, a gossip/social layer, tail-fidelity sampling, a
feature-gated real LLM adapter, and a web UI served by a small Rust
server.*

This tracker ships with **Simulacra**, a Rust workspace that prototypes the
pipeline: synthesized environments, synthesized binary verifiers with
oracle/no-op/unsolved stress tests, heuristic agent personas behind an LLM
provider trait, and a simulated-town playground demo.

## FAQ

### What is synthetic human simulation?

The trend of replacing each human role in the AI training loop — judge,
data labeler, teacher, curriculum designer, researcher, environment
builder, and research subject — with a model. Synthetic data, synthetic
rubrics, AI researchers, and end-to-end RL environments are all instances
of the same idea: simulation that is roughly 10% worse than the human
original but 100x cheaper and 10,000x faster.

### What are the stages of the synthetic simulation stack?

Eight stages, in the order they were automated: (1) the reward signal —
RLHF reward models and LLM judges; (2) the training data — Phi-style
synthetic corpora; (3) the teacher — model distillation; (4) the
curriculum — self-instructing models; (5) the researcher — autonomous
experiment loops like Karpathy's autoresearch; (6) the environment —
synthesized RL worlds with synthesized verifiers; (7) the human subject —
digital twins of real people for surveys and A/B tests; (8) the physical
world — the one layer that resists full synthesis.

### Where does synthetic simulation break?

Independent evaluations converge on one finding: simulated populations
track average responses well but fail on variance, price sensitivity, and
distribution tails, and they break on lived experience and emotional
nuance. Simulation quality is a measured quantity — like the synthesized
verifiers it depends on, a simulator must be stress-tested against ground
truth before anyone acts on its output.

### What is Simulacra?

Simulacra is an open-source Rust workspace that prototypes the pipeline:
seeded environment synthesis, constraint-based binary verifier synthesis
with oracle/no-op/unsolved stress tests, heuristic agent personas behind
an LLM provider trait, a Synthesizer/Solver co-evolution loop, a
gossip-based social layer, and distributional-fidelity experiments that
check simulated populations against theory.

### Does Simulacra expose machine-readable endpoints?

Yes. The same Rust server exposes the experiments as an MCP tool server at
`/mcp` (tools: run_town, run_variance, run_coevolution, run_gossip,
list_stages, get_stage), as an A2A agent whose card lives at
`/.well-known/agent.json` with the JSON-RPC endpoint at `/a2a/`, and as
plain JSON under `/api/`. All endpoints are public, unauthenticated, and
seeded, so every result is reproducible. See the
[Agents & API](agents.html) page.
