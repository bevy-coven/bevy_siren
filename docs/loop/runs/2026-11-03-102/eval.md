# Iteration 102 — eval

## Baseline before anything

Nothing red: 198 / 6 / **421 semantic** / **109 lib** / 6 doc, clippy and fmt clean, docs and examples build.

## The cue cases

**All 33 of `cues.yaml` are green**, unchanged. This iteration touches no cue path — it adds a policy for a model load,
which no cue reaches.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| a short device is refused naming request and availability | **green before** | `Budget::fits` already carried both numbers; re-asserted **at the policy level** and against the message's own MB figures |
| the policy is stated and observable | **red before** | no `Deferral`, no `WaitState`, no `defer` |
| a busy device that frees succeeds on the next poll | **green now** | attempt **3** of 60, two polls, states `Waiting(2)`, `Waiting(3)`, `Ready` |
| a device that never frees gives up after the stated bound | **green now** | exactly **60** looks, **29.5 s** slept, `GaveUp` |
| a query still works while the build waits | **green now** | a real `top_k` runs between polls and the index is intact after |
| a device with room does not wait | **green now** | attempt 1, zero sleeps, nothing reported |
| the attempt count is derived, never zero | **green now** | 60; and 1 for a zero, negative or NaN poll |
| `Session::admit_deferring` is wired | **green now** | a real session comes back carrying its cost |
| a wrong dimension is still named after the wait | **green now** | `DimensionMismatch { graph: 768, expected: 512 }`, not a budget problem |

## The item

`src/semantic/budget.rs` — `Deferral`, `WaitState`, `defer`. `src/semantic/session.rs` — `Session::admit_deferring`,
and the dimension check extracted so both entry points share it.
