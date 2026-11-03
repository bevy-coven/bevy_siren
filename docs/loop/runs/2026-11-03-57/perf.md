# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **6.3 / 5.7 / 6.1**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.071 | 0.082 | 0.063 |
| 64 | 0.390 | 0.358 | 0.358 |
| 256 | 1.435 | 0.995 | 0.726 |

**The change cannot touch these numbers.** `perfcheck` calls `approach`, `db_to_gain` and `chain_db` over a fixed gain
list; it builds no `MusicState` and sends no `Stinger` (`examples/perfcheck.rs:33-40`). This iteration added one boolean
to `arm_stinger_ducks`, which runs **once per stinger sent** — a handful of times in a session, not per frame.

**The spread is the host.** 1.435 → 0.726 µs at 256 voices from one binary is a 2× spread at load 6.3; iteration 51
measured the same 2× on a single binary at load 5.1. No regression and no speedup is claimed.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
