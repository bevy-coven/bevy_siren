# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.2 / 4.6 / 5.3**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.378 | 0.590 |
| 2 | 0.345 | 1.069 |
| 3 | 0.361 | 1.409 |

Median 256 ≈ **1.069 µs**, against iteration 35's 1.235 — inside the 2.4× spread this binary shows on an unchanged
workload. **No `src/` line moved**: the change is `examples/mixer.rs` only, and `perfcheck` does not run the example.

**What the block costs, where it runs.** `draw_panel` gains one `MusicState` read and formats one line from
`names()`/`layer_db()`/`target_db()` — two layers, once per frame, on a debug view that already costs 8.3 ms. The cue
filter builds a `Vec<String>` of the score's cue names per frame; two entries.

## The probe (the real before/after)

`evidence/panel/`, plus a pixel scan of each still:

| | before | after (row per layer) | after (one line, 7 log) |
|---|---|---|---|
| `Music` block | *absent* | 2 rows | **1 line** |
| cue rows | 9 | 7 | 7 |
| log lines | 8 | 8 | **7** |
| lowest ink | y = 708 | **y = 719 — clipped** | **y = 709** |
| margin | 12 px | **1 px** | **11 px** |
| crossfade visible | no | yes | yes — `calm -79.9 -> -80.0   combat -3.1 -> -3.0` |
