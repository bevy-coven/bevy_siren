# Iteration 97 — review

## Pass 1 — function

**`#42`'s four criteria**, `tests/semantic_tools.rs`:

| criterion | test |
|---|---|
| a new anchor returns the new anchor's neighbours | `setting_a_new_anchor_changes_the_neighbours` |
| setting the same anchor twice is idempotent | `setting_the_same_anchor_twice_changes_nothing` |
| no anchor and none passed is refused, saying how to set one | `searching_without_an_anchor_is_refused_with_the_remedy` |
| a filter that excluded a candidate is reported | `an_excluded_candidate_is_counted` |

`#42` also asked for a **stated** scope for the anchor state, which is in the module doc: one `Session` holds one
anchor; there is no global; two clients cannot interleave because each holds its own.

**`#43`'s four**: the effect weighting is used and reported (`the_effect_weighting_is_used_and_reported`), an anchor
with no key or tempo still produces results, a category matching nothing says so, and the weights appear in the answer.
Its **pitfall** — twelve variants of one footstep — is the variant rule, tested by the invariant rather than by a
particular survivor.

**`#44`'s four**: the breakdown equals the search's for the same pair, the sentence names both directions, an unknown
feature says so rather than reporting a zero, and an effect explains too.

**The failure that mattered.** On the iteration's first scale — every signal mapped into `[0, 1]` — twelve of fifteen
tests passed and `the_sentence_names_both_directions` failed. `Breakdown::strongest()` reports the strongest positive
and the strongest **negative**; on `[0, 1]` no contribution can be negative, so `explain()` could never say what counted
against a result. The scale is now signed `[-1, 1]`. A test found it; nothing else would have.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/gunburst-spectrum.png` and compared all 21 rendered wavs by `cmp`.

- The gun burst: full-band attack to 0.03 s, decaying into six flat partials held to 0.33 s. **No vertical line**, no
  shelf, no hole. The colourbar's bottom is reached between the partials, so there is no denormal haze.
- **All 21 wavs byte-identical to iteration 96.** Nothing in this iteration touches the audio path.
- Loudness unchanged: **−21.5 LUFS, −7.5 dBFS true peak**.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Re-read with the *effects* weighting and a category filter, and with an anchor that has no measured features:

- An unmeasured anchor returns results rather than refusing — correct, and it is the case a real effect library is full
  of.
- With `Weights::effects()`, `shares()` no longer reports 70 % of the score going to tempo and key, which is what the
  `[0, 1]`-with-music-weights version would have said while those signals never had a value.
- **One criticism, recorded:** `Toolset::compatible` reads `Floor::default().min_score` rather than a floor the caller
  passes. `#29` asks for a `k` and a floor on the search tools, and this one has no floor argument at all — the default
  is `0.0`, so nothing is dropped today, but a caller cannot raise it. That is a parameter to add when `#29` is built
  rather than a defect now; the `below_floor` tally is already counted and always zero, which would show up immediately.

**Verdict: GREAT.**
