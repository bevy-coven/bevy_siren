# Iteration 120 — perf

**The machine**: Apple M4 Pro, macOS 26.6.2, a shared interactive session (load average 1.3 – 1.9 during the runs, 4 users), release profile. **What is measured**: `perfcheck` is the crate's per-frame work (`update_mix` and its neighbours), an **upper bound** — it costs every voice, including the ones `update_mix` skips because their gain did not move — and **not** the audio thread's. It does **not push log entries**, so the log's change is timed separately below.

## `cargo run --release --example perfcheck`, three runs (`evidence/after/perfcheck.txt` is run 1)

| voices | run 1 (µs/frame) | run 2 | run 3 | iteration 119 (three runs) | change |
|---|---|---|---|---|---|
| 8 | 0.061 | 0.062 | 0.062 | 0.057 – 0.062 | none |
| 64 | 0.322 | 0.328 | 0.357 | 0.304 – 0.359 | none |
| 256 | 1.152 | 0.837 | 0.951 | 1.053 – 1.225 | none (run-to-run spread on this shared machine is larger than the difference) |

No case moves by 10 % or more. Nothing `perfcheck` runs changed: `SirenLog::push` is called by the play, steal, stop and duck paths, which it does not exercise.

## `SirenLog::push` and `since`, timed directly (a scratch release program, deleted; three rounds of two million)

A **full ring**, so every push also evicts — the case a busy scene is always in:

| | round 1 | round 2 | round 3 |
|---|---|---|---|
| build an `Entry::Played` and push it | 37.0 ns | 33.7 ns | 33.4 ns |
| build it alone | 31.8 ns | 29.3 ns | 30.5 ns |
| **the push itself** | **5.2 ns** | **4.4 ns** | **2.9 ns** |
| `since(mark)` with nothing new (the quick start's `report`, most frames) | 10.5 ns | 10.4 ns | 10.4 ns |
| `since(mark)` with two new entries, summing their cue's length | 11.0 ns | 11.0 ns | 11.0 ns |

So the sequence number costs a store and an add in a path that builds four `String`s, and the cursor costs about **11 ns a frame**. **The before was not measured**: a second worktree of `0a0e359` would need a second multi-gigabyte target directory on a disk with 2 GB free. The change to `push` is structural — one `u64` stored per entry and one increment — and the figure above is the whole cost of the push including that.

## The test run

`quick_start::` (4 tests, two of them waiting on the real asset server for two 6 KB files): **0.06 – 0.08 s** for the module, in every run of this iteration.
