# eval.md — iteration 49

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 127 + 5 |
| `cargo test --features serde` | green — 134 + 5 |
| `cargo test --features mixer` | green — 138 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **4.9 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused says `waiting` | green | iteration 44 |

## Where the item came from

Four iterations of `problems()` sentences, so this one went back to **sound output**: `examples/render`, its spectrograms
and its loudness numbers — a gate the skill names and that had not been read since iteration 42.

Two things were checked first and were **correct**:

- `bus-Music: recovered by 1.97 s (hold 0.25 + fade 0.60)` looked like 0.12 s too long. It is right: `update_mix` eases a
  bus toward the duck's already-shaped envelope with `MIX_TAU_S`, and `cues.yaml` case 6's own `green_when` says
  *"both read 0 dB at hold + fade + **50 ms**"* — the 50 ms is that constant.
- `ebur128` returned nothing for `voices-64` and `voices-256`; both are **all-rail**, and `ebur128` has no gate to
  integrate at 0 dBFS. Not a measurement bug.

Then the harness's own line, `(the budget's job, not the gain's)`, which is **false** — and that was the item.

## After

All green: **128 / 136 / 139 + 6 doctests**. See `review.md`.
