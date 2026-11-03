# Iteration 77 — review

Three passes. Pass 1 covers the ADR's decision; passes 2 and 3 cover the loudness measurement it unblocked.

## Pass 1 — Function: does the ADR answer #4's acceptance criteria?

**Critique.** #4 asks for four things. Each is either met by a section or is not met at all:

| Criterion | Met? |
|---|---|
| an ADR exists at `docs/adr/0003-acoustic-features.md` | **yes** |
| each of the six features has a crate or an explicit sidecar assignment | **yes** — six rows, no blanks: `ebur128` ×2, hand-rolled ×3, out-of-scope ×1 |
| loudness validated against a reference on the same file, delta recorded | **yes** — and this is the part that earned the iteration: **0.01 LU** on two long clips |
| tempo and key each state an accuracy floor or are out of scope | **yes** — tempo **±2 BPM** (design default) as a test; key **out of scope** |

The claim the whole ADR rests on is that `ebur128` is pure Rust, and it is stated as **verified, not assumed**: a clean
`cargo build` with `default-features = false` finishes in **1.66 s** and invokes no `cc`. That was measured before the
sentence was written, because the crate's name is the single most misleading thing about it and a reader would
reasonably assume the opposite.

**Critique of the decision itself.** Three features are hand-rolled over the existing FFT rather than pulling `rustfft`.
That is defensible only because `mel.rs`'s FFT is already tested against CLAP's extractor to **0.0545 dB** — a seventh
dependency to compute a centroid over bins this crate already has would not earn its place. Key being *out of scope* is
the honest choice: shipping a detector whose accuracy no source reports, and letting `#27` exclude real matches on it,
is the failure the issue's own "a filter that excludes the right track is worse than a weak ranking" warns against.

**Verdict: GREAT.** Closes #4, unblocks five issues, and the one number it had to produce was measured rather than
asserted.

## Pass 2 — Zoom: the measurement, read closely

**Critique.** A loudness implementation that is *approximately* right is indistinguishable from a correct one until it
is compared, so the comparison is the test.

- **The reference match is real and tight.** −23.0 LUFS asserted at ±0.1 against `ffmpeg`'s own figure; the underlying
  `ebur128` value was −23.01. True peak −20.00 dBFS against the reference's −20.0. A wrong K-weighting, a wrong gate or
  a missing block overlap all move these by whole LU, not tenths.
- **The ratio test is independent of the absolute one.** 0.5 against 0.1 amplitude is `20·log10(5) = 13.979 dB`; the
  measured delta is 13.98. That would pass even if both absolute values were offset — which is why it is a second test
  rather than a restatement of the first.
- **The short clip is the defect the issue exists for, and it is pinned from both sides.** 0.2 s → `None`; 400 ms →
  one block and a loudness. A test that only checked the first would pass on an implementation that never measures
  anything.
- **The first draft had the defect it was written to prevent.** Its silence test failed and named it: one `blocks`
  field cannot distinguish 3 s of digital silence from a 0.2 s footstep, because the count is *above the gate*. Both
  now carry two counts and two predicates, asserted in opposite directions.
- **One assertion was loosened honestly.** `silence_measures_at_the_floor…` originally asserted `blocks > 0` for
  silence, which was simply false. It now asserts `blocks_possible > 0 && blocks == 0 && silent()`, which is what the
  standard says.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use this without the code explained?

**Critique.** Two audiences. For a developer: `Loudness` exposes `integrated: Option<f32>`, `true_peak`,
`sample_peak`, `blocks`, `blocks_possible`, plus `too_short()` and `silent()`. The doc's opening table shows the **three
different answers** the three implementations give for the same 0.2 s clip — `−70.0`, `-inf`, `None` — which is the
whole reason for the shape. A reader who wonders why it is not a bare `f32` finds the answer in the first twenty lines.

For the next agent: the remaining #21 criterion (the index round trip) is not silently skipped. It has a decision file
with three options, a recommendation, and a cost table, and the reason it is a decision rather than a patch — the
`bevy_siren index 1` magic is a versioned format.

**The honest gap.** The default test suite, clippy, rustdoc and the render gate could not run: the volume was at
**100 %, 124 MB free** and every link failed with `errno=28`. The change is behind a non-default feature and touches no
audio file, but **unrun is unrun** and `eval.md` says so rather than claiming green. This is now the third iteration
the disk has cost, and it is recordable as a blocker in its own right.

**Verdict: GREAT**, with the baseline gap named above and carried to the next iteration rather than hidden.
