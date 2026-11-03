# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **6.1 / 6.2 / 5.2** —
the busiest of the last ten iterations.

| voices | run 1 | run 2 | run 3 | µs/voice |
|---|---|---|---|---|
| 8 | 0.064 | 0.070 | 0.056 | 0.0070–0.0088 |
| 64 | 0.372 | 0.392 | 0.328 | 0.0051–0.0061 |
| 256 | 1.367 | 1.381 | 1.268 | 0.0050–0.0054 |

**The change cannot touch these numbers.** `perfcheck` calls `approach`, `db_to_gain` and `chain_db` over a fixed gain
list; it builds no `SirenSettings`, no ducks and no cue table (`examples/perfcheck.rs:33-40`). This iteration added one
comparison inside `SirenSettings::problems()`, which a game calls when it draws a panel — not per frame.

**The 256 figure is the host, as iteration 51 established.** One identical binary ranged `0.673 … 1.384 µs` across six
runs at load 5.1; at load 6.1 every figure here sits in the top half of that spread. No regression is claimed and none
would be measurable: the code path is not executed by the probe.

**What the number means.** `perfcheck` measures the **crate's per-frame work as an upper bound**: every voice is costed,
including the ones `update_mix` skips because their gain did not move. It is not the audio thread.
