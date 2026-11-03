# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.0–4.7** across the runs below; a shared machine's load belongs in the numbers.

| voices | run 1 µs/f | run 2 µs/f | run 3 µs/f | µs/voice (best) |
|---|---|---|---|---|
| 8 | 0.064 | 0.056 | 0.063 | 0.0071 |
| 64 | 0.372 | 0.335 | 0.276 | 0.0043 |
| 256 | 1.049 | 1.392 | 1.273 → **0.416** | 0.0016 |

**This measurement is noise, and the change is not in it.** `perfcheck` neither runs `examples/mixer` nor calls
`Entry::line` (grep: 0 occurrences of either) — the two things this iteration changed. The 256-voice case swung
**0.416–1.392 µs** (3.3×) at the same load on the same binary; iteration 40 read 0.490 on a quieter machine. The spread is
the machine, not the commit.

**What the change actually costs.**

- `Entry::line` is called once per drawn log line — **six per frame**, in the example only, and it allocates a `String` per
  line exactly as it did before. Two extra decimal places format the same `f32`.
- The cue row's `distance_gain` call runs once per **spatial cue per frame** — one cue in the example, and it is already
  computed by `answer` on the frame of a play.
- `src/`'s per-frame work is unchanged: `update_mix`, `retire_voices` and `move_layers` were not touched.

**The probe that matters** — `evidence/after/render.txt`, the crate's own harness:

| | before | after |
|---|---|---|
| a placed cue in the panel | **none** — no `spatial` cue, no `.at(..)` | `bang … inverse at 8 m -6.02 dB` |
| the log's gain for a play | `at -6.0 dB` | `at -6.02 dB` |
| the row vs the log | row absent; log lossy | **the same number** |
| gain at 4 / 8 / 16 m | — | 1.000 / 0.500 / **0.250** |
| `bang` vs an identical flat fixture | — | **exactly 6.00 dB apart** (−6.087 vs −12.087 dB peak) |
| log lines (clean table) | 7, margin 11 px | **6, margin 9 px** — the 8th cue row cost the 7th line |
| `SpatiallyMute` recorded | in tests only | `refusal-bang-no-SpatialListener-to-place-a-sound-at--3--0--4-.txt` |
