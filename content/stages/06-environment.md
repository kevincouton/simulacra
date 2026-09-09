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

The gym, the referee, and the scoreboard are all models now.
