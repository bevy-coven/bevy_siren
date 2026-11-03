# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.6 / 4.1 / 4.2**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.063 | 0.0079 |
| 64 | 0.357 | 0.0056 |
| 256 | 1.355 | 0.0053 |

**Unchanged in the crate**: no `src/` line moved, and `perfcheck` does not run the example. The 256 figure (1.355 µs) is
against iteration 38's 0.424, a three-fold swing on an unchanged crate — the machine.

**What the change costs, where it runs.** `draw_panel` calls `table.problems_in(&settings)` **twice** per frame now — once
for the `Problems` block, once to choose the log's line count. It was already called once. On an eleven-cue table that is
a few hundred nanoseconds on a view that costs 8.4 ms.

## The probe (the real before/after)

`evidence/panel/`, plus a pixel scan of each still:

| | before | after |
|---|---|---|
| a refusal in the log | **never** — no step refuses | `0.83s refused \`no.such.cue\`: no cue of that name` |
| the default script | `steal,layer,duck` | `steal,layer,duck,**refuse**` |
| the clean still's margin | 11 px | **11 px** |
| the fault table's log, 7 lines | — | **1 px — clipped** |
| the fault table's log, 4 lines | — | **11 px** |
| GIF width | 960 (down-scaled) | **1280** (native) |
