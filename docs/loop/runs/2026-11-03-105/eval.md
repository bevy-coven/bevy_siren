# Iteration 105 — eval

## Baseline before anything

Nothing red: 198 / 6 / **436 semantic** / **109 lib** / 6 doc, clippy and fmt clean, docs and examples build.

## The cue cases

| case | state | note |
|---|---|---|
| 1. `footstep.grass`, 3 variants, `avoid_repeat: 1` — *"no two adjacent sounds are equal"* | **green** | and this iteration made the neighbouring case (a window of zero) report itself |
| 2. `footstep.stone`, 2 variants, `avoid_repeat: 4` — *"a window wider than the list still refuses"* | **green** | |
| all other 31 cases | **green** | no cue path changed for a valid table |

**A new case is not added to `cues.yaml`**, because this one is a table-authoring mistake rather than a play behaviour, and
`problems()` is where the crate keeps those — `cues.yaml` case 18 is the table-level case, and this is a seventh kind of
mistake it names.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| `avoid: 0` is reported | **RED before** | `problems()` returned nothing for it; the play looked normal |
| `avoid: 0` refuses nothing | **proved** | 24 picks, same seed: identical to `Shuffle`, and **contains** a back-to-back repeat |
| `avoid: 1..255` is not reported | **green** | the refusal that must not fire |
| `Shuffle` is not reported | **green** | it never claimed to refuse anything |
| every problem names its cue | **green** | `tests/table.rs` now reads the name instead of enumerating variants |
| 436 semantic / 460 all-features | **green** | the existing suite is the proof nothing else moved |

## How the defect was found

Reading `Repeat::avoid()` beside `VariantPicker::pick_from`. Fourth iteration running where a bug surfaced from comparing
a documented claim with the code rather than from a failing test — the doc said *"never one of the last `avoid`
variants"*, and `avoid()` returned the field verbatim into a `min(len)` that makes `0` mean nothing.

## The item

`CueProblem::InertAvoidRepeat`, its sentence, the `Repeat` doc, and `needs-decision/diagnostic-enums-are-extensible.md`.
