# eval.md — iteration 63

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — **174** + 6 |
| `cargo test --features serde` | green — **183** + 6 |
| `cargo test --features mixer` | green — **184** + 6 |
| `cargo test --doc` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | **0 errors** |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing is red.** Iteration 62 left no uncommitted work; working tree clean at `2ada2d2`.

## Machine state

| | |
|---|---|
| local disk | **4.3 GB free (100 % reported)** — `target` is 22 GB |
| `bmb` | Apple M4, 48 GB RAM, venv at `/tmp/siren-spike` with torch 2.15.0 |
| `big` | RTX 3090, reachable |

Disk is the binding constraint. Anything this iteration writes must be small, and the model work goes to `bmb`.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — still blocked on the `CueProblem` enum decision |

Case 31 is the only red case and remains blocked. The semantic path does not touch it.

## What is open on the critical path

| Issue | State | What it needs |
|---|---|---|
| #10 | open | extend `tests/leaf.rs` — gates `ort` |
| #12 | open | the resampler decision (`needs-decision/semantic-dependencies.md`) |
| #17 | open | the model load — needs `ort` |
| #26 | open | text encode + `top_k` — needs the text tower exported |
