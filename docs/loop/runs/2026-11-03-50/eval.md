# eval.md — iteration 50

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 128 + 6 |
| `cargo test --features serde` | green — 136 + 6 |
| `cargo test --features mixer` | green — 139 + 6 |
| `cargo test --doc --all-features` | green — 6 |
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
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a parented sound whose owner is despawned | green | iteration 43 |
| 33 | a layer the budget refused says `waiting` | green | iteration 44 |

## The item, and why it was not an art decision

Iteration 49's backlog called the shipped table's clipping *"an art decision"*. Reading
`docs/loop/needs-decision/above-unity-gain.md` settled it: its recommendation **C — do nothing** rests on
*"`SirenSettings::master_db` is the tool for it"*. So the crate already said what to do, and had never tested the
sentence, said it in the guide, or applied it to its own two demonstration files.

Measured first: `worst_peak_db` is `0`, `−6`, `−20` dBFS at masters of `0`, `−6`, `−20`. The claim holds.

## After

All green: **129 / 137 / 140 + 6**. `mix.wav` from `+0.2 dBFS / −4.3 LUFS` to **`−6.1 dBFS / −18.6 LUFS`**. See
`review.md`.
