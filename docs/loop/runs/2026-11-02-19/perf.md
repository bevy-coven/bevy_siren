# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.07 / 9.79 / 13.07** — loaded throughout, and worse than iteration 18's first sample.

| voices | µs/frame (after) | µs/frame (iter. 18) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.064 | 0.079 | 0.0080 |
| 64 | 0.361 | 0.357 | 0.0056 |
| 256 | 1.311 | 1.288 | 0.0051 |

**Within noise in both directions** — the 8-voice row is *faster* than iteration 18's and the 256-voice row is 1.8 %
slower, in a window whose load average was 13.07 against 8.72. The example costs every voice as an upper bound, so a
loaded window inflates the largest count.

**This change is not on that path.** `finite_or` is called when a layer is **created** and when a `MusicLayers` message
is **read** — a handful of layers, once per score edit or per move — not per frame per voice. `update_mix` and the
per-voice loop are untouched.

## The probe (the real before/after)

`evidence/music-gain{,-before}.txt`:

| row | before | after |
|---|---|---|
| a score layer at `gain_db: NaN` | `layer_db NaN`, `base_db NaN`, `Applied −80.0`, `problems []` | `layer_db 0.0`, `base_db 0.0`, `Applied 0.0`, one sentence |
| a move to `NaN` | `layer_db NaN`, `target NaN` | `0.0`, `0.0` |
| a layer started at `SILENT_DB` (guard) | −80 dB | −80 dB |
