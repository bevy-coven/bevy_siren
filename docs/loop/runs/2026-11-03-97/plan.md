# Iteration 97 — plan: the workflow as calls

## Ranked

1. **`#42`/`#43`/`#44`** — chosen. Unblocked by iteration 96, and they are the calls `#32`–`#34`'s editor panel and
   `#28`'s tool surface both wrap. Library functions, so both paths share one implementation.
2. `#47` query latency budget — measurable now; needs a number and a test.
3. `#36` throughput target — 750 clips/s is measured; the target is unset.
4. `#13` chunk long tracks — the last core index behaviour missing.
5. `#28` MCP server — **cannot** be built here: it needs a JSON dependency, which changes the closed dependency list in
   `Cargo.toml` and `tests/leaf.rs`. Goes to `needs-decision/`.

## Design

`src/semantic/tools.rs`:

- `Session` — one `Option<u64>`. `set_anchor` is idempotent by construction; `clear_anchor`; `resolve_anchor` refuses
  with `NoAnchor` naming `set_anchor`.
- `Toolset { music: Weights, effects: Weights, variants: Variants }` with `compatible_music`, `compatible_sfx`,
  `explain_match`.
- `Found { hash, score, similarity, breakdown, provenance }` with `explain()`.
- `Tally { returned, excluded, below_floor, variants: Vec<(dropped, resembled)>, wrong_category }` with `describe()`.

`src/semantic/weights.rs`: `Weights::effects()`.

`src/semantic/ensemble.rs`: the scale becomes signed `[-1, 1]`.

## Fields

- `Weights::effects()` — four values, each a **design default**, documented in the impl.
- `Variants::identical_below = 0.99` — a serde type, **design default**.

## Tests (`tests/semantic_tools.rs`, 15)

One per criterion of the three issues, plus the invariants: a new anchor changes the neighbours; the same anchor twice
is field-for-field idempotent; no anchor refuses with the remedy; an unindexed anchor names its hash; an excluded
candidate is counted; the effect weighting is used and reported; an unmeasured anchor still matches; a category that
matches nothing says so; a category keeps its own; a near-identical variant is reported with its survivor; the
explanation equals the search's breakdown; the sentence names both directions; an unknown feature says so; an effect
explains; the two weightings are separate, usable, and total the same.
