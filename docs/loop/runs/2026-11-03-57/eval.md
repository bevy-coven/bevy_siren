# eval.md — iteration 57

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 154 + 6 |
| `cargo test --features serde` | green — 162 + 6 |
| `cargo test --features mixer` | green — 164 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **4.3 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

**`bevy_siren` 0.1.0 is published** (crates.io, 2026-11-02, 9 downloads), so the `CueProblem` break the two blocked items
wait on is a real break against a real consumer. The decision stays open.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## Nine hypotheses probed, one bug found

The loop's rule is that a pick is an observation, so this iteration probed and **passed** nine candidates before finding
one: cue-cap steal order (four ways, including archetype reuse after a free), a stopped voice's duck recovery, a bus level
changed mid-voice, and the `SirenSounds` merge. All are *accidentally* correct or already correct; the four cue-cap
probes were **deleted** rather than kept, because they assert behaviour no change touched and the file is not a museum
for this loop's questions.

**The bug that was real:** `arm_stinger_ducks` armed `clock.manual` and wrote `Ducked { by_db: … }` **unconditionally**,
so a `Stinger { duck_db: 0.0 }` logged a duck while `active_duck_db`'s `by_db >= 0.0` guard made it lower nothing.
Measured: **`[0.0]` in the log, `Music` unmoved**.

## After

Green: **155 / 163 / 165 + 6**, clippy, fmt, rustdoc, examples. `a_stinger_at_zero_db_logs_no_duck` fails on the old tree
with `a 0 dB stinger must not claim a duck: [0.0]` and passes on the new; the −9.0 non-regression still logs `[-9.0]` and
still dips. See `review.md`.
