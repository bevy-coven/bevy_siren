# research.md — what the judging rests on

No feature was built this run; the research question was *how does one judge game audio it cannot hear?* The answers
are the numbers the harness prints and the plots the command demands.

## Sources

1. **Jason Gregory, *Game Engine Architecture*, 3rd ed.** §14.5.8 (categories and buses, voice limits, stealing by
   priority then age) and §14.6.3 (ducking between groups, mix snapshots). Read through home-still; the crate's design
   rules quote it in `docs/research/siren.md`. The numbers this run judges against: a dialogue duck of **8 dB on music
   and 4 dB on effects**, hold **0.25 s**, fade **0.6 / 0.4 s**.
2. **ITU-R BS.1770-4**, *Algorithms to measure audio programme loudness and true-peak audio level* — the loudness
   measure `ffmpeg`'s `ebur128` implements: K-weighted, gated, integrated LUFS plus true peak. EBU R128's practice
   value is **−23 LUFS** for broadcast; game mixes are usually mastered louder and are not broadcast, so this
   command judges a cue's loudness against the item's own target and never invents a master target.
3. **Bevy 0.20's vendored source** (`~/.cargo/registry/src/index.crates.io-*/bevy_audio-0.20.1/`) for what a play
   actually does with a gain, a speed and a sink — the facts `tests/play.rs` asserts.
4. **`rodio` 0.22.2's `source::Spatial`** for the panner's own distance law, which the crate divides back out
   (`model::backend_spatial_gain`): each ear is multiplied by `min(1, 1/d²)` *and* panned.

## The technique

- **Judge a gain curve as data, never as audio.** A bus's gain is a step function sampled once per frame; plotted as a
  waveform it is a blob, plotted as a line it shows the duck's depth, hold and release exactly.
- **Judge a sample as a picture.** `showspectrumpic` with a log frequency scale and an explicit attack crop
  (`atrim=0:0.05`) and tail crop (`-sseof -0.1`) makes a click a vertical line, clipping a flat shelf at the top of the
  scale, a gap a hole, a denormal tail a faint floor that never fades.
- **Judge loudness in LUFS, not in peaks.** Peak tells a clipping story; LUFS tells an audibility story.

## Parameters, and where each comes from

| Parameter | Value | Source |
|---|---|---|
| dialogue duck on music / effects | −8 dB / −4 dB | Gregory 3e §14.6.3 |
| duck hold / fade | 0.25 s / 0.6 s (music), 0.4 s (effects) | Gregory 3e §14.6.3 |
| inverse-distance falloff | 1.0 at 4 m, 0.5 at 8 m, 0.25 at 16 m | inverse-distance law; −6.02 dB is half amplitude |
| bar at 160 bpm, 4/4 | 1.50 s | tempo arithmetic |
| loudness target | per item, in LUFS | ITU-R BS.1770-4 / EBU R128; **not a design default**, an external standard |
| render format | 22.05 kHz mono 16-bit | the format `examples/mixer` synthesises; small enough for `evidence/` |

## What good sounds like

A cue's spectrogram has one clean fundamental with harmonics that decay together, an attack under ~30 ms with no
vertical smear above it, and a tail that reaches the noise floor smoothly. A duck's line plot leaves 0 dB, reaches its
depth inside the hold, and returns to 0 dB inside the fade with no overshoot. A mix sums without a shelf at full scale.

## Pending index

Nothing unread for this run. A future item on loudness will need ITU-R BS.1770's gating algorithm itself (the
`ebur128` filter is the implementation, not the specification), and one on occlusion will need a propagation model
the corpus does not yet hold.
