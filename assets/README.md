# assets/

The two footsteps `examples/quick_start.rs` plays — the files the README's quick start names. **Synthesized for this repository and released under CC0 1.0 (public domain).** They are not part of the published crate (`Cargo.toml`'s `include` leaves `assets/` out): a crate user supplies their own recordings, a clone has these.

| file | length | what it is |
|---|---|---|
| `audio/step_01.ogg` | 0.221 s | brown noise (seed 11), high-passed at 90 Hz and low-passed at 1.7 kHz, a 4 ms fade in and an exponential fade out; peak −6.16 dBFS, RMS −23.9 dBFS |
| `audio/step_02.ogg` | 0.261 s | the same recipe on another seed (23), high-passed at 70 Hz and low-passed at 1.3 kHz, a 5 ms fade in; peak −6.10 dBFS, RMS −24.4 dBFS: darker and a little longer, so the two are audibly two steps |

They are placeholders: short enough not to overlap at the quick start's half-second step, quiet enough not to startle a reader who runs the example with the volume up, and with no click at either end (`docs/loop/runs/2026-11-04-120/evidence/spectrograms/`).

Made once with `ffmpeg 9.0.2`. Its native Vorbis encoder takes exactly two channels, so the one signal is written to both (`pan`) at full level:

```sh
ffmpeg -f lavfi -i "anoisesrc=d=0.22:c=brown:r=44100:a=0.9:seed=11" \
  -af "highpass=f=90,lowpass=f=1700,afade=t=in:d=0.004,afade=t=out:st=0.012:d=0.208:curve=exp,volume=5dB,pan=stereo|c0=c0|c1=c0" \
  -c:a vorbis -strict -2 assets/audio/step_01.ogg
ffmpeg -f lavfi -i "anoisesrc=d=0.26:c=brown:r=44100:a=0.9:seed=23" \
  -af "highpass=f=70,lowpass=f=1300,afade=t=in:d=0.005,afade=t=out:st=0.014:d=0.246:curve=exp,volume=5.4dB,pan=stereo|c0=c0|c1=c0" \
  -c:a vorbis -strict -2 assets/audio/step_02.ogg
```

## `audio/demo/` — the tour's sound set

The eleven recordings `examples/demo` and `examples/demo_render` play. **Synthesised by `tools/make_demo_sounds.py` and released under CC0 1.0 (public domain)**; not part of the published crate, as above. They exist so the demo can be *audible* in a clone with no downloads, and so the repository carries no recording whose licence has to be tracked.

| file | length | what it is |
|---|---|---|
| `step_01.ogg` | 0.220 s | filtered noise, 90 Hz – 1.7 kHz, exponential decay |
| `step_02.ogg` | 0.260 s | the same recipe, 70 Hz – 1.3 kHz: darker and longer |
| `step_03.ogg` | 0.190 s | 140 Hz – 2.4 kHz: brighter and shorter, so three footsteps are three sounds |
| `hit_01.ogg` | 0.260 s | a 70 Hz body plus a broadband edge: a hit |
| `click.ogg` | 0.050 s | a 1.1 kHz tick |
| `wind.ogg` | 9.600 s | a band-limited noise bed, two bars at 100 bpm, loopable |
| `music_calm.ogg` | 9.600 s | a four-bar pad at 196 Hz (root, fifth, octave), loopable |
| `music_combat.ogg` | 9.600 s | the same shape at 233 Hz, brighter |
| `stinger.ogg` | 0.600 s | a rising triad |
| `line_01.ogg`, `line_02.ogg` | 0.450 s | dry two-partial "voice" tones |

**Every loop is a whole number of bars at the score's tempo** (100 bpm, 4 beats: 2.4 s a bar), and every periodic component is evaluated on a phase that fits the loop an exact number of times, so the join is a continuation rather than a step. Measured: `music_calm.ogg`'s seam was **0.0544** before the phase correction, against a 99.9th-percentile step of 0.0330 inside the file — a click every pass — and is inside that percentile now (`tests/demo.rs`).

**The Vorbis padding, again.** A stream's last page declares its length and its last packet decodes to a whole block: `step_01.ogg` is 0.220 s of arithmetic and the decoder returns **0.232 s**. `-frames:a` does not change it (measured: a 9 702-frame input still decoded to 10 240). The declared length is the one to trust, which is what `src/ogg.rs` is for and what `tests/demo.rs` asserts.

Regenerate, and verify:

```sh
python3 tools/make_demo_sounds.py assets/audio/demo           # rewrites all eleven
python3 tools/make_demo_sounds.py --check assets/audio/demo   # fails if any differs by a sample
```

`--check` compares **decoded samples**, not bytes: an Ogg container carries a random stream serial, so two encodes of the same arithmetic differ in about twenty bytes and in no sample (measured: 21 bytes for `step_01.ogg`, 95 for `music_calm.ogg`, decoded PCM identical).
