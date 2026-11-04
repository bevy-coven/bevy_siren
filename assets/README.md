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

The fourteen recordings `examples/demo` and `examples/demo_render` use. **Synthesised by `tools/make_demo_sounds.py` and released under CC0 1.0 (public domain)**; not part of the published crate. They exist so the demo is audible in a clone with no downloads, and so the repository carries no recording whose licence has to be tracked.

| file | length | what it is |
|---|---|---|
| `step_01..03.ogg` | 0.19–0.26 s | three filtered-noise footsteps, each a different band: the variants that stop a foot sounding mechanical |
| `hit_01.ogg` | 0.26 s | a low thump with a click edge |
| `click.ogg` | 0.05 s | a 1.1 kHz tick |
| `note_low/mid/high.ogg` | 0.5–0.9 s | **plucked** notes: harmonic partials that decay faster the higher they are, plus a noise attack |
| `bass.ogg` | 0.9 s | a plucked low note, no drone |
| `music_calm.ogg`, `music_combat.ogg` | 9.6 s | **phrases**: plucks on a beat grid, four bars at 100 bpm, with silence between the notes |
| `stinger.ogg` | 0.7 s | a plucked three-note riser |
| `line_01/02.ogg` | 0.45 s | dry two-partial tones standing in for a spoken line |

**Why this set was rewritten.** The first version had a **noise bed** (`wind.ogg`) and an organ-like pad — three pure sines
at 196/294/392 Hz, spectral flatness **0.0005** — playing from the third scene of the demo onward. The demo's quietest
50 ms frame after its fifth second was therefore **−28 dBFS**: it was heard as a hum. Every loop is now a *phrase*, so
between notes the file is **silent** (31–34 % of each music file is below −60 dBFS), and the bed is gone.

**Every loop is a whole number of bars** at the demo's tempo (100 bpm, 4 beats: 2.4 s a bar), and every periodic component
is evaluated on a phase that fits the loop an exact number of times, so the join is a continuation rather than a step.
Measured: `music_calm.ogg`'s seam was **0.0544** before the phase correction, against a 99.9th-percentile step of 0.0330
inside the file — a click every pass — and is inside that percentile now (`tests/demo.rs`).

**The Vorbis padding, again.** A stream's last page declares its length and its last packet decodes to a whole block:
`step_01.ogg` is 0.220 s of arithmetic and the decoder returns **0.232 s**. `-frames:a` does not change it (measured: a
9 702-frame input still decoded to 10 240). The declared length is the one to trust, which is what `src/ogg.rs` is for and
what `tests/demo.rs` asserts.

**Encoding.** ffmpeg's native Vorbis encoder takes exactly two channels (`-af pan=stereo|c0=c0|c1=c0`) or fails `Could not
open encoder before EOF`, and the temporary file needs the **output** suffix — a `.wav` name once made the encode fail.

Regenerate, and verify:

```sh
python3 tools/make_demo_sounds.py assets/audio/demo           # rewrites all fourteen
python3 tools/make_demo_sounds.py --check assets/audio/demo   # fails on a difference, or on a file it does not make
```

`--check` compares **decoded samples**, not bytes: an Ogg container carries a random stream serial, so two encodes of the
same arithmetic differ in about twenty bytes and in no sample. It also reports a file in the folder that the script does
not make — which is how the old noise bed was caught after it was removed.
