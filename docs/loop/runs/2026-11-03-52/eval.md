# eval.md — iteration 52

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 136 + 6 |
| `cargo test --features serde` | green — 144 + 6 |
| `cargo test --features mixer` | green — 147 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **3.8 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.** Disk was at **100 %** on arrival; `/tmp/probe-target` (1.2 GB) was this loop's own scratch from
iteration 51 and was removed, leaving 4.9 GB. `target/` (38 GB) is not mine and was not touched.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## Why this item and not the two above it

Backlog #2 (`AvoidRepeat`'s unachievable window) and #3 (case 31) both need a **seventh `CueProblem`**, gated by the open
`bad-gain-problem.md`. Probing found a fault with the same character that needs **no enum change**:
`SirenSettings::problems()` returns `Vec<String>`, so a mix-table sentence is additive.

**Measured before the fix**, one duck each: `by_db: 0.0` with hold 0.25 and fade 0.6 → `problems: []`. `by_db: NaN` →
also `[]`, but **RON refuses `nan`** (`Expected float`), so it is not authorable and needs no sentence. `by_db: +6.0` and
a real duck with no envelope were already reported.

## After

Green: **137 / 144 / 147 + 6**, clippy, fmt, rustdoc, examples. Five new tests; two of them fail on the old tree (`[]`)
and the "must not happen" pair stays green — `the_default_ducks_report_nothing` (Gregory's −8 dB duck is silent) and
`a_zero_depth_duck_still_fires_and_still_moves_nothing` (the rule is inert, not absent).

Panel: `SIREN_MIX=zero` renders the sentence as the whole `Problems` block — `evidence/panel/zero-after.png`.
