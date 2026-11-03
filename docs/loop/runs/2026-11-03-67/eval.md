# eval.md — iteration 67

## Baseline, before anything changed

Iteration 66 shipped as `33ef8f4` with its own records; the tree was clean apart from this iteration's work.

| Step | Result |
|---|---|
| `cargo test` | green — **174** + 6 |
| `cargo test --features serde` | green — **183** + 6 |
| `cargo test --features mixer` | green — **184** + 6 |
| `cargo test --all-features` | green — **293** |
| clippy / fmt / rustdoc / examples | green, 0 errors |

**Nothing red in the crate.** The red is in the *harness*, and it was found by looking at the evidence:

| | |
|---|---|
| `md5 cue-footstep-grass-0.wav` | `eefe987d5aef348c1d6f4abe482db585` |
| `md5 cue-dialogue-greeting-0.wav` | `eefe987d5aef348c1d6f4abe482db585` |
| `md5 cue-bang-0.wav` | `eefe987d5aef348c1d6f4abe482db585` |

**Three different cues, one file.** That is the item.

## Machine state

| | |
|---|---|
| start | **228 MB free**, `target` 23 GB |
| after `cargo clean` | **20 GB free** (20.4 GiB removed) |
| a build attempt before the clean | **`ld: write() failed, errno=28`** — the full-disk failure that reads as a code error |

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged in the crate |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — still blocked on the `CueProblem` enum decision |

## What is red, exactly, and where

`examples/render.rs`, line 99 before this iteration:

```rust
let samples = wave(440.0 * semitones_to_ratio(semitones), ms, 12.0);
```

Every cue and every variant synthesised **the same 440 Hz sine** with the same 12/s decay. And `write_gain` began
`let _ = speed;`, discarding the playback rate the module's own doc says it applies.

The consequence is not cosmetic: **every spectrogram this project has judged was one sine wave**, whatever cue name the
file carried. The visual gate measured one waveform 30 times and reported 30 cues.
