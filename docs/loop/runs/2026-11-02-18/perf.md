# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.72 / 4.85 / 4.83** — a spike in the first sample.

| voices | µs/frame (after) | µs/frame (iter. 17) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.079 | 0.077 | 0.0099 |
| 64 | 0.357 | 0.370 | 0.0056 |
| 256 | 1.288 | 0.713 | 0.0050 |

**The 256-voice figure is not comparable** and is reported as measured rather than smoothed: the first load sample was
8.72 (against 4.85 and 4.83 either side), and this example costs every voice as an upper bound, so a loaded window
inflates exactly the largest count. The 8- and 64-voice rows — the ones a loaded machine moves least — are within 3.5 %
and *faster* than iteration 17's.

**This change is not on that path.** `finite_or` and `played_semitones` are called in `answer`, **once per play**, not
per frame per voice; `update_mix` and the per-voice loop are untouched. A per-frame regression is not reachable from
this diff.

## The probe (the real before/after)

`evidence/nan{,-before}.txt`:

| row | before | after |
|---|---|---|
| a cue authored `gain_db: NaN` | `gain -80.00`, **silent** | `gain 0.00` |
| `Play::gain(NaN)` | `gain -80.00` | `gain 0.00` |
| `Play::gain(-∞)` | `gain -80.00` | `gain 0.00` |
| `Play::pitch(NaN)` | `pitch NaN` for a `1.0×` sound | `pitch 0.00` |
| `Play::pitch(1e9)` | `pitch 1000000000.00` for a `16×` sound | `pitch 48.00` |
