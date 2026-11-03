# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.8 / 9.6 / 13.8**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.110 | 0.405 |
| 2 | 0.353 | 1.267 |
| 3 | 0.341 | 1.349 |

Median 256 ≈ **1.267 µs**, against iteration 23's 1.391 — within the 3.4× spread this binary has shown on this machine.
**The change is not on this path**: `SirenLog::push` is called once per `Played`/`Refused`/`Stopped`/`Ducked`, and
`perfcheck`'s per-voice loop writes no log entry.

## What the change does cost, measured directly

| workload | time | per push |
|---|---|---|
| 200 000 pushes, all routine (the common case) | 11.1 ms | **55 ns** |
| 1 000 pushes into an all-refusal log (pathological) | 272 µs | **272 ns** |

The common case finds index 0 immediately — the oldest entry is routine — so the scan is O(1) there. The pathological case
scans 256 each push, and is reachable only when every one of the 256 entries is a refusal, which is a game already
failing loudly.

## The probe (the real before/after)

`evidence/refusal-lifetime{,-before}.txt`:

| | before | after |
|---|---|---|
| one refusal, then plays | evicted after **256 plays** — 0.40 s at 640 entries/s | survives **2000**; leaves at 256 *routine* entries |
| 600 refusals | len 256, oldest `c344` | len 256, oldest `c344` (guard) |
| buffer size | 256 | 256 (guard) |
