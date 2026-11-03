# Iteration 96 — perf

This iteration adds **offline retrieval arithmetic**, called once per search over at most `k` results. It touches no
per-frame system, so the runtime figures are a control rather than a target.

## Per-frame audio cost (control — must not move)

| voices | µs/frame | µs/voice | iteration 95 |
|---|---|---|---|
| 8 | 0.041 | 0.0051 | same |
| 64 | 0.329 | 0.0051 | 0.327 / 0.0051 |
| 256 | 1.228 | 0.0048 | 1.132 / 0.0044 |

The 256-voice figure is **8 % above** iteration 95's and within the run-to-run spread this machine shows on a shared
host (iteration 95 measured 1.132 against iteration 93's 1.150 for unchanged code). Nothing in this iteration's change is
on that path: `search_ranked` is reached only from a search call, and no system schedules it.

## Retrieval cost

Not benchmarked, and deliberately so: the work per result is two subtractions, a multiply and a clamp on four signals —
**O(k·4)** with `k` typically 3. A microbenchmark of that would be noise dressed as a number. The honest statement is
that the ensemble adds a constant, small amount per returned result and adds **no pass over the index** — `top_k` already
scanned it, and this reorders only what `top_k` returned.

## What the change costs the caller

`search_ranked` takes a `Ranked<'_>` rather than eight positional arguments. That is a type, not a cost: it borrows the
embedding, the feature map and the weights, and allocates only the `Vec<Scored>` whose length is `k`.
