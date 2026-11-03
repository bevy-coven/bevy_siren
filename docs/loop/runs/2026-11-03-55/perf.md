# perf.md — the new retain, and the frame at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **7.8 / 9.1 / 7.9** —
the highest this loop has recorded.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.070 | 0.021 | 0.038 |
| 64 | 0.373 | 0.109 | 0.245 |
| 256 | 1.420 | 1.031 | 0.584 |

**`perfcheck` does not run the changed code** — it calls `approach`, `db_to_gain` and `chain_db` over a fixed gain list
(`examples/perfcheck.rs:33-40`). The **2.4× spread at 256 voices from one binary** is the host at load 9. Iteration 51
measured a 2× spread on a single binary at load 5.1; nothing here is a regression and nothing is a speedup.

## The new work, measured directly — and a regression caught by measuring

`retain_cues` runs from `play_requests`, so its cost is real per-frame work and had to be measured rather than assumed.

**The obvious implementation was 116× too slow.** `CueTable::get` is a linear scan (`self.cues.iter().find(...)`), so
`self.pickers.retain(|name, _| table.get(name).is_some())` is O(pickers × cues) **every frame**:

| cues | pickers | naive retain |
|---|---|---|
| 10 | 10 | **1.5 µs** |
| 100 | 100 | **20.6 µs** |
| 1000 | 1000 | **989.8 µs** — 6 % of a 60 Hz frame |

Replaced with one pass into a `HashSet<&str>` and a retain against it — O(cues + pickers):

| cues | one-pass retain |
|---|---|
| 10 | 0.36 µs |
| 100 | 2.7 µs |
| 1000 | **8.5 µs** |
| 5000 | 34–55 µs |

**And then the guard**, because a table is static for a whole session in the ordinary case and paying 8.5 µs every frame
for a constant is the same fault as paying 990:

| case | cost |
|---|---|
| unchanged table (`table.is_changed()` false) | **0 ns** |
| empty map (early return, before the first play) | **0 ns** |
| the frame the table is written | 8.5 µs at 1000 cues |

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
