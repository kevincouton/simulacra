# Stage 6 — The Environment (2026)

RL's scaling bottleneck moved from the model to the environment: you need
thousands of executable, verifiable, professionally realistic task worlds,
and humans can't hand-build them fast enough.

- **Z.ai / GLM-5.3 pipelines** — synthesize environments end to end:
  research agents mine real work patterns and convert them into long-horizon
  environments with hidden state; a judge agent attempts each task to
  confirm it's solvable; verifiers are synthesized *without seeing the
  reference solution*, then stress-tested with oracle, no-op, and
  unsolved-state checks until their binary reward is reliable enough to
  train on directly. The environment, judging, and verification stack is
  synthetic all the way down.
- **Ornith-1.5** — claimed end-to-end self-improvement: the model proposes
  its own tasks and generates its own RL rollouts.

By mid-2026 the GLM-5.3 recipe had been productized and open-sourced:

- **Agent World Model (AWM)** (Snowflake, open source) — generates 1,000
  SQL-backed executable environments for agentic RL with benchmark-winning
  results; environment synthesis is now an arXiv subfield in its own right.
- **Harness-native training pipelines** — five-stage synthesis (propose →
  build → judge-agent solvability → verify → train) built on agent SDKs such
  as Claude's, with a frontier model as the backbone of every module.
- **Verifier-in-the-loop** — the LLM–verifier interface is now formal enough
  to anchor a summer school (FoPSS 2026), covering generate–verify–retry
  loops and binary-reward reliability.

Late 2026 pushed the stage from "synthesize a gym" to "co-evolve with the
agent":

- **Agent–environment co-evolution** — world models that co-train with the
  policy (WebEvolver, DreamGym): the environment adapts to the agent while
  the agent learns, instead of staying fixed.
- **General Agent** (Prime Intellect) — synthetic task creation as a
  two-player game: a Synthesizer agent invents tasks, a Solver attempts
  them, and the game itself generates the curriculum.
- **A vendor market appears** — directories of RL-environment vendors
  (e.g. RL List, Oct 2026) and funded startups selling simulation sandboxes
  for benchmarking and training agents before deployment. Environment
  synthesis is a procurement category now, not just a paper topic.

The gym, the referee, and the scoreboard are all models now — the referee
is getting its own theory, and the gym has started fighting back.
