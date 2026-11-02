# eval.md — the harness itself, before any feature

This is not a feature iteration: no crate behaviour changed. It is the run that built the loop's eyes — `examples/render`,
`docs/loop/cues.yaml`, the run and decision directories — and proved the harness by reading its output.

## Baseline, as recorded (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 41 + 4 doctests green |
| `cargo test --features serde` | 48 + 4 green |
| `cargo test --features mixer` | 41 + 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green (now including `render`) |

Nothing was red, so the iteration's work is the harness and the case table, not a fix.

## The `cues.yaml` table, as the harness observes it

The harness drives the crate's own functions (`bevy_siren::model`) rather than the ECS path, so what it reports is the
arithmetic every cue rests on. Its output is `evidence/after/render.txt`.

| Case | Observed | Green? |
|---|---|---|
| 5 — a cue's gain and bus | `cue-ui-cancel-0: -6.00 dB`; the default table has the four buses | green |
| 6 — a duck holds then recovers | `bus-Music: duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s (fired at 1.00 s, hold 0.25 + fade 0.60)` | green: depth reached, hold respected, recovered inside hold + fade + the 50 ms tail |
| 10–13 — each refusal | five `refusal-*.txt`, each the crate's own sentence: `no cue of that name`, `no handle for sound \`step_09\``, `bus \`Reverb\` is not in the mix table`, `no SpatialListener to place a sound at [3, 0, 4]`, `no voice free on bus \`Dialogue\`` | green: each names what is missing |
| 14 — distance decides the gain | `spatial: 1.000 at 4 m, 0.500 at 8 m, 0.250 at 16 m` | green: the inverse law at twice and four times the near radius |
| 15 — a bar at tempo | `music: a bar at 160 bpm from 0.3 s is 1.50 s` | green: 4 beats at 160 bpm |
| 19 — many voices | `voices-8: peak 3.419, rms 0.397, 307 samples would clip (the budget's job, not the gain's)` | open: the sum of 8 voices at their authored gains exceeds full scale, which is a voice-budget question and is the first item of the next iteration |

Cases 1–4, 7–9, 16–18 are the play system's decisions and are already proven by `tests/play.rs`, `tests/mix.rs` and
`tests/music.rs`; the harness does not re-run them, and `cues.yaml` says which observation proves each.

## Loudness, measured

| File | Integrated | True peak | Note |
|---|---|---|---|
| `cue-ui-cancel-0.wav` (0.32 s) | −24.4 LUFS | −12.1 dBFS | authored at −6 dB |
| `cue-footstep-grass-0.wav` (0.32 s) | −18.4 LUFS | −6.1 dBFS | authored at 0 dB |
| `mix.wav` | −4.3 LUFS | 0.2 dBFS | ten cues summed; above 0 dBFS peak, which is the budget's job |
| `bus-Music.wav` | −33.1 LUFS | 0.4 dBFS | a ducked bus held at one level; peak above 0 because it is a gain curve, not audio |
| `voices-8.wav` | −12.2 LUFS | 0.1 dBFS | eight voices at their authored gains |

**`ffmpeg`'s `ebur128` reports −70 LUFS for a clip shorter than one gating block (0.4 s), however loud it is.** A
0.32 s cue at −12 dB peak reads as silence; looped to clear the block it reads −24.4 LUFS. So a short cue's loudness is
measured looped (`aloop=loop=3:size=<samples>`) and its amplitude by `astats` (peak and RMS), and the command says so.

## Two defects the harness found in itself, and what they taught the command

1. **A bus envelope written one sample per frame is a 4 ms file.** `bus-Music.wav` was 0.004 s; a gain is a step
   function between frames, so each frame's gain must be repeated for the samples that frame lasts. The command now
   says so where the harness is described, because the next person to write a bus plot will make the same mistake.
2. **The duck was fed the wall clock, so it never recovered.** `active_duck_db` takes the time *since the duck started*
   — what `DuckClock` stores — and the first version passed the score's elapsed time, so the duck sat at its depth
   forever. The command names that trap under step 6.

A third defect was mine in the judging, not in the code: `showwavespic` on a gain curve draws audio and turns a clean
envelope into an unreadable blob. The command requires a line plot for a bus's gain and reserves `showwavespic` for
audio.

## Verdict

The harness is the deliverable; it is judged in `review.md`.
