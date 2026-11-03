# eval.md — iteration 45

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 122 + 5 |
| `cargo test --features serde` | green — 129 + 5 |
| `cargo test --features mixer` | green — 132 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **6.9 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused is a virtual voice | green | iteration 44 |

Case 31 re-checked a third time: every route (a `CueProblem`, `Refusal` or `Entry` variant) breaks a public enum with
no `#[non_exhaustive]`, and the decision file's own recommendation is *"C now, A at the next minor"*. Nothing to build.

## The item found

The case table said green; the **report** had four sentences no test reached, and the panel drew only half of it. Both
are measured in `research.md`: the fixed-string search over `tests/`, and the `SIREN_MIX=broken` capture where the
block said `none` while every bus read −80 dB.

## After

All green: **123 / 130 / 133 + 5**. See `review.md` for the three verdicts.
