# Iteration 90 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#27`'s judgeable half** | p1, and the consumer of every feature built since iteration 77. #24 and #25 are open, so tempo and key are correctly *unjudged* rather than invented. |
| 2 | `#28` the MCP tool host | Largest user-facing win. |
| 3 | Finish `#36` on the GPU | Needs the current commit and the graph on `big`. |
| 4 | Share the spectral FFT with `mel` | The 55 % decode regression from iteration 89. |
| 5 | `f16`, with `V-01` | Needs a labelled set. |

**Why (1) beats (2) in one sentence:** #28 exposes an API whose ranking is currently unfiltered, so building the tool
first would put a polished surface over a policy nobody had written down.

## The change

`semantic::filter` — `Features`, `Verdict`, `Exclusion`, `Judgement`, `Report`, `evaluate`, `filter`.

| Decision | Choice | Why |
|---|---|---|
| an unknown feature | **`Unjudged`, never excluded** | `#27`'s criterion; a 0.2 s click has no loudness and is not a bad match for it |
| loudness | **hard exclusion** | a 12 LU difference is not a ranking weakness — the two cannot play together |
| noise floor | **penalty only** | `#27`'s own scope, and a library holds clean and noisy takes of one instrument |
| the boundary | `difference > tolerance` | at exactly the tolerance the pair is admitted; 6.00 passes, 6.01 fails |
| a cascade | `Report::describe()` names the rule | `#27`'s pitfall: an empty result must be a sentence |
| unjudged | **counted separately** | "the filter excluded nothing" and "the filter judged nothing" are different facts |
| tempo and key | **absent**, and that is correct | their features do not exist yet; an `Unjudged` verdict is what an unimplemented rule should give |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/filter.rs` | **new** — the rules, the report, 10 tests |
| `src/semantic/weights.rs` | `noise_floor_tolerance_db` (12.0, design default), its validation, its test module |
| `src/semantic/mod.rs` | one `pub mod` |
| `tests/semantic_weights.rs` | one literal updated for the new field |

## Tests that prove it

1. `twelve_lu_is_refused_and_two_lu_is_admitted` — criterion 4, with the difference reported.
2. `the_loudness_threshold_is_where_the_verdict_changes` — criterion 1: 5.9 / 6.0 / 6.01 / 12.0.
3. `an_unknown_loudness_never_excludes` — criterion 2, on either side and both.
4. `the_tolerance_is_overridable_and_changes_the_answer` — criterion 3.
5. `a_noisy_candidate_is_penalised_but_kept` — the penalty-only rule.
6. `an_all_excluded_set_says_why` — the cascade, named.
7. Four more: a clean set, unjudged counting, an empty set, and the new field's validation.

## Carved off, and why

- **Tempo and key rules.** Their features do not exist (#24, #25 open). Writing a rule against a field that is always
  `None` would be a stub dressed as a feature; the `Unjudged` verdict is the honest shape until the measurement lands,
  and the module says so.
- **Combining with embedding similarity** — `#27` puts it out of scope (`R-04`).
- **Tuning the weights** — `R-05`, and it needs `V-01`'s labelled set.
