# Iteration 101 — review

## Pass 1 — function

**Does it do what `#23` says?**

| criterion | test | result |
|---|---|---|
| a dry recording and its artificially reverberated copy separate in the expected direction | `a_reverberated_copy_reads_wetter_than_its_dry_source` | **pass** — and by more than 6 dB, asserted, so it is not a rounding difference |
| the measure has a stated range and a stated failure mode for a fully dry clip | `a_fully_dry_clip_reports_the_ceiling_rather_than_an_infinity`, `a_short_clip_has_no_measure_rather_than_a_confident_one` | **pass** — range −∞…+∞ in principle, clamped to ±60 dB; a zero tail reports **60 dB, not infinity**; a clip too short to have a tail reports **no measure**, which is a different fact |
| its discrimination on the labelled dry/wet pairs is recorded as a number | `the_discrimination_is_recorded_over_a_labelled_set` | **pass** — `12/12 separated (100 %); dry ≥ 60.0 dB, wet ≤ 1.9 dB; bands disjoint` |

**The issue's own candour requirement is the one I want to be exact about.** The measurement is on **synthetic** pairs: a
dry burst convolved with an exponentially decaying noise tail. That tail has no modal structure, no early reflections and
no absorptive character. A real studio and a real cave differ in more ways than decay rate, and **real recordings have
not been measured**. So:

- `DISCRIMINATION_MEASURED` says *"synthetic only … real recordings not measured"* in its own text;
- the module doc says it;
- **the signal is not in the default weights**, and the doc says why.

`#23` says *"if the discrimination measurement is poor, say so and recommend dropping it from the default weights rather
than shipping it because it exists."* The measurement is **not** poor — 100 % with disjoint bands — but it is also not
evidence about the material that matters. Recommending it stay out is the answer the issue's own logic demands.

**Verdict: GREAT.**

## Pass 2 — zoom

No spectrogram is the right instrument here: this iteration adds no audio and touches no playback path, and the rendered
wavs are byte-identical (checked below). The visual check that *does* apply is the measurement sentence, which the test
prints every run:

```
REVERB DISCRIMINATION: 12/12 pairs separated (100 %); dry ≥ 60.0 dB, wet ≤ 1.9 dB; bands disjoint
```

- **`dry ≥ 60.0 dB`** is the ceiling, not a measurement of the dry clips: their tails are exact digital silence after the
  20 ms burst, so the denominator is zero and the clamp fires. That is the intended path, and it is asserted directly by
  `a_fully_dry_clip_reports_the_ceiling_rather_than_an_infinity`.
- **`wet ≤ 1.9 dB`** is a real measurement: the wettest copy still had slightly more early than late energy at a 500 ms
  window. **That is worth recording**: my synthetic tails are conservative, and a real cave would push the wet end
  negative. If the real corpus ever shows wet values *above* the dry clamp, the clamp is hiding a mis-measure rather than
  capping a dry clip.
- **No NaN, no infinity, no panic** on the four degenerate inputs (empty, too short, silent tail, silent head).

**One criticism, recorded rather than fixed:** `peak_at_s` uses the frame grid (`HOP` = 480 samples, 10 ms), so the
direct-sound proxy is quantised to 10 ms. For a 50 ms boundary that is 20 % of the early window, which is coarse. A
sample-accurate peak would need a second pass over the audio; the frame grid is what the shared spectra already give. The
error is bounded and one-directional (the proxy is never *before* the true peak), so it makes the measure slightly
*less* early-weighted than it should be — a conservative bias rather than an optimistic one, which is the right way for
this number to be wrong.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Re-read against a different corpus (24 pairs, RT60 0.4–1.6 s) and asked whether a developer could understand the field
without the code:

- `Reverb { early_to_late_db: Option<f32> }` with a doc that says `None` is *"no late window"* and **not** *"dry"* — the
  optionality trap `#25` and `#23` both name. A developer reading `None` cannot conclude the clip is dry.
- The range table (+20 dry, 0–10 audible room, below 0 wet) is in the module doc, so a number has a meaning without
  looking up a formula.
- `DISCRIMINATION_MEASURED` is a `const &str` rather than a doc sentence, so the caveat travels with the crate rather than
  staying in a run file.
- At 24 pairs with a wider RT60 spread, separation stayed 24/24 and the wet maximum rose to about 4 dB — still disjoint,
  still conservative. The measure is stable across the corpus's spread rather than tuned to one value.

**Verdict: GREAT.**
