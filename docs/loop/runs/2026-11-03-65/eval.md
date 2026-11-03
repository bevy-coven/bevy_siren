# eval.md — iteration 65

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 40.8 s after `cargo clean` |
| `cargo test` | green — **174** + 6 |
| `cargo test --features serde` | green — **183** + 6 |
| `cargo test --features mixer` | green — **184** + 6 |

**Nothing red.** Working tree clean at `4b7a928`.

## Machine state at the start

**1.6 GB free**, `target` 23 GB. `cargo clean` (the standing approval for this repo's own build cache) freed 21.9 GiB
in 2.5 s.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged — `render.txt` reports the same variants and gains |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — still blocked on the `CueProblem` enum decision |

## After

`bevy_siren`: **174 / 183 / 184 / 282 all-features**, clippy 0, fmt, rustdoc, examples — green.
Loudness: **−18.6 LUFS**, true peak **−6.1 dBFS**.
