# Iteration 91 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#24` tempo** | p1, unblocked, and one of the two features `#27`'s remaining rules need. ADR 0003 made its accuracy a **test**, so the item is falsifiable rather than aspirational. |
| 2 | `#25` key | The other half of `#27`, and ADR 0003 declares it out of scope for Rust — so it needs a decision, not code. |
| 3 | `#28` the MCP tool host | Largest user-facing win. |
| 4 | Finish `#36` on the GPU | Needs the current commit and the graph on `big`. |
| 5 | Share the spectral FFT with `mel` | Iteration 89's 55 % decode regression. |

**Why (1) beats (3) in one sentence:** #28 exposes an API whose filter cannot judge tempo until this exists, and ADR 0003
already wrote the acceptance criterion that makes the work checkable.

## The change

`semantic::tempo` — `estimate(&[f32]) -> Result<Tempo, TempoProblem>`, with `Tempo { bpm: Option<f32>, confidence,
octave_ambiguous }`.

| Decision | Choice | Why |
|---|---|---|
| absence | **`bpm: Option<f32>`** | `#24`'s pitfall: a single impact must report *no tempo*, not a confident wrong one |
| the range | applied to the **lags**, not the answer | the octave error cannot be created by the search |
| octave | **flagged, never corrected** | `#24` asks for the ambiguity to be *countable*; silently halving is the same error reversed |
| the gate | an **onset-ratio** before any correlation | confidence alone scores a tone at 0.29 and a click at 0.86, so it cannot separate them |
| refinement | parabolic interpolation on the peak | at 100 frames/s a whole-frame lag is 2 BPM coarse — the entire floor |
| silence | `None`, not an error | it is a fact about the sound |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/tempo.rs` | **new** — `Tempo`, `TempoProblem`, `estimate`, `MIN_BPM`/`MAX_BPM`/`FLOOR_BPM`/`MIN_CONFIDENCE`/`MIN_ONSET_RATIO`, 11 tests |
| `src/semantic/mod.rs` | one `pub mod` |

No new dependency, no serde type, no index section.

## Tests that prove it

1. `the_floor_holds_across_the_whole_stated_range` — **80 tracks**, every whole BPM 80–159, each within ±2 (criterion 1).
2. `an_octave_estimate_is_flagged` — `within_floor` and `is_octave_of` distinguish the two failure kinds (criterion 2).
3. `a_single_impact_has_no_tempo` and `a_steady_tone_has_no_tempo` — criterion 3, from both directions.
4. `nothing_outside_the_stated_range_is_reported` — the range is a bound, not a preference.
5. Seven more: silence, empty, too short, non-finite, determinism, a subdivision, and the ambiguity field.

## Carved off

- **Beat tracking and downbeat detection** — `#24` puts them out of scope.
- **Wiring tempo into `build` and the index.** That is a section plus a field, and it belongs with `#25` so the two
  features land in one schema change rather than two.
- **`#27`'s tempo rule.** It needs this to exist first; it is the natural next iteration.
