# Iteration 117 — perf: asking the decoder, once per asset

Machine: this Mac (arm64), shared; nothing heavy running during a measurement. Release builds. The raw output and the source as benchmarked are `evidence/after/undecodable-cost.txt` and `sounds-with-bench.rs.txt`; the probe's per-format
costs are in `evidence/before/undecodable-probe-on-the-unchanged-tree.txt`. **Measured on the shipped design** (the memo behind a `Mutex` in `SirenSounds`, shared by the two systems that ask); an earlier run on a per-system memo is noted in the cost file.

## 1. What the crate pays

| thing | cost | how it was measured |
|---|---|---|
| `refuse_unplayable_voices`, nothing waiting | **0.06 – 0.11 µs a frame** (iteration 116's system, which this one extends: 0.055) | one system in a single-threaded `Schedule`, 20 000 runs |
| … 256 running voices (a `MixerVoice`: skipped by the archetype filter) | 0.056 µs | same |
| … 8 / 64 / 256 voices waiting **with bytes present and an `AudioPlayer`** (a memo hit each, every frame) | 0.35 / 1.77 / **6.46 µs** | same — the state of a headless app or a machine whose device did not open; with a device Bevy starts a voice in its first frame and the next frame skips it |
| … 8 / 64 / 256 voices waiting for bytes, or the crate's mixer waiting | 0.07 – 0.09 / 0.24 – 0.26 / 0.78 – 0.81 µs | same |
| a memoised verdict (`Mutex` lock + key + one `HashMap` get + compare) | **21.1 ns** | 2 000 000 calls |
| `bytes_key` alone | 2.6 ns | same |
| **first sight of a Vorbis** | **0.8 ms** (815 µs: a decoder construction, the setup header's codebooks) | one call; the probe reads 0.64 – 0.75 ms whatever the length (4.5 KB to 105 KB) |
| first sight of a `hound` WAV | 0.1 – 0.2 µs | the probe, 200 calls |
| first sight of a `symphonia` WAV / MP3 / FLAC | 10 – 180 µs, growing with the file (5 MB WAV: 182 µs) | the probe, `--features mixer` |
| `explain()`, only on a refusal | 1.4 ns | 2 000 000 calls (the earlier run) |
| `perfcheck` (the mix's arithmetic; it does not run this system) | 0.060 – 0.062 / 0.321 – 0.362 / 0.571 – 1.216 µs at 8 / 64 / 256 voices | `evidence/after/perfcheck.txt`; iteration 116's runs were 0.059 – 0.064 / 0.336 – 0.355 / 1.189 – 1.245 |

**The price of knowing is one decoder construction per distinct asset — and Bevy pays the same construction on every play of it** (`AudioSource::decoder` builds a decoder each time it starts a voice). For a Vorbis that is 0.8 ms the first time a sound is
played and nothing after; the crate's first play of a sound costs about twice what Bevy's own decode of it costs, once. **"Once" is held by a test**: `sounds::tests::a_sound_is_decoded_once_by_the_crate_however_often_it_is_played` counts the constructions
for one buffer across four plays and four frames of the in-flight system — with the memo defeated it reads 14.

**Why the memo is shared and behind a `Mutex`** (a change made in review): the first cut held a `Local` memo in each of the two systems, so a sound's first play built a decoder in `play_requests` and again in `refuse_unplayable_voices` (the new voice is
visible to it in the same frame). Sharing costs about 6 ns a memo hit and 2 µs a frame at 256 never-starting voices; it saves 0.8 ms once per Vorbis.

## 2. What it does not cover (stated, not hidden)

- **A debug build.** `rodio`'s unoptimised Vorbis setup is many times slower, and Bevy's own per-play decode with it; I did not measure it.
- **A burst of first plays.** Twenty distinct Oggs started in one frame would cost about 16 ms of checking (20 × 0.8 ms) on top of Bevy's own decoding that frame. Validation at asset arrival or off the main thread would move it; neither is built, and the plan
  says what would trigger it.
- **A cold file cache and a real disk** do not enter: the bytes are in memory when the check runs.

## 3. The perfcheck rule

*"A change must not slow a case it did not target by more than 10 %."* `perfcheck` is inside iteration 116's band; the render's 21 WAVs are **byte-identical** to iteration 116's, so no cue's output moved.
