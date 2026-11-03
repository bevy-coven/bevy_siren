# Iteration 96 — eval

## Baseline, before anything

Nothing was red: 198 / 6 / 380 semantic / 94 lib / 6 doc, clippy and fmt clean, docs and examples build.

## The cases

`cues.yaml`'s 33 cases are unchanged and green — this iteration adds retrieval arithmetic and touches no cue path.

| Case | Green/red | Observation |
|---|---|---|
| `Breakdown` is constructed anywhere | **red before** | `grep` over `src/` outside `model.rs` returned **nothing**; the type, its tests and its `explain()` were unreachable |
| two identical searches agree | **red before the fix** | a derived `PartialEq` compared `raw: NaN` for absent signals and returned false — determinism failed on any result with no tempo |
| contributions sum to the score | **green** | new test |
| an absent signal is not a zero | **green** | new test |
| removing a signal changes the order | **green** | new test, and it fails when the acoustic weight is left on a fixture built to defeat it |
| semantic map preserves order | **green** | strictly increasing across `[-1, 1]` |
| tempo agreement falls with distance | **green** | and is `None`, not `0.0`, when unmeasured |
| runtime per-frame cost | **unchanged** | 8/64/256 voices, no system reads the new code |

## The item

`src/semantic/ensemble.rs` (new), `search::search_ranked` + `Scored` + `RankedAnswer` + `Ranked`, and a hand-written
`PartialEq for Contribution` in `src/model.rs`.
