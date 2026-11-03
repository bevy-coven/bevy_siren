# Iteration 91 — research: tempo, and a threshold whose unit was wrong

The item is [#24](https://github.com/bevy-coven/bevy_siren/issues/24): a tempo estimate with a **stated accuracy floor**.

## Sources

| Source | The rule it reports |
|---|---|
| ADR 0003 (`docs/adr/0003-acoustic-features.md`) | tempo ships in Rust with a **±2 BPM floor at 80–160 BPM**, and *"a tempo estimate outside the floor is a wrong number that #27 would exclude real matches on"* |
| ADR 0003, the same section | *"at 120 BPM, ±2 BPM is ±1.7 %, below the just-noticeable difference for tempo in a rhythmic layer"* — **design default**, since no source gives the number for game music transitions |
| `#24` | estimate in BPM **with a confidence**; handle the octave error explicitly; **a non-rhythmic asset reports no tempo**; octave errors counted separately |
| `#24`'s pitfall | *"an asset with one impact has no tempo, and most tempo estimators will invent one"* |
| `src/semantic/mel.rs` | `WINDOW = 1024`, `HOP = 480`, and a radix-2 FFT tested to 0.0545 dB |

## The technique, in the sources' terms

Onset strength is the frame-to-frame **increase** in spectral energy, positive part only — an onset is a rise, and a
decay is not a second onset. Its **autocorrelation** over the lag range the BPM bounds allow is the tempo evidence.

Two properties of that choice matter:

1. **The range is applied to the lags, not to the answer.** A lag corresponding to 40 or 320 BPM is never searched, so
   the octave error cannot be *created* by the search — it can only come from the music's own subdivision, which is what
   [`Tempo::octave_ambiguous`] reports rather than "corrects".
2. **A confidence is always produced**, and the estimate is `None` below it. The issue's pitfall is an estimator that
   invents a tempo for a single impact; an `Option` makes that state representable, and `R-03`'s filter already treats
   `None` as *unjudged* rather than as a mismatch.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| search range | **80–160 BPM** | ADR 0003 |
| accuracy floor | **±2 BPM** | ADR 0003, design default |
| confidence floor | **0.10** | **design default**, measured against the corpus and the false positives |
| onset-ratio gate | **2.0** | **design default** — the number this iteration had to correct |
| corpus | **80 tracks**, every whole BPM 80–159 | `#24` asks for at least 20 |

## What good sounds like — and the failure that took three attempts

A click track at every whole BPM from 80 to 159 must estimate within **±2 BPM**; a single impact and a steady tone must
report **no tempo**; the same buffer must estimate identically twice.

**The false positive was a steady 440 Hz tone reported as `150.1 BPM` at confidence 0.29.** That is exactly the failure
`#24` warns about, and it survived the first fix because **the constant's unit was wrong**.

The first gate compared the mean onset rise against the buffer's mean sample energy and used `0.02`, justified by a
windowed time-domain energy I had measured by hand — **0.0003 for a tone**. But `onset_envelope` uses **FFT energy**,
which is ~10⁶ times larger, so the real ratios are:

| | mean onset | mean sample energy | ratio |
|---|---|---|---|
| steady tone | 2.25 × 10⁻² | 8.00 × 10⁻² | **0.28** |
| click track | 1.81 × 10² | 4.40 × 10⁻⁴ | **4.1 × 10⁵** |

A threshold of `0.02` therefore sat **below** the tone's 0.28 and let the false positive through while looking correct.
The gate is `2.0` — between the two by three orders of magnitude on one side and five on the other — and the constant's
doc records the measured values so the next reader can check the unit rather than trust the number.

**The general lesson**, and the fifth fixture-shaped error in seven iterations: *a threshold derived from a hand
calculation is only as good as its agreement with the unit the code actually produces.* Measuring the ratio the code
computes, rather than the ratio I expected, is what found it.
