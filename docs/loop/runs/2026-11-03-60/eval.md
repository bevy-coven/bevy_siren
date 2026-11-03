# eval.md — iteration 60

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 162 + 6 |
| `cargo test --features serde` | green — 170 + 6 |
| `cargo test --features mixer` | green — 172 + 6 |
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

## Six probes, all correct

`SirenLog`'s chronology through out-of-order eviction; `semitones_to_ratio`'s clamp vs the mixer's `set_speed`; pitch
composition with a `Play` override; `load_table`'s ask-then-not-missing semantics; a muted bus still playing; and two
music layers settling on a one-voice bus. Each probe was deleted — a test asserting untouched behaviour is noise.

**One of them failed first and the code was right.** My music-budget probe asserted `blocked("combat")` when the
**stolen** layer `calm` is the one refused and recorded. The log made it obvious:
`played calm` → `stopped calm: stolen by combat (bus)` → `played combat` → `refused calm: no voice free on bus Music`.
That is the documented virtual-voice path working.

## The item

**A play draws for its detune even when its `pitch` range is exact**, so adding a fixed-pitch cue to a scene moves every
later pick for every other cue. Measured and pinned by
`tests/play.rs::an_exact_pitch_range_still_draws_from_the_stream`.

**Not fixed**, and the reason is in the decision file: skipping the draw **re-rolls every project's soundscape** in the
build that ships it, with no authoring change to explain it. That is a release note, not a patch.

## After

Green: **163 / 171 / 173 + 6**, clippy, fmt, rustdoc, examples. Nothing audible moved: `bus-Music` still
`duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still −18.6 LUFS at −6.1 dBFS. See `review.md`.
