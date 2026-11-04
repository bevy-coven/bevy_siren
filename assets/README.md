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
