# eval.md — iteration 69

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 39.9 s after a clean |
| `cargo test` | green — **181** + 6 |
| `cargo test --all-features` | green — **309** |

**Nothing red.** Working tree clean at `593e6b8`.

## Machine state

| | |
|---|---|
| start | **4.0 GB free**, `target` 17 GB |
| after `cargo clean` | **20 GB free** (16.8 GiB removed) |
| when a release probe failed | **495 MB** — `rustc-LLVM ERROR: IO failure on output stream: No space left on device` |

A release build of the example target spends ~18 GB here. The probe was moved into the test suite instead, where it
costs nothing.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — blocked on the `CueProblem` enum decision |

## After

`bevy_siren`: **181 / 324 all-features**, clippy 0, fmt, rustdoc, examples — green.

## A mistake, recorded

Freeing disk I ran `git checkout -- .`, which **reverted the two tracked edits** this iteration had made
(`src/semantic/mod.rs`, `tests/all.rs`). The new files survived because they were untracked. Both edits were restored
from the same script and verified with `grep`. The rule is the repository's own: `git restore` over changes I did make is
still a destructive command, and the narrower `git checkout -- examples/evalprobe.rs` is what was wanted.
