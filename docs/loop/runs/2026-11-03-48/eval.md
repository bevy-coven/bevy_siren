# eval.md — iteration 48

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 126 + 5 |
| `cargo test --features serde` | green — 133 + 5 |
| `cargo test --features mixer` | green — 136 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **4.9 GB free (99 %)** — the tightest of any iteration |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused says `waiting` | green | iteration 44 |

## The item

Backlog #5, **verified in iteration 47 and built here**: a bus cap above the table's is silently the table's. The test
that verified it last iteration asserted the **silence**; this iteration inverted it to assert the sentence, which is
the only change a report addition makes to its own proof.

## After

All green: **127 / 134 / 137 + 5**. See `review.md`.
