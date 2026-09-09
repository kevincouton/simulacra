# Stage 4 — The Curriculum (2024)

Stages 1–3 made the inputs synthetic; stage 4 is where the loop starts
closing on itself — the model begins deciding what to learn next.

The pieces existed early — **Self-Instruct** (models writing their own
instruction sets) and **STaR** (models bootstrapping their own reasoning
traces) are both 2022 — but the flip came when:

- **Self-Rewarding Language Models** (Meta) — a model generates its own
  tasks, judges its own outputs, and improves past the ceiling of its human
  preference data.
- **SPIN** — self-play-style improvement without new human labels.

Curriculum design — historically the most artisanal part of ML, the
taste-driven choice of what to train on next — became something models do
to themselves.
