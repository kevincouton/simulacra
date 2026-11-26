# Rust Style

House standard: [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html).
Applies to all Rust code in this repository (workspace crates under `crates/`).

## Enforcement

- `cargo fmt --all -- --check` — required (`rustfmt.toml` at repo root)
- `cargo clippy --workspace --all-targets -- -D warnings` — required
- Both run in: `.github/workflows/e2e.yml` (GitHub Actions is the canonical CI
  definition; execution is local — run the same commands directly or via `act`)

## rustfmt

`rustfmt.toml` carries the stable-compatible subset of the guide:
`max_width = 100`, `reorder_modules = true`, `use_field_init_shorthand = true`,
`use_try_shorthand = true`.

Deliberately skipped (nightly-only, unavailable on our stable toolchain):
`imports_granularity`, `group_imports`, `format_code_in_doc_comments`,
`format_macro_matchers`, `format_strings`, `wrap_comments`, `normalize_comments`,
`normalize_doc_attributes`, `reorder_impl_items`, `match_block_trailing_comma`,
`hex_literal_case`, `short_array_element_width_threshold`.

## Deviations

- `#[rustfmt::skip]` only for manual alignment or generated code (per the guide).
- Any clippy allow must be explicit, minimally scoped, and justified in a comment.
- Current explicit allows: `clippy::single_element_loop` (extensible pages list in
  `crates/tracker`), `clippy::inherent_to_string` (deliberate `Json::to_string`
  serialization API in `crates/server`).

## Verification for behavior-preserving changes

Compilation and green tests are necessary but do not by themselves prove parity.
For ports and compatibility work:

- Keep correctness assertions independent of the implementation change; never
  weaken an existing assertion merely to make a port pass.
- Run identical seeded inputs through the incumbent and candidate, comparing
  outputs, errors, and observable side effects. Record intentional differences
  explicitly instead of normalizing them away.
- Add a regression test that fails before each parity fix and passes afterward.
- Treat sandbox agreement as a fast inner check, then validate representative
  deployed behavior where feasible. Isolate duplicate writes and other side
  effects before shadowing, and retain a tested rollback path for cutovers.
- Keep human review responsible for whether a matching result is acceptable and
  maintainable, not just whether a verifier reports equality.

These precautions reflect failure modes documented in [Anthropic's production
API migration](https://www.anthropic.com/research/migrating-api-to-rust),
[GitHub's Copilot runtime port](https://github.blog/ai-and-ml/generative-ai/migrating-the-github-copilot-runtime-to-rust-using-copilot/),
and [GitHub's Scientist experiment pattern](https://github.blog/developer-skills/application-development/scientist/).
