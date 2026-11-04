# Draft upstream report: `symphonia-codec-vorbis` 0.5.5 — the padding of the last Vorbis block is returned

**Not filed.** It is another project's tracker, so filing it is the owner's call; this is the report, ready. Measured in iteration 112
(`evidence/after/decode-three-way.txt`, `ogg-extra-tail-level.txt`, `ogg-tail-vs-clamp.txt`).

## The defect

The Vorbis I specification (A.2, end of stream) says a decoder discards the part of the last packet beyond the last page's granule position.
`symphonia` returns it. Against `libvorbis` (libsndfile 1.2.2, `soundfile` 0.13.1), on 209 real Ogg effects from Kenney's CC0 packs (RPG, Sci-fi, Jingles —
`https://opengameart.org/sites/default/files/{RPGsounds_Kenney,sci-fi_sounds,jingleSounds_Kenney}.zip`): **59 files decode up to 975 frames long** (20 ms at
48 kHz) and none short. The extra frames are near-silent (median peak −69 dBFS, worst −35 dBFS), so it is inaudible; but a consumer that feeds short clips to a
model gets a different result from the length alone — on a 0.27 s effect it moved a CLAP similarity by up to 0.085. A fixture: any two-block Ogg whose last page
declares N frames decodes to more than N (`ffmpeg -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.5 -ac 2 -c:a vorbis -strict -2 t.ogg`: the last
page's granule is 24 000, `symphonia` returns 24 576).

Another project reports the same, 112 – 636 samples, and gives the cause as the Ogg end-granule trim being ignored
([storytold/soundcraft#991](https://github.com/storytold/soundcraft/issues/991)).

## What is *not* a defect, recorded so nobody reads it as one

`symphonia-codec-vorbis` clamps its output to [−1, 1] (`src/dsp.rs:136`), where `libvorbis` returns the overshoot a hot master decodes to. In the RPG pack 22 of
51 files decode above full scale (peaks up to 1.303), and the decoder's output differs from `libvorbis`'s on exactly those by up to 0.303 in a sample. **I first
read this as wrong samples and was wrong**: my own table had `peak = 1 + max|diff|` on every row. Clamping `libvorbis`'s output the same way makes all 209 files agree
with `symphonia` to 1.8 × 10⁻⁷ relative rms (`evidence/after/vorbis-clamp-explains-it.txt`). It is a design choice, harmless for playback, and moved a similarity by at most
0.004 here.

## What `bevy_siren` did

**Corrected on the indexing path only**: `semantic::decode::decode` cuts an Ogg to the length its last page declares, when the file is one logical stream that ends cleanly
(`tests/semantic_decode.rs::an_ogg_is_cut_to_the_length_its_last_page_declares`). `mixer` plays through `rodio` directly and still gets the near-silent padding. If this is fixed
upstream the cut becomes a no-op, and `formats_parity.py` and `decode-three-way.txt` rerun in a minute to show it.
