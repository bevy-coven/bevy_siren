# perf.md — the mix's per-frame cost

Measured 2026-11-02 with `cargo run --release --example perfcheck` on this Mac (Apple M4 Pro, 14 cores), under the
repository's toolchain 1.99.0. The crate's `src/` did not change this run, so this is a **baseline**, not a
before/after: the next iteration compares against it.

| Voices | µs per frame | µs per voice |
|---|---|---|
| 8 | 0.058 | 0.0072 |
| 64 | 0.332 | 0.0052 |
| 256 | 1.253 | 0.0049 |

At 60 fps a frame is 16 667 µs, so 256 voices cost **0.0075 % of a frame**. The per-voice cost falls as the count
rises because the four bus approaches are a fixed cost spread over more voices.

**This is an upper bound, and deliberately.** It costs `chain_db` and `db_to_gain` for *every* live voice, while
`update_mix` writes a voice only when its gain moved more than 0.01 dB and skips the rest; the real system is cheaper.
The number that matters for a release is therefore "well under a microsecond per voice", and a change that doubles it
still cannot be seen.

**What is not measured here** is the audio thread: `src/mixer/`'s `BusGain` and `VoiceGain` sources do one atomic load
and one multiply per sample, and a frame cannot measure them because they run on `rodio`'s thread. That belongs to an
item that touches the mixer, with `voices-256.wav` from `examples/render` as its clipping check.

## Case 19 of `cues.yaml`

The harness's `voices-8.wav` peaks at 3.419 — eight voices summed past full scale. That is the harness summing what a
bus *would* sum with no budget; the crate bounds it with `max_voices`. So case 19 is **the budget's case, measured
here and in `tests/mix.rs`**, not a gain defect in the mix, and `cues.yaml` says so. The next iteration either teaches
the harness to model the budget or moves case 19 wholly into this file.
