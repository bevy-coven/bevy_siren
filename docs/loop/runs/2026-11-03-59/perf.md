# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **6.4 / 5.0 / 5.1**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.062 | 0.066 | 0.062 |
| 64 | 0.355 | 0.407 | 0.356 |
| 256 | 1.232 | 1.289 | 1.290 |

**The change cannot touch these numbers.** `perfcheck` calls `approach`, `db_to_gain` and `chain_db` over a fixed gain
list; it builds no `SirenMix` and calls no peak accessor (`examples/perfcheck.rs:33-40`). `peak_db` is one
`voices_on` lookup plus a `then`, and it is a **query** a meter makes when it draws — not per-frame crate work.

**The 8- and 256-voice rows are flat to 6 % and 5 %**, which is as steady as this host gets at load 6. The 64-voice
row's 15 % spread is the host.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
