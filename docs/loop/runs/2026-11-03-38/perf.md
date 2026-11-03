# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **5.7 / 4.1 / 4.5**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.018 | 0.0023 |
| 64 | 0.114 | 0.0018 |
| 256 | 0.424 | 0.0017 |

**Lower than iteration 37's 1.219 at 256 by a factor of three, on an unchanged crate.** That is the machine, not the
change: no `src/` line moved, `perfcheck` does not run the example, and the same binary read 0.424 here against 1.219 an
hour ago. Reported as measured, not as an improvement.

## The probe (the real before/after)

`evidence/panel/`, plus a pixel scan of the still:

| | before | after |
|---|---|---|
| the still's header | `9 cues, 4 buses, 4 voices` | **`7 cues, 2 layers, 4 buses, 4 voices`** |
| cue rows beneath it | **7** | 7 |
| the fault table's header | `9 cues` (table's count) | **`9 cues, 2 layers`** |
| fault-table cue rows | 9 | **9** |
| lowest ink / margin | y = 709 / **11 px** | y = 709 / **11 px** |
| arithmetic edges | — | `9,2→(7,2)`, `2,2→(0,2)`, `1,2→(0,2)`, `0,0→(0,0)` |
