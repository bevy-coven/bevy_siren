# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **9.3 / 6.8 / 5.7**.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.062 | 0.022 | 0.0027–0.0078 |
| 64 | 0.350 | 0.119 | 0.0019–0.0055 |
| 256 | 1.265 | **0.643** | 0.0025–0.0049 |

**The spread is the host**, as it has been every iteration since 41 (0.40–1.42 at 256 voices). `perfcheck` has no bus
with a cap at all and never runs `examples/mixer`.

**What the change costs.** One `u16` comparison per **bus** per `problems()` call — a report an editor draws on a status
line. `update_mix` and `bus_voice_limit` are untouched.

**The probe** — `evidence/panel/overbudget-40.png`, read:

| | before | after |
|---|---|---|
| `Effects: 16` under `max_voices: 4` | mixes **4**, report silent | mixes 4, **named** |
| the sentence | — | `bus \`Effects\` asks for 16 voices but the table's max_voices is 4, so it mixes 4` |
| the bus rows in the same capture | `Effects 2 / 12` | `Effects 2 / **4**` — the table's budget, agreeing with the sentence |
| a bus at `0` (the sentinel) | silent | **still silent** |
| a bus at exactly the table's cap | silent | **still silent** |
| sentences `problems()` can emit | 11 | **12** |
| the log in that capture | 6 lines | 6 lines — the block is one line, so the count rule gives the log its room |
| tests | 126 / 133 / 136 + 5 | **127 / 134 / 137 + 5** |
