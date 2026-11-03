# eval.md — iteration 64

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 43.5 s after `cargo clean` |
| `cargo test` | green — **174** + 6 |
| `cargo test --features serde` | green — **183** + 6 |
| `cargo test --features mixer` | green — **184** + 6 |
| `cargo test --doc` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | **0 errors** |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `cargo test --all-features` | green — **265** |

**Nothing is red.** Iteration 63 left no uncommitted work; working tree clean at `ceef36b`.

## Machine state

| | |
|---|---|
| local disk | started at **642 MB free (100 %)**, `target` 24 GB |
| after `cargo clean` (approved) | **24 GB free**; 23.3 GiB of artifacts removed |
| `target` after the rebuild | ~2 GB |
| `bmb` | not used this iteration |

The disk was the first obstacle: 642 MB cannot build this crate. The clean was the crate's own build cache, no source
and no commits, and the rebuild took 43.5 s.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — still blocked on the `CueProblem` enum decision |

Case 31 is the only red case and remains blocked. It is not on the semantic path.

## The critical path

Eight of nine issues are closed (#2, #3, #7, #9, #10, #11, #15, #17, #26). **#12 is the last one open**, and it is
the one whose second half waits on a dependency decision.
