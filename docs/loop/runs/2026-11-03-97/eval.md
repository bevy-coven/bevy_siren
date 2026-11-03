# Iteration 97 — eval

## Baseline before anything

Nothing red: 198 / 6 / 402 semantic / 94 lib / 6 doc, clippy and fmt clean.

## The cases

`cues.yaml`'s 33 cases unchanged and green — no cue path is touched.

| Case | Green/red | Observation |
|---|---|---|
| `#42` anchor changes neighbours | **red before** | no `set_anchor` existed |
| `#42` idempotent re-set | **red before** | idem |
| `#42` no-anchor refusal names the remedy | **red before** | idem |
| `#42` exclusion reported | **red before** | idem; the test now uses a **loudness** gap, because noise floor only penalises |
| `#43` effect weighting reported | **red before** | `Weights::effects()` did not exist |
| `#43` unmeasured anchor still matches | **green** | new |
| `#43` empty category says why | **green** | new |
| `#43` variants reported not repeated | **green** | new, after two fixture corrections — the invariant is "exactly one of the pair", not a particular survivor |
| `#44` explanation equals the search's breakdown | **green** | new |
| `#44` sentence names both directions | **red before the scale fix** | on `[0,1]` no contribution is negative, so `strongest()` could never find one |
| `#44` unknown feature says so | **green** | new |
| `#44` effect explains | **green** | new |

## The defect this iteration found

The `[0, 1]` agreement scale made `Breakdown::explain()` structurally incapable of naming what counted **against** a
result: `strongest()` looks for a contribution below zero, and on `[0, 1]` none can be. Twelve of fifteen tests passed
against the wrong scale; the three that failed were the ones asserting the breakdown *explains* anything.

## The item

`src/semantic/tools.rs` (new), `Weights::effects()` (new), and the signed scale in `src/semantic/ensemble.rs`.
