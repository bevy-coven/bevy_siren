# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.12 / 14.44 / 19.28** — the 15-minute figure was rising through the run, so this
window is noisier than iterations 18–19.

| voices | µs/frame (after) | µs/frame (iter. 19) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.046 | 0.064 | 0.0058 |
| 64 | 0.327 | 0.361 | 0.0051 |
| 256 | 0.991 | 1.311 | 0.0039 |

**Every row is *faster* than iteration 19's**, which is not attributable to this change and should not be read as an
improvement: `perfcheck` costs the per-voice mix loop, and **nothing in this diff runs there**. The figures are reported
as measured, with the load noted, because a shared machine's 15-minute average drifting from 13 to 19 is the more likely
explanation for a sub-microsecond spread.

**This change is not on that path.** `problems()` is a **host** call — the panel runs it when it draws, not every frame —
and it now does one `bus_names()` (a `Vec` of four `&str`) plus a set insert per rule. The mix loop is untouched.

## The probe (the real before/after)

`evidence/report{,-before}.txt`:

| table | before | after |
|---|---|---|
| default (all four buses) | 0 sentences | 0 (guard) |
| `buses: []` (the crate's four) | **4**, all false | **0** |
| two buses (menu mix) | **2** identical for `Dialogue` | **1** |
| one bus + two rules naming `Ghost` | **2** identical | **1**, exact words |
