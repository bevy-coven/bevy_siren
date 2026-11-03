# research.md — iteration 67: the evidence harness was measuring one waveform

## The item

`examples/render` is how this project judges a change without an ear. Two of its claims were false, and both were found
by looking at its output rather than at its code.

## S1 — the measurement that found it

    md5 cue-footstep-grass-0.wav    eefe987d5aef348c1d6f4abe482db585
    md5 cue-dialogue-greeting-0.wav eefe987d5aef348c1d6f4abe482db585
    md5 cue-bang-0.wav              eefe987d5aef348c1d6f4abe482db585

**Three different cues, one file.** The cause, at `examples/render.rs:99`:

```rust
let samples = wave(440.0 * semitones_to_ratio(semitones), ms, 12.0);
```

Every cue and every variant synthesised the same 440 Hz sine with the same 12/s decay. Two cues at the same gain were
therefore the same bytes, and a spectrogram of any cue was a spectrogram of that one tone.

## S2 — the second claim, also false

`write_gain` began `let _ = speed;` — the playback rate was **discarded**, while the module's own doc says the file is
written "at the gain and speed a play gives it". The report line `cue-footstep-grass-0: 0.00 dB, speed 0.9439` printed a
speed that never reached the samples.

A pitch in this crate is a *playback rate*: a source at 0.9439× is 1/0.9439 longer and every frequency is lower by that
factor. Writing it at 1.0× is neither.

## S3 — the technique: a timbre from the name

Four numbers per sound, derived by FNV-1a over the sound's own name — the same hash `src/library.rs` uses, so a name's
timbre is stable across runs:

| | |
|---|---|
| fundamental | **90 – 620 Hz** (`90.0 + unit(0) * 530.0`) |
| second partial ratio | 2.0 – 4.0 for an effect; **1.5** for a loop |
| third partial ratio | 3.0 – 7.0 for an effect; **2.0** for a loop |
| decay | **6 – 24 /s** for an effect; **0.6 – 1.6 /s** for a loop |

The numbers are this crate's own choice — **design default** — and the argument is what they *separate*, not what they
are: a loop must be a sustained body and an effect a transient, which is the difference the spectrograms did not show
when every cue was one sine.

`at_speed` is linear interpolation over the source at `speed` steps per output sample, which is what a rodio `speed()`
does: at `speed` < 1 the buffer is stretched and every frequency drops by the same factor.

## S4 — what changed in the evidence

| | before | after |
|---|---|---|
| distinct cue files | **3 cues shared one hash** | **14 files, 14 distinct hashes** |
| `voices-8` peak | 3.419 (307 samples past full scale) | **2.959 (162 past)** |
| `mix.wav` integrated | −18.6 LUFS | **−21.5 LUFS** |
| `mix.wav` true peak | −6.1 dBFS | **−7.5 dBFS** |

The mix moved because the cues it is built from are no longer all 0.5-amplitude sines; the new wave peaks at 0.32 ×
(1 + 0.45 + 0.22) and decays, so the sum sits lower. **−21.5 LUFS is further under the −14 LUFS a game mix targets**,
which is the safe direction and is recorded rather than tuned.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| rendered rate | 22 050 Hz mono 16-bit | the harness's own doc |
| effect length | 320 ms → 7056 samples | `RATE * ms / 1000`, asserted |
| loop length | 1500 ms → 33 075 samples | same |
| a detuned cue | speed 0.9439 → **1.0594× longer** | `1 / 0.9439`, asserted |

## What good sounds like

For the harness, "good" is **distinguishable**: a spectrogram of a footstep must not be a spectrogram of a bang, a
loop's envelope must still be sounding at its last tenth while an effect's has decayed, and a cue detuned by a
semitone must be measurably longer and lower. All three are asserted in `tests/render_shapes.rs` in 0.01 s, against the
same functions the harness calls — a test of the arithmetic rather than of a nine-minute release build.
