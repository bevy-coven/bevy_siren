# Iteration 104 — eval

## Baseline before anything

Nothing red: 198 / 6 / **430 semantic** / **109 lib** / 6 doc, clippy and fmt clean, docs and examples build.

## The cue cases

**All 33 of `cues.yaml` green**, unchanged. This iteration touches the tool surface and the weights, not a cue path.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| a category matches regardless of case | **RED before** | `source_of` lowercased the haystack; `contains(want)` did not lowercase the needle. `Some("Footsteps")` against a pack recorded as `Footsteps` returned **nothing**, and the tally blamed the category |
| the similarity floor can be set and counts what it drops | **RED before** | `tools.rs` compared against `Floor::default().min_score`, which is `0.0` — `Tally::below_floor` was documented, rendered, and **permanently zero** |
| a bad multiplier names the multiplier | **RED before** | three compatibility checks pushed `BadDropFraction`, so `clash_penalty: 1.5` produced *"`drop_fraction` is 1.5"* |
| a similarity floor beyond a cosine is refused | **green now** | `2.0` would empty the library and look like a filter |
| the default keeps everything | **green, unchanged** | `min_similarity: 0.0` — no existing caller's behaviour moves |
| 25 tools tests, 13 weights tests, 31 index, 8 search | **green** | the existing suite is the proof nothing else shifted |

## How the defects were found

By reading, not by a failing test — the same way iteration 103's override bug was found. Three of them, in code this
loop wrote and closed issues against:

1. `grep floor src/semantic/tools.rs` showed `Floor::default()` at the filter site and **no floor on `Toolset`**.
2. Reading `source_of` beside its one caller showed the needle was never lowercased. **My own test passed because it
   happened to use the lowercase form.**
3. Reading `problems()` showed two checks pushing a variant whose `Display` names a different field.

**The pattern worth keeping**: a documented field with no path to a non-default value (`below_floor`), and a test that
passes for a reason adjacent to the behaviour it claims (`"footsteps"` instead of `"Footsteps"`).

## The item

`Weights::min_similarity`, `WeightProblem::{BadMultiplier, BadSimilarity}`, the two `tools.rs` call sites, and
`needs-decision/weight-problem-names-its-field.md` for the public-enum change.
