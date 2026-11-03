# Iteration 94 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#27`'s tempo and key rules** | The last two in its set, unblocked by 91–93. Without them the filter uses two of the four features the crate now produces and stores. |
| 2 | One shared spectral pass | The measured 2.7× decode regression from iteration 93, and the third feature to walk the same frames. |
| 3 | `#28` the MCP tool host | Largest user-facing win. |
| 4 | Finish `#36` on the GPU | Needs the current commit and the graph on `big`. |
| 5 | `V-01`'s labelled set | The only thing that measures any of this on real assets. |

**Why (1) beats (2) in one sentence:** #27 closes a p1 issue whose remaining work is already designed, while the shared
pass is an optimisation of code that works and that a labelled set may show is unnecessary.

## The change

`Features` gains `tempo` and `key`; `evaluate` gains two rules; `Weights` gains two fields.

| Decision | Choice | Why |
|---|---|---|
| tempo | **hard exclusion** beyond `tempo_tolerance_bpm` | 90 under 128 is a clash whatever the timbres — `#27`'s own example |
| the tolerance | **4.0 BPM**, twice the ±2 estimate floor | excluding inside the estimator's own uncertainty would reject tracks that merely look different |
| `bpm: None` | **read as no tempo** | iteration 93 stored it as *judged, not rhythmic*; treating it as a tempo would compare every non-rhythmic asset with every other |
| key | **penalty only**, from `Compatibility` | `#27`'s scope, and key is the least reliable feature |
| `multiplier == 1.0` | **no penalty recorded** | a project that sets `clash_penalty: 1.0` gets no penalty, rather than one whose value is nothing |
| both new fields | **validated on `problems()`** | a negative tempo tolerance excludes everything; a multiplier outside `0..=1` inverts or rewards a clash |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/filter.rs` | `Features` gains two fields and two accessors; `Exclusion` gains `Tempo` and `Key`; `evaluate` gains two rules; **7 new tests** |
| `src/semantic/weights.rs` | `tempo_tolerance_bpm` (4.0) and `compatibility`, their defaults, their validation |
| `tests/semantic_weights.rs` | one literal updated for the new fields |

## Tests that prove it

1. `the_tempo_threshold_is_where_the_verdict_changes` — 90/128 refused; 3.9, 4.0 admitted; 4.1, 38.0 refused.
2. `an_unknown_or_non_rhythmic_tempo_never_excludes` — absent record, `bpm: None`, and either side.
3. `a_clashing_key_is_penalised_and_never_excluded` — C against C#, multiplier from the table.
4. `a_fifth_and_a_same_key_are_unpenalised` — and a parallel shift is gentler than a clash.
5. `an_unknown_key_never_penalises` — including a *judged atonal* record.
6. `the_new_thresholds_are_overridable` — `#27`'s criterion 3, including `clash_penalty: 1.0` turning the rule off.
7. Two more: the defaults validate, and the empty-set sentence still holds.

## Carved off

- **A tempo rule for octave-ambiguous estimates.** `Tempo::octave_ambiguous` is stored but not read by the rule: an
  ambiguous estimate already falls inside the 4 BPM tolerance against its own octave only if the tempo is under 8 BPM,
  so reading the flag would need a rule about *which* octave to compare in — a design question, not a bug.
- **Combining with the ensemble** — `#27` puts it out of scope (`R-04`).
