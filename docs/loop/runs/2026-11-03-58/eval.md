# eval.md — iteration 58

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 156 + 6 |
| `cargo test --features serde` | green — 164 + 6 |
| `cargo test --features mixer` | green — 166 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **4.3 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## Three hypotheses probed and found correct

- a stinger's manual duck recovers after its voice is gone (the envelope is the clock's, not the voice's);
- a deeper stinger recovering hands back to a shallower dialogue duck still running (deepest-wins, re-evaluated);
- a negative or `NaN` `duck_s` still recovers (`DuckClock::manual` clamps).

All three passed and were **deleted** rather than kept: they assert behaviour no change touched.

## The item

`move_layers` used `if let Some(layer) = … find(..)`, so a target naming a layer the score does not have was **silently
dropped**. The one message in the crate with no failure path. Measured: the log held only the layers' own `played` lines
and nothing about the typo.

## After

Green: **157 / 165 / 167 + 6**, clippy, fmt, rustdoc, examples. `a_layer_change_naming_a_missing_layer_says_so` fails on
the old tree and passes on the new; `a_typo_says_which_layer_and_a_real_one_still_moves` proves one change can move a real
layer and name a typo in the same frame. The panel capture `docs/media/mixer-layer-typo.png` shows the sentence beside
`combat -3.0`. See `review.md`.
