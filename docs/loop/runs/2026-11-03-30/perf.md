# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.9 / 5.7 / 6.4**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.058 | 0.0073 |
| 64 | 0.318 | 0.0050 |
| 256 | 1.301 | 0.0051 |

**Unchanged.** This iteration touches `examples/mixer.rs` only — one bounded `match` arm and a capture knob — so no `src/`
line moved and the crate's per-frame work is identical in source. The rows are within 3 % of iteration 29's.

**The panel's own cost, which is what changed:** `problems_in` is still called once per frame, and the block now stops
after three sentences instead of formatting all of them — so the fault-heavy case does strictly **less** work per frame.
Across the captures: 8.34–8.84 ms for six faults, 8.35 for two, 8.34 for the clean table — all inside the run-to-run
spread, and none of it is the crate's.

## The probe (the real before/after)

`evidence/panel/`, 1280×720:

| faults | before | after |
|---|---|---|
| **six** (`SIREN_FAULTS=many`) | six sentences; **`Log` heading and ten lines off screen** | three sentences + `and 3 more; …`; **log on screen** |
| **two** (`SIREN_FAULTS=1`, the README GIF) | both sentences, log present | **unchanged** — the bound does not fire |
| **none** | `Problems` / `none` | unchanged |
