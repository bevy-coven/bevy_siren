# Iteration 94 — perf

## Per-frame: unchanged

| voices | µs/frame | iteration 93 |
|---|---|---|
| 64 | **0.327** | 0.323 |
| 256 | **1.132** | 1.119 |

No audio-path change; inside the ranges this code has shown across iterations 80–94.

## Indexing: unchanged this iteration

The tempo and key rules read features already stored — `build` is untouched — so iteration 93's **420–569 clips/s**
decode figure stands, and so does its regression.

## The filter's own cost

Two more comparisons per candidate: an `Option` match and a subtraction for tempo, a `relation` call and a table lookup
for key. Both are branch-predictable and allocation-free, on a result set `rank::top_k` already bounds. A microbenchmark
of two subtractions would be noise dressed as a number, and the honest statement is that the filter remains O(k) with a
small constant.
