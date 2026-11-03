# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **3.9 / 5.1 / 5.5**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.062 | 0.063 | 0.067 |
| 64 | 0.507 | 0.329 | 0.397 |
| 256 | 1.249 | 1.301 | 1.312 |

**`perfcheck` does not run the changed code** — it calls `approach`, `db_to_gain` and `chain_db` over a fixed gain list
(`examples/perfcheck.rs:33-40`), and builds no `MusicState`.

**Does the change add per-frame work?** `move_layers` now does one `find` where it did one `find`: `if let Some(..)` became
`let Some(..) else { … continue }`, so a miss writes a log line and *skips the same work as before*. There is no extra
lookup and no extra work on the happy path — the only addition is one `SirenLog::push` per **misspelled target per
fire**, which is an authoring mistake, not a steady state.

**The 64-voice spread (0.329–0.507 µs, 54 %) is the host** at load 5.5; iteration 51 measured a 2× spread on a single
binary.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
