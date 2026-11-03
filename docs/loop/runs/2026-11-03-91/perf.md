# Iteration 91 — perf

## Per-frame: unchanged

| voices | µs/frame | iteration 90 |
|---|---|---|
| 64 | **0.282** | 0.329 |
| 256 | **1.055** | 0.630 |

No audio-path change; inside the ranges this code has shown across iterations 80–91.

## Indexing: unchanged this iteration

`semantic::tempo` is not yet called by `build` — the wiring is deliberately deferred so tempo and key land in one schema
change. Iteration 89's **~1 150 clips/s** decode figure therefore stands untouched.

## The cost tempo will add, measured rather than estimated

The library test suite now takes **14.8 s** for 68 lib tests, of which the tempo corpus is most: **80 click tracks**,
each analysed frame by frame. That is the analysis cost in the open — a 4 s track needs ~400 frames of FFT plus an
autocorrelation over the 80–160 BPM lag range, and 80 of them is the corpus.

The honest statement: this is **not** measured as clips/s in `indexbench` because tempo is not in the build path yet.
When it lands there, `indexbench` will show it, and iteration 89's precedent is that a second pass over the same frames
is expensive enough to be the headline.
