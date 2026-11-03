# eval.md — iteration 46

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 123 + 5 |
| `cargo test --features serde` | green — 130 + 5 |
| `cargo test --features mixer` | green — 133 + 5 |
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
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` (a public enum; the crate is 0.1.0) |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused says `waiting` | green | iteration 44 |

## The item found

Iteration 45's own backlog listed a **self-duck** as an open question. Investigating it produced a different and better
finding: the test written to pin the self-duck's behaviour returned **`0.0`** where I expected `-6.0`, and the reason
was not the self-duck at all — `Duck::new` leaves **hold and fade at zero**, so *any* rule built that way is inert.

Measured, not assumed: `a_duck_with_no_hold_and_no_fade_ducks_for_no_time` asserts `0.0` for the bare rule and `-8.0`
for the same rule with a hold. That the crate's own `default_ducks()` chains both is the evidence the hazard is real
and known to nobody.

## After

All green: **124 / 131 / 134 + 5**. See `review.md`.
