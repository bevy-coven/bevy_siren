# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.5–2.7** — the quietest run of the last five iterations.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.070 | 0.069 | 0.0086 |
| 64 | 0.402 | 0.390 | 0.0061 |
| 256 | 1.380 | 1.360 | 0.0053 |

**That figure is higher than iteration 44's, and it is not this commit.** Measured directly, alternating the working
tree with `git stash`ed HEAD on the same machine minutes apart, 256 voices read:

| | run 1 | run 2 |
|---|---|---|
| HEAD (unmodified crate) | 1.307 µs | **0.537 µs** |
| working tree (this iteration) | 0.455 µs | 1.424 µs |

Overlapping ranges on an **unchanged `src/`** — this iteration touched one doc comment in `src/mix.rs` and nothing
executable. The swing is the host, not the code, and `perfcheck` does not run `examples/mixer` at all.

**What the change costs.** Nothing measurable: `src/mix.rs`'s only edit is a doc comment; the panel's extra
`settings.problems()` call runs in the example, over a two-bus table.

**The probe** — `evidence/panel/` and `evidence/after/render.txt`:

| | before | after |
|---|---|---|
| a mix table with a NaN master | `Problems: **none**` while every bus read −80 dB | `master_db is not a number` |
| a duck whose hold is negative | not drawn | `the duck on \`Effects\` has a hold or fade that is negative…` |
| sentences reached by a test | **5 of 9** | **9 of 9** |
| the doc's list | six of nine | nine of nine |
| 12 cue rows (`SIREN_FAULTS=many`) | log pushed **off the bottom** (heading not on screen) | `and 6 more`, log visible |
| margins, clean / broken / many | 9 / 0 / 0 px | **9 / 9–10 / 10 px** |
| tests | 122 / 129 / 132 + 5 | **123 / 130 / 133 + 5** |
