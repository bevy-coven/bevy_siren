# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.8 / 3.9 / 4.8**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.059 | 0.0074 |
| 64 | 0.322 | 0.0050 |
| 256 | 1.219 | 0.0048 |

**Unchanged.** This iteration touches `examples/mixer.rs` only — the capture's sampling — and no `src/` line. The 256-voice
figure matches iteration 36's 1.069 within the 2.4× spread this binary shows.

**What the change costs, where it runs.** `capture` gains one `elapsed_secs_f64()` read and a float comparison per frame,
and spawns **fewer** screenshots than before at the default (`every` 2 hundredths = 20 ms, versus every 2nd update ≈ 17 ms —
comparable). A denser capture is a debug-only choice.

## The probe (the real before/after)

A per-frame scan of the capture directory, measuring the music line's ink width (an arrow makes the line **wider**):

| | before | after |
|---|---|---|
| GIF frames | 20 | **46** |
| `SIREN_SHOT_EVERY` means | updates (12 ≈ 100 ms, but machine-dependent) | hundredths of a second (4 = 40 ms) |
| samples per 50 ms fade | **0.5** | **1.25** |
| frames carrying the arrow | 1, by luck of the phase | **11 consecutive** (`f_0028`–`f_0038`) |
| the crop, read | — | `calm -76.2 -> -80.0   combat -6.9 -> -3.0` |
