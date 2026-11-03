# Iteration 94 — eval

## The cases

`src/semantic/filter.rs` and `weights.rs`, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **380** and **87** lib (80 + 7 new) |
| 19 (per-frame) | **green** | 0.327 / 1.132 µs/frame at 64/256 |
| #27 crit 1 — a pair either side of each threshold | **green** | tempo: 3.9 / 4.0 admitted, 4.1 / 38 refused; key: same/fifth vs parallel vs clash |
| #27 crit 2 — an unknown never excludes | **green** | tempo absent, `bpm: None`, and either side; key absent **and** judged-atonal |
| #27 crit 3 — every threshold overridable | **green** | `tempo_tolerance_bpm` 1.0 / 100.0; `clash_penalty: 1.0` turns the rule off |
| #27 crit 4 — 12 LU refused, 2 LU admitted | **green since iteration 90** | unchanged |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 380 + 87 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **19 GB** free; no disk interruption.

## Three defects

1. **My `Features` literal edits missed twice.** The struct gained two fields and the `impl` methods did not land the
   first time — `pub fn` versus `fn` in the anchor string — so the build reported the struct change as complete while the
   accessors were absent. Caught by the compiler on the next build, not by me.
2. **`Relation` was not imported**, though `Exclusion::Key` names it. One import.
3. **`clippy::collapsible_if`** on the nested `if let` the key rule first used. Collapsed with let-chains.

## A design point that needed deciding

`Weights::compatibility` is a **nested serde struct**, the first on this type. It is right because `#27` requires the
compatibility table be a named type rather than a matrix in a system, and `#25` had already built exactly that type — so
the field is a reuse rather than a second definition. A project that wants clashing keys sets `clash_penalty: 1.0` and
the rule stops penalising, which is the overridability criterion satisfied by data rather than by a flag.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–93.
