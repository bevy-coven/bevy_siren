# Iteration 103 — eval

## Baseline before anything

Nothing red: 198 / 6 / **430 semantic** / **110 lib** / 6 doc, clippy and fmt clean, docs and examples build.

## The cue cases

**All 33 of `cues.yaml` green**, unchanged. This iteration touches no cue path.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| an overridden index can be queried | **RED before** | the probe printed `Some(StampMismatch)`: the escape hatch loaded an index and refused every question |
| a plain index still refuses a different model | **green, unchanged** | `a_model_mismatch_is_refused_naming_both_versions` |
| an override does not extend to a third model | **green now** | `accepts(&third)` is false and `search_ranked` answers `StampMismatch` |
| a matching stamp needs no override | **green, unchanged** | `matching.overridden.is_none()` and `accepts(&written)` |
| 31 index + 8 search tests | **green, unedited** | the existing suite is the proof nothing else moved |

## How the defect was found

Not by a failing test — by reading. `grep overridden src/semantic` returned the field, its construction sites, and **no
reader**, while the field's own doc claimed every search would see it. The claim and the code had diverged and no test
compared them.

**That is the finding worth keeping**: a doc that asserts a behaviour no test observes is a claim, not a fact. The new test
is what makes this one a fact.

## The item

`Index::accepts` and its two call sites in `search.rs`.
