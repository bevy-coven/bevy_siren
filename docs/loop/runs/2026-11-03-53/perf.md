# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **7.1 / 7.5 / 6.4** —
the highest this loop has measured.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.021 | 0.056 | 0.062 |
| 64 | 0.109 | 0.321 | 0.354 |
| 256 | 0.935 | 1.294 | 1.345 |

**The change cannot touch these numbers.** `perfcheck` calls `approach`, `db_to_gain` and `chain_db` over a fixed gain
list; it constructs no `MusicScore` and reads no `beats_per_bar` (`examples/perfcheck.rs:33-40`). This iteration replaced
a `derive(Default)` with a handwritten one — a change to a value, not to any code a frame runs.

**The spread is the host.** Run 1 at 256 voices reads `0.935 µs` and run 3 `1.345 µs` from the same binary, a 44 % spread
at load 7.1; iteration 51 recorded a **2× spread** on one binary at load 5.1. No regression or speedup is claimed, and
none would be measurable.

**What the number means.** `perfcheck` measures the **crate's per-frame work as an upper bound**: every voice is costed,
including the ones `update_mix` skips because their gain did not move. It is not the audio thread.
