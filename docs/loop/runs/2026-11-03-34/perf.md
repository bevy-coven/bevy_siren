# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.5 / 6.3 / 5.3**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.356 | 1.337 |
| 2 | 0.320 | 1.095 |
| 3 | 0.391 | 0.821 |

Median 256 ≈ **1.095 µs**, against iteration 33's 0.543 — opposite direction on a machine whose load average moved from 5.7
to 8.5 between the two runs. The spread within one binary here is **1.6×** over three runs. `perfcheck` runs on
`MinimalPlugins` with no `MusicPlugin`, so `start_missing` — where this change lives — never executes.

**What the change costs, where it runs.** `start_missing` gains one `cue.bus_name() != "Music"` comparison per **layer
without a voice** — two layers in the example, a handful in a project — on the frame a score is loaded or a table changed.
A layer that is playing short-circuits on `layer.voice.is_some()` before the check.

## The probe (the real before/after)

`evidence/wrong-bus{,-before}.txt`:

| | before | after |
|---|---|---|
| `score.problems(table)` | *(nothing)* | `music layer \`calm\` names cue \`wrong.bus\`, which plays on bus \`Effects\`; a layer is a stem of the \`Music\` bus` |
| live voices | `wrong.bus@Effects`, `track.ok@Music` | `track.ok@Music` |
| `MusicState::live()` | **2** | **1** |
| `SirenMix` voices | Effects **1**, Music 1 | Effects **0**, Music 1 |
| log | two `played` lines | one `played`, one `refused` naming layer, cue and bus |
