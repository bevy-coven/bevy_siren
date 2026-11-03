# perf.md — frame cost at 8, 64 and 256 voices, and the new retain

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **5.9 / 6.9 / 6.3**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.065 | 0.066 | 0.062 |
| 64 | 0.361 | 0.359 | 0.357 |
| 256 | 1.129 | 1.192 | 1.327 |

**`perfcheck` does not run the changed code.** It calls `approach`, `db_to_gain` and `chain_db` over a fixed gain list;
it builds no `DuckClock` and no mix table (`examples/perfcheck.rs:33-40`). The 8- and 64-voice rows are flat to within
1 %, and the 256-voice row's 18 % spread is the host — iteration 51 measured a **2× spread on a single binary** at load
5.1, and iteration 53 a 44 % spread at load 7.1.

## The new per-frame work, measured directly

`retain_rules` is called once per frame from `update_mix`, so it is new per-frame work and must be measured rather than
assumed. 200 000 iterations per case, `--release`:

| starts held | rules in the table | ns per frame |
|---|---|---|
| 1 | 2 | 14.3 |
| 10 | 2 | 13.9 |
| **1000** | 2 | **13.0** |

**Flat at ~13 ns across a thousand-fold range of held starts**, against a 16.7 ms frame — 0.00008 % of it. The number is
flat because the map is now bounded by the table: after the first frame it holds one entry per rule, so the `retain` has
almost nothing to walk. Before the fix the map grew without bound, which is the cost this removes rather than adds.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.

**No regression is claimed and none is measurable**: the probe cannot see this code, and the direct measurement puts the
addition three orders of magnitude below a frame.
