# Iteration 90 — perf

## Per-frame: unchanged

| voices | µs/frame | iteration 89 |
|---|---|---|
| 64 | **0.329** | 0.353 |
| 256 | **0.630** | 1.265 |

No audio-path change; inside the ranges this code has shown across iterations 80–90.

## Indexing: unchanged

`semantic::filter` runs at query time over features already stored, so `build` is untouched this iteration. Iteration
89's **~1 150 clips/s** decode figure stands, and its regression (spectral analysis is a second FFT sweep) is still the
item that would need measuring if it ever matters.

## The cost of filtering, stated rather than measured into the noise

`evaluate` is a handful of `Option` matches and one subtraction per candidate, with no allocation for an undecided
pair. `filter` allocates one `Vec` for the kept indices. On a result set of `k` — which `rank::top_k` already bounds —
that is not a cost worth a benchmark, and claiming one from a microbenchmark of two subtractions would be noise dressed
as a number.
