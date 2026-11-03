# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **5.7 / 4.4 / 4.5**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.354 | 0.543 |
| 2 | 0.101 | 0.395 |
| 3 | 0.341 | 1.406 |

Median 256 ≈ **0.543 µs**, against iteration 32's 1.056 — a drop this change cannot cause. The spread within one binary is
**3.6×** (0.395–1.406), which is the machine; `perfcheck` drives `model`'s arithmetic and `src/music.rs` is not on its path.

**What the change costs, where it runs.** `move_layers` gains a loop over `state.pending` — the number of **queued layer
changes**, which is at most one per request a game makes and is almost always 0 or 1 — and inside it a `tempo` comparison
that fails on every frame the score has not been edited. The `next_boundary` call only happens on a tempo change.

## The probe (the real before/after)

`evidence/tempo{,-before}.txt` — walking the clock until `target_db` moves, so the number is when the crossfade
**actually happens**:

| case | before | after |
|---|---|---|
| queued `NextBar` at 160 bpm, tempo unchanged | 1.44 s | 1.44 s (a bar is 1.5; one 16 ms tick) |
| queued `NextBar` at 80 bpm, tempo unchanged | 2.944 s | 2.944 s (a bar is 3.0) |
| **160 → 80 bpm** before the bar | **1.44 s — mid-bar** | **2.944 s — on the bar** |
| **160 → 240 bpm** before the bar | **1.44 s — a bar late** | **0.944 s — on the bar** |
