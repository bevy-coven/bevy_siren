# eval.md — iteration 59

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 158 + 6 |
| `cargo test --features serde` | green — 166 + 6 |
| `cargo test --features mixer` | green — 168 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **3.3 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## What was probed, and what it was

- **A wrong-bus stinger is invisible to a program** — confirmed (`refused` is empty), but it needs a new `Refusal`
  variant: a public-enum break on a **published** crate. Already `needs-decision/stinger-refusal.md`. Not shipped.
- **`load_table` semantics** — a key it has asked for is not "missing", and a second call asks nothing. Correct;
  probe deleted.
- **A muted bus** — still plays, `Played` carries the cue's own gain, the bus reads silent. Correct; probe deleted.
- **A refused play advances the variant window and spends a draw** — measured, and it is **protective**: a cue with one
  missing file still sounds **4 of 8** requests. Not a bug. Recorded as
  `needs-decision/pick-before-validate.md` with the measurement, so a later iteration does not "fix" it into a livelock.

## The item

`worst_peak_db` returns `SILENT_DB` for a bus with **no voices**, which is the same number it returns for a bus **mixed
to the floor** — while its own doc says *"nothing playing is not `SILENT_DB` — it is no peak at all"*. Measured:
`assert_ne!(empty, silent_with_voices)` fails at `−80.0` for both.

**Ranked honestly as rank 3, not rank 1**: every caller in this repository filters `voices_on > 0` first, so it is a
latent trap rather than an active wrong output — but the workaround is in the crate's **own example**, which is how a
consumer would meet it.

## After

Green: **161 / 169 / 171 + 6**, clippy, fmt, rustdoc, examples. `SirenMix::peak_db` is **additive**; `worst_peak_db`
keeps its sentinel because changing its signature would break consumers of 0.1.0. The example now uses `peak_db` and
its `n > 0` filter is gone. See `review.md`.
