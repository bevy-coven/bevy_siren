# Iteration 90 — eval

## The cases

`src/semantic/filter.rs` plus one field on `Weights`, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **376** and **55** lib (45 + 10 new) |
| 19 (per-frame) | **green** | 0.329 / 0.630 µs/frame at 64/256 |
| #27 crit 4 — 12 LU refused, 2 LU admitted | **green** | with the difference reported |
| #27 crit 1 — a pair either side of each threshold | **green** | 5.9 / 6.0 / 6.01 / 12.0 against a 6.0 tolerance |
| #27 crit 2 — an unknown feature never excludes | **green** | short clip on either side and on both |
| #27 crit 3 — every threshold overridable | **green** | 1.0 refuses 4 LU, 24.0 admits it |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 376 + 55 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **20 GB** free; no disk interruption.

## Four defects, three of them mine to begin with

1. **A test landed in an inherent `impl`.** `weights.rs` had **no test module at all**, so appending `#[test] fn` to the
   file's last brace put it inside `impl Weights`. Clippy caught it as *"the `test` attribute may only be used on a free
   function"*. Fixed by creating a real `#[cfg(test)] mod tests`.
2. **The new field broke a literal construction.** `tests/semantic_weights.rs` builds a `Weights { .. }` exhaustively,
   so adding a field failed to compile — which is the right outcome: a struct a project writes in RON and a test writes
   in Rust should both notice a new threshold.
3. **A broken intra-doc link.** The module doc linked `[Report::excluded]`, a field that never existed — the report
   carries *counts*, not a reason list. `rustdoc -D warnings` caught it and the sentence now matches the type.
4. **A `WeightProblem` that would have gone unchecked.** Without adding the field's validation, a negative
   `noise_floor_tolerance_db` would have made every candidate look noisier than the anchor — a sign error the
   `problems()` report exists to name.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–89.
