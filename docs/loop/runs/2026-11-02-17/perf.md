# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.25 / 9.79 / 14.97** — high, and rising through the run. The numbers below are
noisier than iterations 14–15 for that reason and are read as bounds, not as point measurements.

| voices | µs/frame (after) | µs/frame (iter. 15/16) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.077 | 0.064 | 0.0096 |
| 64 | 0.370 | 0.351 | 0.0058 |
| 256 | 0.713 | 0.734 | 0.0028 |

**Not a regression, and provably not this change.** The fix is one `Option::is_some()` and an `&&` in `play_requests`, on
the path that **spawns** a voice — not in `update_mix`, which is what `perfcheck` costs. The 8-voice delta (+21 %) and
the 256-voice delta (−3 %) point in opposite directions, which is what a load of 8–15 does to a sub-microsecond
measurement; the example's own note says it costs every voice as an upper bound.

## The probe (the real before/after)

`evidence/backend{,-before}.txt`:

| row | before | after |
|---|---|---|
| `Backend(false)`, no `Output` | `AudioPlayer 1` | unchanged |
| **`Backend(true)`, no `Output`** | **`AudioPlayer 0`** — silent, unretirable, holds a slot forever | `AudioPlayer 1` |
| `Backend(true)`, with `Output` | `AudioPlayer 0` | unchanged — the mixer still owns it |
