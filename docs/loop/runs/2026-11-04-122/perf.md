# Iteration 122 — perf

**The machine**: Apple M4 Pro, macOS 26.6.2, a shared interactive session (load average 2.9 – 6.8 during the runs, 4 users), release profile. **What is measured**: `perfcheck` is the crate's per-frame work (`update_mix` and its neighbours), an **upper bound** — it costs every voice, including the ones `update_mix` skips because their gain did not move — and **not** the audio thread's.

**What this change can cost**: it **removes** a draw from two places (the pitch of a range with one detune in it; a pick with one candidate left) and adds a branch to each. `perfcheck` does not play, so it does not draw; the figures below are the check that nothing moved.

## `cargo run --release --example perfcheck`, three runs (`evidence/after/perfcheck.txt`)

| voices | this iteration (µs/frame) | iteration 121, eight runs | iteration 120, three runs | change |
|---|---|---|---|---|
| 8 | 0.047 – 0.057 | 0.041 – 0.067 (median 0.060) | 0.061 – 0.062 | none |
| 64 | 0.283 – 0.325 | 0.246 – 0.374 (median 0.333) | 0.322 – 0.357 | none |
| 256 | 0.846 – 1.115 | 0.769 – 1.248 (median 1.170) | 0.837 – 1.152 | none |

Every case is inside the band iteration 121's own eight runs described on the same shared machine; no case moves by 10 % that the run-to-run noise does not also move by.

## The play path, counted rather than timed

`Rng::next_f32` costs **1.5 ns** (iteration 121, unchanged code). A play that used to take two draws takes one, or none:

| play | draws before | after |
|---|---|---|
| one sound, `pitch: exact` (a burst, a UI click) | 2 | **0** |
| two sounds, `avoid: 1` (a footstep), first play | 2 | 1 |
| two sounds, `avoid: 1`, every later play | 2 | **1** |
| three sounds, `avoid: 1` | 2 | 2 (two candidates: still a decision) |

So the change takes **up to 3 ns a play** off the path, and it is a branch on a `f32` comparison and a `usize` compare.

## The baseline's disk cost, recorded

| | GB |
|---|---|
| this crate's debug + release artifacts (cleaned at the start of every matrix) | ~5.2 + 0.002 |
| `target/debug/deps` (dependencies: Bevy, ORT, wandbox for `semantic`) | **9.8** |
| `target/release/deps` | 2.8 |
| stale example binaries of scratch examples from iterations 110 – 113 (`diag`, `probe`, `lcheck`, `zzdump`, `zz_demo_library`) | 0.9 |
| `~/.cache/huggingface` (**the owner's**) | **50** |

The volume is at 100 % with **2 – 3 GB free**. `cargo test --features semantic` failed at **2.1 GB** with `No space left
on device` while `cargo test --all-features --tests -j 1` passed at 2.8 GB: the matrix's `guard` cleans this crate below
5 GB, and the dependency artifacts are what fills the volume. **Not deleted**: the 9.8 GB of deps is the owner's, the
50 GB cache is the owner's, and the 0.9 GB of stale scratch binaries **predate this session** — they are named in the
log's top 5 with their paths (`target/{debug,release}/examples/{diag,probe,lcheck,zzdump,zz_demo_library}*`).
