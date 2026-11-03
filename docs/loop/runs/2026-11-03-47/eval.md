# eval.md — iteration 47

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 124 + 5 |
| `cargo test --features serde` | green — 131 + 5 |
| `cargo test --features mixer` | green — 134 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **5.9 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused says `waiting` | green | iteration 44 |

## Six probes, one finding

The case table said green, so the loop went looking. Six areas were read or measured and **five were already correct**:

| probe | result |
|---|---|
| `Cue::new` vs the serde defaults, field by field | agree; a RON table with a missing field is the Rust default |
| `backend_spatial_gain` vs rodio's own `spatial.rs` | **identical** — same formula, same clamps, confirmed from the crate source |
| `stop_requests` against a music layer's voice | handled by `reconcile`/`is_game_stop` (iteration 36) |
| a looping cue at `max_instances: 1`, asked 60 times | **one voice** — new guard kept, no bug |
| a self-duck's depth | **−6.0 dB on its own voice** — the finding |
| a duck with no hold or fade | iteration 46's, now reported |

## After

All green: **126 / 133 / 136 + 5**. See `review.md`.
