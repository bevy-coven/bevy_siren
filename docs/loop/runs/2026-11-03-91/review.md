# Iteration 91 — review

## Pass 1 — Function: does tempo meet #24?

**Critique.** Four criteria, and the first is the one ADR 0003 made falsifiable:

| # | Criterion | Test | Why it is not vacuous |
|---|---|---|---|
| 1 | accuracy within ±2 BPM on ≥20 tracks | **80 tracks**, 80–159 | every whole BPM, so a bias at one end fails |
| 2 | octave errors counted separately | `is_octave_of` vs `within_floor` | 128 against 64 is an octave *and* outside the floor — both true, and reported as different facts |
| 3 | a non-rhythmic asset reports no tempo | a single impact **and** a steady tone | two different ways to be non-rhythmic |
| 4 | the floor documented; absent treated as unknown | `FLOOR_BPM`'s doc; `bpm: Option` | the type makes "unknown" representable |

**Critique of the design.** The range is applied to **lags**, not to the answer, so the octave error cannot be *created*
by the search — it can only come from the music's own subdivision. And the ambiguity is **flagged, never corrected**:
`#24` asks for it to be *countable*, and silently halving a tempo would be the same mistake in the other direction.

**Verdict: GREAT.**

## Pass 2 — Zoom: the false positive that survived one fix

**Critique.** A steady 440 Hz tone estimated **150.1 BPM at confidence 0.29**, while a real click track scores 0.86. So
the confidence alone cannot separate them — any threshold low enough for music accepts the tone.

The fix was an **onset-ratio gate**, and its first version used `0.02`, which is *correct arithmetic on the wrong unit*.
I measured a windowed time-domain energy by hand (0.0003 for the tone) and wrote a threshold below it, but
`onset_envelope` returns **FFT energy**, ~10⁶ times larger. The measured ratios are:

| | ratio |
|---|---|
| steady tone | **0.28** |
| click track | **4.1 × 10⁵** |

So `0.02` sat **below** the tone's 0.28 and let the false positive through while reading as a conservative choice. The
gate is now **2.0** — between the two by three orders of magnitude on one side and five on the other — and the constant's
doc records both measurements so the next reader can check the unit rather than trust the number.

**The general lesson, and the fifth fixture-shaped error in seven iterations**: a threshold derived from a hand
calculation is only as good as its agreement with the unit the code produces. Measuring the ratio *the code computes*
is what found it, and the same discipline caught 87's constant vector, 88's `seed % 24`, 89's five-window "short" clip,
and 90's test inside an `impl`.

**Two doc defects in one sentence**: `[tests]` was not a linkable item, and *"twenty"* tempi was stale — the corpus has
eighty. `rustdoc -D warnings` caught the first; reading my own sentence caught the second.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does a caller do with a `None`?

**Critique.** `Tempo { bpm: Option<f32>, confidence, octave_ambiguous }`, and the absence is the design:

```rust
match tempo.bpm {
    Some(bpm) => println!("{bpm:.1} BPM"),
    None      => println!("no tempo (confidence {:.2})", tempo.confidence),
}
```

`confidence` is reported **even when the estimate is absent**, so a caller can see how close a refused asset came — the
difference between "this is not rhythmic" and "this is rhythmic and I could not read it".

**The floor is a method on the type**, `within_floor(truth) -> Option<bool>`, so a corpus test is one line and a caller
comparing two assets can ask the same question the crate asks. `is_octave_of` is separate, because `#24` asks for the two
failure kinds to be counted apart and conflating them is how an octave error gets mistaken for a bad estimator.

**What is deliberately not done**: tempo is not wired into `build` or the index. That is a schema section plus a field,
and it belongs with `#25` so the two features land in **one** schema change rather than two — the same reasoning that made
`provenance` and `loudness` separate decisions but a single shape.

**Verdict: GREAT.**
