# Iteration 97 — perf

Offline retrieval calls again: no system schedules them, so the runtime figures are a control.

## Per-frame audio cost (control)

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.041 | 0.0051 |
| 64 | 0.329 | 0.0051 |
| 256 | 1.228 | 0.0048 |

Unchanged from iteration 96.

## What the calls cost

`compatible_music` / `compatible_sfx` iterate the index once — the same walk `top_k` already does — and then apply the
variant rule, which is **O(k²·d)** over the results actually kept: `k` is typically 3, so 9 cosine evaluations of 512
values. Negligible beside the single pass over the index, and it runs once per user action rather than per frame.

`explain_match` is **O(d)**: two hash lookups and one cosine.

## What changed in the arithmetic

The scale went from `[0, 1]` to `[-1, 1]`, which is the same number of operations — one multiply and one clamp per
signal either way. Nothing in this iteration is a performance change.
