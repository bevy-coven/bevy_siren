# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **10.0 / 7.4 / 7.7**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.364 | 0.910 |
| 2 | 0.095 | 0.619 |
| 3 | 0.415 | 1.143 |

Median 256 ≈ **0.910 µs**, against iteration 27's 1.338 — a 32 % *drop*, which a change that adds work cannot cause. The
spread within one binary here is **1.8×** (0.619–1.143), and `perfcheck` runs on `MinimalPlugins` with **no `Output`**, so
`start_pending` returns at its first `else` and **none of this change executes**. The number is the machine; it is
reported as measured, not as an improvement.

**What the change costs, where it does run:** `start_pending` gains one `Query` and one `retain` over the starts this
system made — the number of voices the mixer is playing, which is bounded by `max_voices` (32 by default). Once per
frame, and only when a mixer exists.

## The probe (the real before/after)

`evidence/mixer-slot{,-before}.txt`, a fake backend recording `stop_voice`:

| | before | after |
|---|---|---|
| mixer voices started | 1 | 1 |
| owner alive, 10 frames | `stop_voice []` | `stop_voice []` (guard) |
| owner despawned, 10 frames | **`stop_voice []`** — leak | **`stop_voice [0]`** — slot freed |
| `SirenMix::voices_on("Effects")` | 0 | 0 |
| log | `["played engine …"]` | `["played engine …"]` — the record gap, unchanged and named |
