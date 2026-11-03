# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **7.4 / 5.9 / 4.4** — the busiest of the last six iterations, which is why the two runs
differ so much.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.047 | 0.057 | 0.0058–0.0071 |
| 64 | 0.165 | 0.325 | 0.0026–0.0051 |
| 256 | **0.406** | 1.313 | 0.0016–0.0051 |

**The spread is the host, not the commit.** The same binary read 0.406 and 1.313 µs at 256 voices minutes apart at load
7.4; every iteration since 41 has shown this envelope (0.40–1.42). `perfcheck` builds no duck table with a self-rule
and never runs `examples/mixer`.

**What the change costs.** One boolean expression per **duck rule** per call to `problems()`, which is a report an
editor draws on a status line — not a per-frame path. `update_mix` is untouched; no system was added.

**The probe** — `evidence/panel/broken-40.png`, read:

| | before | after |
|---|---|---|
| a rule from `Duck::new(a, b).by_db(-8.0)` | fires, logs, ducks **0.0 dB** for ever | — (behaviour unchanged, and deliberately) |
| the same rule with `.hold_s(0.25).fade_s(0.6)` | `-8.0 dB` | `-8.0 dB` |
| what `problems()` says about the bare rule | **nothing** | `the duck on \`Music\` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s` |
| sentences the report can emit | 9 | **10** |
| sentences `problems_names_every_mistake` asserts | 9 | **10** |
| margin of the broken-table capture | 9 px | **9 px** |
| tests | 123 / 130 / 133 + 5 | **124 / 131 / 134 + 5** |
