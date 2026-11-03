# Iteration 104 — perf

Machine: this Mac, shared and loaded. Nothing in this iteration is on the per-frame path.

## Per-frame audio cost (control)

| voices | this run |
|---|---|
| 8 | 0.067 µs/frame |
| 64 | 0.382 |
| 256 | 1.253 |

Inside the band iteration 102 recorded across runs of unchanged code (256 voices: **0.401–1.339**), so no claim is made
from them. Iteration 103 recorded that spread explicitly, and this run is consistent with it rather than with any change.

## What the fix costs

The category comparison is **one extra `to_lowercase()` per candidate** — a short allocation for a needle that is at most
a pack name. On a 10 000-asset effect library that is 10 000 allocations per filtered query, which is not nothing.

**Is it worth it?** Yes, and for a reason a benchmark would not show: without it the filter returns nothing and the caller
believes their library is empty. A correct answer that allocates beats a wrong one that does not.

A caller who cares can hoist it — the needle is fixed for the whole call, so lowercasing it **once** outside the loop is
the obvious improvement, and it is what the code would do if this were hot. It is left inside because moving it would
require the comparison to take a pre-lowercased parameter, which is a worse signature for a function with one caller.

## The floor

`weights.min_similarity` replaces `Floor::default().min_score` — a field read instead of a constant, so the cost is
identical to before. `below_floor` can now be non-zero, which is a counter increment that already existed.
