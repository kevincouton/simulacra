# Stage 1 — The Reward Signal (2022)

The first thing to go synthetic was, counterintuitively, **the judge**.

InstructGPT established the now-canonical trick: collect human preferences
once, train a reward model, and let the policy optimize against the model
rather than the humans. From the policy's point of view, the thing
dispensing approval was already an LLM.

- **InstructGPT** (OpenAI) — RLHF with a learned reward model replacing
  per-step human judgment.
- **Constitutional AI / RLAIF** (Anthropic) — the AI critiques itself against
  a set of principles; AI feedback replaces human feedback.
- **Lee et al.** — showed AI feedback matching human feedback at a fraction
  of the cost.
- **LLM-as-judge** — by the time MT-Bench and AlpacaEval made model judges
  the default eval methodology, the entire approval apparatus (reward,
  critique, evaluation) ran on models judging models.

The pattern set here — *capture the human signal once, then simulate it
forever* — is the template every later stage reuses.
