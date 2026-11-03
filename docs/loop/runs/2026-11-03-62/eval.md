# eval.md — iteration 62, the semantic critical path

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 163 + 6 |
| `cargo test --features serde` | green — 172 + 6 |
| `cargo test --features mixer` | green — 173 + 6 |
| `cargo test --doc` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing is red in `bevy_siren`.** Iteration 61 left no uncommitted work: working tree clean at `a86c404`.

## Machine state

| | |
|---|---|
| local disk | **11 GB free (98 %)** before; `bevy_siren/target` is 19 GB |
| `bmb` (MacBook Pro M4) | 48 GB RAM, torch 2.15.0 in a fresh venv, **8.7 GB free** |
| `big` (RTX 3090) | reachable, Linux x86_64 |

Both hosts are at 98 % disk. Nothing may download more than a few hundred MB without freeing space first.

## The board

48 open issues, #2–#49, filed this session. The critical path is
`#2 → #3 → #7 → #9 → #11 → #12 → #15 → #17 → #26`.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged by iteration 61, which touched no crate code |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — blocked on the `CueProblem` enum decision |

Case 31 remains the only red case and remains blocked on `docs/loop/needs-decision/`. The semantic work does not touch
it.

## What this iteration measures

The semantic feature is not in the crate yet, so `cues.yaml` has nothing to say about it. The evidence for this
iteration is the spike's own measurement, recorded in `research.md` and shipped as the verdict on #2.
