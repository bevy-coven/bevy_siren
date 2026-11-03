# eval.md — iteration 71

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 40.5 s after a clean |
| `cargo test` | green — **181** + 6 |
| `cargo test --all-features` | green — **339** |

**Nothing red.** Working tree clean at `c842b6b`.

## Machine state

| | |
|---|---|
| start | **2.1 GB free**, `target` 16 GB |
| after `cargo clean` | **17 GB free** (15.6 GiB removed) |
| end of iteration | **1.2 GB free** |

The clean was needed before the baseline could run at all. The end-of-iteration figure means the next release build will
need one too.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — blocked on the `CueProblem` enum decision |

## After

`bevy_siren`: **194 default / 352 all-features**, clippy 0, fmt, rustdoc — green.

## What was not built, and why

Three of the five remaining items are **decisions this loop must not make**:

| Item | Why it is not decidable here |
|---|---|
| where the weights live | a definitions-file schema change → `needs-decision/weights-live-where.md` |
| `V-01`'s labelled set | needs a person and a licence-cleared corpus |
| `#40`'s full ensemble | needs `#39`'s encoder, blocked on `S-05`'s licence audit |

What **is** decidable is the structure the ensemble scores into — a breakdown that keeps every contribution and can
explain itself — and that is what shipped.
