# Iteration 102 — perf

Machine: this Mac, **shared and loaded** — the variance below is the machine's, not the change's, and it is reported
rather than smoothed.

## Per-frame audio cost

No system reads the new code. `budget::defer` runs **once per model load**, never per frame, so this is a control.

| voices | run 1 | run 2 | run 3 | prior iterations |
|---|---|---|---|---|
| 8 | 0.057 | 0.057 | — | 0.041 |
| 64 | 0.325 | 0.323 | 0.107 | 0.327–0.329 |
| 256 | 0.401 | 0.471 | **1.263** | 1.132–1.228 |

**Read this honestly.** The 256-voice figure spans **0.401 to 1.263 µs per frame** across three runs of *identical code* —
a 3.1× spread. The 0.107 and 0.401 above are the flattering end of that spread, not an improvement; the 1.263 matches
every prior iteration. On a shared host a single 256-voice measurement cannot resolve a 10 % change, and this change is
not on the path anyway. The 64-voice row is the stable one (0.323–0.329 across five iterations) and it did not move.

**The crate must not claim a speedup it did not produce**, and prior iterations reporting "unchanged" on a 3× spread have
been reporting the machine's luck rather than a result. That is worth saying here rather than in a footnote.

## What the wait costs

The deferral's cost is **wall-clock, not CPU**: `defer` sleeps between polls, and it does so only while a device is
genuinely short. At the default `poll_s = 0.5` a deferral adds at most **0.5 s** of latency past the moment room appears,
which is the number `Deferral::poll_s`'s doc states.

The measurement itself is `Budget::fits` — three comparisons and no allocation — repeated at most `wait_s / poll_s` = **60**
times. Sixty comparisons over thirty seconds is not a number worth benchmarking.

## What it protects

A refused indexing run does not just cost a retry: it wastes the decode work already done. On the shared 3090 the cost of
*not* waiting is a user who closes the editor to run an index, which is the failure `#37` exists to remove.
