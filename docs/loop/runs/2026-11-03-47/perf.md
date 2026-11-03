# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **10.9 / 7.9 / 5.6** — by far the busiest of any iteration this loop has recorded.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.065 | 0.066 | 0.0081 |
| 64 | 0.362 | 0.252 | 0.0039–0.0057 |
| 256 | 1.353 | **0.580** | 0.0023–0.0053 |

**The spread is the host.** At load 10.9 the same binary read 1.353 and 0.580 µs at 256 voices a minute apart; every
iteration since 41 shows this envelope (0.40–1.42). `perfcheck` has no duck table at all and never runs `examples/mixer`.

**What the change costs.** One `String` comparison per **duck rule** per `problems()` call — a report an editor draws on
a status line, not a per-frame path. `update_mix` is untouched.

**The probe** — `evidence/panel/broken-40.png`, read:

| | before | after |
|---|---|---|
| a self-duck's depth on the frame its own voice starts | −6.0 dB, **unreported** | unchanged, **named** |
| the bus it mixes | 6 dB lower | unchanged |
| sentences `problems()` can emit | 10 | **11** |
| the log, same frame | nothing about it | `sword.hit ducked Effects by -6.0 dB` |
| the sentence's width | 118 chars — **wrapped onto two lines**, pushed the log to `and 1 more` | **78 chars**, one line |
| tests | 124 / 131 / 134 + 5 | **126 / 133 / 136 + 5** |
