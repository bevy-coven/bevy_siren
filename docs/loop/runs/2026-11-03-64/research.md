# research.md — iteration 64: decoding, and the resampler's last word

## S1 — the decoder is already in the graph

`cargo tree --edges normal -p bevy_siren -i rodio`:

    rodio v0.22.2
    └── bevy_audio v0.20.1
        └── bevy_internal v0.20.1
            └── bevy v0.20.1
                └── bevy_siren v0.1.0

**`rodio` 0.22.2 is resolved in the default graph already**, as a dependency of `bevy_audio`. The `semantic` feature's
`dep:rodio` entry therefore **adds no crate**: it turns on this crate's own `wav` and `vorbis` features and nothing
else. Measured: `cargo tree --edges normal` is **413 lines** with no features, before and after this iteration.

This is the argument the dependency list needs, and it is stronger than "we already trust rodio": the decoder costs
nothing to the graph at all.

## S2 — `rodio`'s decoder reports its own rate as a `NonZero`

`Source::sample_rate()` returns `NonZero<u32>` and `Source::channels()` returns `NonZero<u16>` in 0.22.2. That is a
stronger promise than a runtime check: **a stream cannot declare 0 Hz or 0 channels**, so the two `BufferProblem`
variants for those are unreachable through the decode path. They stay, because `buffer::model_buffer` is public and a
caller can hand it anything.

Reading the rate from the **stream** rather than from `library`'s header parse makes the two independent confirmations
of the same file rather than one trusting the other.

## S3 — what decoding proves, and what it does not

`tests/semantic_decode.rs` writes real WAV files and decodes them:

| Asserted | How |
|---|---|
| the rate and channels are the file's | a 48 kHz mono WAV reports 48000 / 1 |
| the duration is the file's | 4800 frames at 48 kHz is **0.1 s** |
| the audio is the **signal**, not noise | a 440 Hz tone for 0.5 s has exactly **220** rising zero crossings |
| 48 kHz reaches a model buffer | `decode_for_model` returns 9600 mono samples |
| 44.1 kHz **decodes** and is then refused | `ResamplingRequired { from: 44100, to: 48000 }`, both rates in the message |
| a missing file names the path | `/nonexistent/nope.wav` is in the message |
| a non-audio file is refused, not a panic | `Undecodable`, naming the file |

The fourth row is the one that matters: **decoding succeeds for the commonest rate in a game's library**, and the
refusal that follows is about *conversion*, not about reading. A developer learns their file is fine and the crate
cannot yet convert it — which is a different message from "this file is broken".

## S4 — the resampler decision, restated with its numbers

`needs-decision/resampler-dependency.md` now carries the four options and the measurements that rule three out:

| Option | Verdict |
|---|---|
| A — add `rubato` | **recommended** — 1 kHz keeps 0.9763 of its energy, 15 kHz intact, builds in 2.12 s |
| B — require 48 kHz libraries | breaks the commonest workflow; 44.1 kHz is what asset stores ship |
| C — convert in the tool server | contradicts ADR 0006: the two front ends would disagree about what indexes |
| D — the Python sidecar | reverses ADR 0002 on measurement, not preference |

The implementation note that would bite: `FftFixedInOut` drops the **final partial chunk** unless it is flushed —
measured as a 480-sample gap (23520 out of 24000). A test must pin the flushed length.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `rodio` already in the default graph | **yes**, via `bevy_audio` | measured, S1 |
| default graph size, unchanged | **413 lines** | measured, S1 |
| 1 kHz energy through `rubato` | **0.9763** | measured, iteration 63 |
| 15 kHz energy through `rubato` | 0.488094 of 0.500000 | measured, iteration 63 |
| `rubato` build | **2.12 s** | measured, iteration 63 |
| unflushed tail | **480** samples per 0.5 s | measured, iteration 63 |

## What good sounds like

For this iteration, "good" is two things: **decoding reproduces the signal** — proven by a zero-crossing count, which
noise would fail — and **the refusal for a rate the crate cannot convert names both rates and the file**, so a
developer with a 44.1 kHz library learns what is wrong in one line rather than by trying things.
