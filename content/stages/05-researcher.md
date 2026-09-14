# Stage 5 — The Researcher (2026)

The assistance era (Copilot, then SWE-agents) kept a human choosing the
experiments. The discovery era did not.

- **AlphaEvolve** (DeepMind) — evolved genuinely new algorithms (2025).
- **The AI Scientist** (Sakana) — sketched the full paper-writing pipeline,
  later published in Nature.
- **autoresearch** (Karpathy, March 2026) — the big moment: a deliberately
  minimal ratchet loop where a coding agent modifies a real LLM training
  setup, runs a five-minute experiment, keeps the change only if validation
  loss improves, and repeats overnight. His extended run stacked 700
  experiments into 20 kept improvements, cutting time-to-GPT-2 from 2.02 to
  1.80 hours — real, transferable code changes found while he slept.

The pattern replicated within weeks, turning one GitHub repo into a genre:

- **Red Hat OpenShift AI** — ran the loop unsupervised for 24 hours: 198
  experiments, a 2.3% validation-loss improvement, zero human intervention.
- **Hyperspace** — distributed the loop peer-to-peer: 35 autonomous agents
  ran 333 experiments in a single night (March 8–9, 2026).
- **Shopify** — 37 overnight experiments for a 19% gain on an internal
  workload.
- **Research-lifecycle benchmarks** ("Act As a Real Researcher") — by
  mid-2026 the loop itself had become something you score agents on.

Experiment selection — the last intellectual step humans kept in the loop —
became an optimization target, then a benchmark category.
