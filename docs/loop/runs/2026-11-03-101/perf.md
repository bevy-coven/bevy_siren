# Iteration 101 — perf

Offline analysis, once per clip at index time. No system schedules it.

## Cost

`estimate` reads **frame energies from `analysis::spectra`**, the shared pass iteration 95 built. It therefore adds **no
transform**: the cost is one pass over ~400 `f32`s (a max and two sums) on top of a pass the build already runs.

That was the reason for choosing an envelope measure over a spectral-flux one — the frames were already computed, and
recomputing them for one scalar would have been the duplication iteration 95 removed.

## Per-frame audio cost (control)

Unchanged, and nothing in this iteration is on that path: 8 / 64 / 256 voices at 0.041 / 0.329 / 1.228 µs per frame.

## What it would cost per clip

A 0.5 s clip has ~50 frames, so the arithmetic is ~50 comparisons and two sums — well under a microsecond, against the
**~1.3 ms** a clip's four analyses take. It is not separately benchmarked because it is not separable from the shared
pass that produces its input.
