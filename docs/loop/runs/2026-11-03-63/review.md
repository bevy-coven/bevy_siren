# review.md — iteration 63, three passes

## Pass 1 — Function: do the pieces do what the measurements say?

**The text tower.** Exports at **cosine 0.999999969**, 501.4 MB, 31 standard operators, 1,016 nodes. Variable tokens
work — 4 and 15 in one graph — which is the opposite of the audio tower's broken batch axis.

**The premise.** Six clips, five descriptions. Four of five winners correct. The magnitudes are the finding:
**correct 0.126–0.429, incorrect −0.104–+0.150**, and the bands **overlap**.

**The pipeline.** End to end on `bmb`: 3 of 4, the miss a **0.006** tie between two noise clips.

**The gate.** `ort` builds in 3.2 s, needs **two** features not one, loads the 501 MB graph, reports `[-1, -1]`, and
returns a unit vector at norm **1.000000**.

**Verdict: GREAT.** Every claim is a recorded measurement, and the two that were inconvenient — the overlapping bands
and the broken dynamic batch — are in the code as constants with `const` assertions rather than in a comment.

## Pass 2 — Zoom: read the awkward cases

Four defects, three found by the tests that were supposed to pass:

| Defect | Found by | Fix |
|---|---|---|
| `batch_sizes` built its capacity from the model's **own cost**, so the leftover was always zero and a dynamic graph reported **one** size | `a_dynamic_batch_graph_admits_a_range` | takes the budget as a parameter — capacity is a function of what is free **now** |
| the tiny ONNX fixture's `MatMul` assumed a fixed 8 tokens | `a_graph_runs_and_returns_unit_norm_values` | the fixture was regenerated as `[batch, tokens] × [tokens, 512]` |
| `assert!` on two constants is a **constant assertion** clippy rejects | clippy | moved into `const { … }`, where it stops the build rather than failing a test |
| `ort`'s API had moved: `Environment::builder` → `ort::environment::init().build()`, `Session::builder` now takes `&Environment`, the tensor extractor returns a `(shape, data)` pair | the compiler, four times | every one is recorded in ADR 0002, because a consumer would hit them too |

The first is the important one: an optimistic batch capacity is exactly the number that fails at run time and looks
fine in a test that only checks it is non-zero.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use this without me?

- **The messages were read, not just the variants.** *"the file is 44100 Hz and the model needs 48000 Hz; no resampler
  is approved yet (docs/loop/needs-decision/semantic-dependencies.md)"* — names both rates and where the decision
  lives. *"this graph returns 256-element vectors and the index holds 512. Re-index the library with this model."*
  *"1 results, 2 below the floor."* None says "invalid input".
- **The floor cannot be quietly made worse.** `the_measured_bands_overlap_which_is_why_the_default_floor_keeps_everything`
  is a `const` assertion: if a later measurement finds the bands separated, **the build stops**. And
  `an_intuitive_floor_would_have_rejected_every_correct_match` pins the 0.5 trap with the measured numbers.
- **A 16 KB fixture replaces a 501 MB one** for the shape contract, so the test that matters runs on every push.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| issues closed | **3** — #10, #17, #26 |
| issues advanced | **1** — #12, narrowed to one measured dependency |
| new modules | `model.rs`, `search.rs`, `session.rs`, `query.rs` |
| tests | **+29**, from 236 to **265** all-features |
| the path | **8 of 9 closed**; only #12 remains, and its decision is measured |
