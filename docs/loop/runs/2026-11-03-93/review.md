# Iteration 93 — review

## Pass 1 — Function: are tempo and key now *produced*?

**Critique.** The gap was exact: both features were built and tested, and `build` called neither. The test that settles it
asserts a **number a caller did not supply**:

| Claim | Assertion | Why it is not vacuous |
|---|---|---|
| a build produces tempo | `index.tempo.len() == 1` and **120 BPM ±2** from 0.5 s clicks | a stored field would be zero |
| a build produces key | `index.key.len() == 1` with `tonic == None` for a click train | a click train genuinely has no key, and *recording that* is the feature |
| both survive | round trip in **both** formats | the text and binary sections are separate code |

**The three-state distinction is the design**, and it is what the second test was rewritten to assert:

| asset | tempo record | key record |
|---|---|---|
| a 4 s click track | `bpm: Some(120.0)` | present, `no key` |
| a 0.1 s clip | present, `bpm: None` | **absent** (shorter than a chroma window) |
| an asset never measured | absent | absent |

A `None` *inside* a record and *no record* are different facts, and `#27` reads the first as **unjudged** rather than
**unknown**. Collapsing them would make every short asset look unmeasured.

**Verdict: GREAT.**

## Pass 2 — Zoom: the cost, and the assertion that was wrong

**Critique.** The regression is the headline and it is measured twice: **419.8** and **568.8 clips/s** against **~1 150**
before. Two passes over every file — tempo's autocorrelation and key's 8192-point transform — and neither shares
arithmetic with the mel path.

That is the case this iteration targeted, so it is a trade rather than a leak, and 13 s for 5 000 clips remains far
below the ~3 minutes the audio tower would take. **But it is the third consecutive iteration where a feature added a
pass**, and saying so is more useful than a footnote would be.

**The defect in my assertion.** I claimed a 0.1 s clip would have no tempo record. It has one: 4 800 samples exceeds
tempo's two 1 024-sample frames, so tempo records and reports `bpm: None`; only key skips the record. **The test now
asserts all three states rather than two**, which is a stronger test than the one I set out to write.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does a consumer do differently?

**Critique.** Nothing, except that `index.tempo` and `index.key` are now populated rather than empty. A filter reads
them:

```rust
let tempo = index.tempo.get(&hash).and_then(|t| t.bpm);   // None → unjudged
let key   = index.key.get(&hash).map(|e| e.key);          // None → unknown; Some(absent) → judged, no key
```

That is the two-level absence — `Option` inside the map and `Option` inside the record — and it is the shape `#27`'s
criteria need. An index written before this iteration loads unchanged, because **an empty section is not written**: the
additive promise holds for the fifth and sixth time.

**The honest limit**, stated in `perf.md`: the scan is now 2–2.7× slower and the fix — one shared spectral pass — is
named and unbuilt.

**Verdict: GREAT.**
