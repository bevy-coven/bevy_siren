# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **5.5 / 4.5 / 4.6**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.046 | 0.079 | 0.061 |
| 64 | 0.283 | 0.342 | 0.497 |
| 256 | 0.720 | 1.313 | 0.934 |

**Nothing in `src/` changed this iteration** — a guide paragraph, a CHANGELOG entry, a decision file and one test. The
probe cannot see a documentation change, and the spread above is the host: 0.720 → 1.313 µs at 256 voices from one
binary is 1.8×, in line with the 2× spread iteration 51 measured at load 5.1.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
