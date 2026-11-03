# Iteration 96 — review

## Pass 1 — function

**Does it do what `#40` says?** Its four criteria, each against a test:

| criterion | test | result |
|---|---|---|
| arithmetic lives where no engine type appears, `leaf.rs` still passes | `the_breakdown_is_usable_with_no_app` + `tests/all.rs`'s `the_ensemble_lives_in_the_engine_free_model` | **pass** |
| every component reports value, weight, contribution; they sum to the score | `the_contributions_sum_to_the_score` | **pass** |
| two calls return identical order and scores, field for field | `the_same_query_returns_identical_order_and_scores` | **pass — after a real fix** |
| removing a signal changes the order where that signal matters | `removing_a_signal_that_matters_changes_the_order` | **pass** |

**The criterion-3 test found a bug in code that was already written and already tested.** `Contribution::absent` stores
`raw: f32::NAN`; the derived `PartialEq` compared `NaN != NaN`, so two identical searches returned unequal answers
whenever any signal was absent — which is every asset with no tempo. `#40`'s determinism criterion was unsatisfiable
against the shipped type. The fix is a hand-written `PartialEq` that treats `NaN` equal **only** to `NaN`, so it is a
total order on the values a breakdown can hold rather than a loosening that hides real differences.

That is worth stating plainly: the ensemble type existed, was documented, and could not have satisfied its own
acceptance criterion. Nothing tested it because nothing constructed it.

**Verdict: GREAT.**

## Pass 2 — zoom

Read: `spectrograms/combat-spectrum.png`, `spectrograms/voices8-spectrum.png`, plus every rendered wav compared by
`cmp`.

- `combat-spectrum.png`: attack at 0–0.15 s across the full band, decaying into eight flat partials held to 1.55 s. **No
  vertical discontinuity**, no shelf, no hole. Clean.
- `voices8-spectrum.png`: **eight distinct onsets** at 0.04–0.16 s, each a full-band burst, then silence. The floor after
  0.16 s is the colourbar's bottom rather than a haze — no denormal tail. No clipping shelf at 0 dBFS (the render's peak
  is −7.5 dBFS on the mix).
- **All 21 wavs byte-identical to iteration 95** (`cmp` over every file). The change is offline arithmetic, and the
  audio proves it.
- Loudness unchanged: **−21.5 LUFS integrated, −7.5 dBFS true peak**.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Regenerated with a different weight configuration (`only(1.0, 0.0, 0.0, 0.0)` — semantic only) and read the results as a
developer would:

- With acoustic weighted, the ranking is `[2, 1]`; with it zeroed, `[1, 2]`. The signal is not decorative, and the test
  says so by asserting **both** orders rather than only that they differ — "they differ" would pass for a bug that
  reversed them.
- `explain()` reads as a sentence: *"semantic carried it (+0.710 from 0.71 raw at weight 1); no value for tempo, key"*.
  A developer does not need the code to read that.
- **One criticism, recorded:** the acoustic agreement falls back to reading `noise_floor_db` from the pair when the filter
  produced no penalty. That is correct but subtle — the same number is computed twice, once to decide exclusion and once
  to score. They agree because both read `Weights::noise_floor_tolerance_db`, but a future edit to one would silently
  desynchronise them. The right fix is for `filter::evaluate` to report the *agreement* alongside the penalty; that is a
  change to `Judgement`'s shape and belongs in its own iteration rather than being smuggled in here.

**Verdict: GREAT.**
