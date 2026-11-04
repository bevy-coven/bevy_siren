# Iteration 115 — research: an Ogg loop is as long as its stream says it is

## The question

Both playback paths loop whatever the decoder returns, and `symphonia` returns the **padding of the last Vorbis block** (iteration 112: up to 975 frames, 20 ms). **(1)** Where does the spec put a stream's true length, and what must a player do with the rest? **(2)** Is a gap that long audible, and what
does it do to a stem on the bar? **(3)** How can each path be *told* the length without a decoder of its own? **(4)** What arithmetic gives exactly `N` samples?

Every `rodio`, `bevy_audio` and `symphonia` fact is read from the vendored registry source (`~/.cargo/registry/src/index.crates.io-*/`), as `AGENTS.md` requires; the specs are read from their primary documents.

## Sources

| # | source | what it establishes |
|---|---|---|
| S1 | **Vorbis I specification**, Appendix A.2 (`xiph.org/vorbis/doc/Vorbis_I_spec.html`, read 2026-11-03) | *"A granule position on the final page in a stream that indicates less audio data than the final packet would normally return is used to end the stream on other than even frame boundaries. The difference between the actual available data returned and the declared amount indicates how many trailing samples to discard from the decoding process."* Also: header pages carry granule zero; *"the first audio packet … begins a fresh Ogg page"*; the last page is marked end-of-stream; a page's granule is in **PCM samples per channel**; and the first page's granule *"need not indicate that the stream started at position zero"*. |
| S2 | **RFC 3533**, *The Ogg Encapsulation Format* (Pfeiffer 2003), §4–6 | A page is a 27-byte header (`OggS`, version 0, `header_type` — `0x02` bos, `0x04` eos —, an 8-byte little-endian granule position, serial, sequence, CRC, segment count) and a segment table; a packet ends at a lacing value below 255. **Grouping** puts every bos page at the head of the file; **chaining** concatenates complete streams — a file with more than one bos has no single length. |
| S3 | `bevy_audio-0.20.1/src/audio.rs:66-70` and `audio_output.rs:160-161` | `PlaybackSettings::duration`: *"If the playback mode is set to `Loop`, each loop will last for this duration"*; the sink is built as `decoder.take_duration(duration).repeat_infinite()`. **The default path can be told a loop's length without a decoder of the crate's own.** |
| S4 | `rodio-0.22.2/src/source/take.rs:59-63`, `:101-126`, `:136-147` | `TakeDuration` emits a sample while `remaining > p` and then subtracts `p`, where `p = ⌊10⁹ / (rate × channels)⌋` ns (integer). It emits exactly `⌈remaining / p⌉ − 1` samples: **`remaining = N·p + 1 ns` gives exactly `N`**. |
| S5 | `rodio-0.22.2/src/decoder/{builder.rs:87, symphonia.rs:67-70}`; iteration 112's `evidence/after/ogg-extra-tail-level.txt` | `gapless: true` is the default and is passed to symphonia as `enable_gapless`, **and symphonia still returns the last Vorbis block's padding** (59 of 209 Kenney Ogg effects up to 975 frames long, none short; the extra frames' peak: median −69 dBFS, worst −35). `storytold/soundcraft#991` reports the same (112 – 636 samples). |
| S6 | Brennan, McCreery, Buss & Jesteadt 2018, *Ear and Hearing*, DOI `10.1097/aud.0000000000000558`; Penner 1977, *JASA* 61:552, DOI `10.1121/1.381297`; Tyler et al. 1982, DOI `10.1121/1.388254`; Moore, Peters & Glasberg 1992, DOI `10.1121/1.405240` | **The shortest silence a listener hears between two noise bursts is about 3 ms** (the mean gap threshold of normal-hearing adults for 60 dB SPL broadband noise, Brennan et al. citing Irwin et al. 1981); Moore et al. measured gaps in **sinusoids** from 100 to 2 000 Hz, where thresholds are higher (the abstract gives no figure at 440 Hz). |
| S7 | Khan et al. 2023, arXiv:2303.15734; `docs/research/siren.md` R5; `docs/guide/music.md:20-22` | Adaptive music is per-instrument gain keyed to game state, *"the same tempo and beat throughout"*; the crate promises *"every layer phase-aligned"*. A stem is a loop, so its **length** is its tempo. |

## The technique, in the sources' terms

The stream's length is **the granule position of its last page** (S1, S2). A player discards everything past it. `symphonia` does not (S5), and neither path's loop asks (S3, `mixer/mod.rs::looped`). The crate already reads that granule — `semantic::decode::ogg_declared_frames` (iteration 112) cuts an *index's* clip to it —
but only where a model would notice. This iteration moves the page walk into a core module and hands the result to each path in the form it accepts: **`PlaybackSettings::duration` on Bevy's path** (S3), **`take_duration` on the mixer's**, both through the same exact arithmetic (S4).

**What a player cannot assume**: the decoded length is not known without decoding, so on the playback paths a **plausibility check** replaces `semantic::decode`'s surplus cap. A well-formed stream with `N` audio packets and block sizes `bs0 ≤ bs1` decodes to between `(N − 1)·bs0/2` and `(N − 1)·bs1/2` frames (S1: each packet returns a quarter
of the previous block and a quarter of its own), and the declared length is that decoded length less at most one block's padding (`bs1/2`). A granule outside `((N − 1)·bs0/2 − bs1/2, (N − 1)·bs1/2]` is refused and the loop is left as it was.

## Parameters the sources report

| value | where | what it means here |
|---|---|---|
| padding up to **975 frames** (20 ms at 48 kHz), median peak **−69 dBFS**, worst **−35 dBFS** | S5 | the size of the gap; near-silence, not signal |
| gap threshold **≈ 3 ms** | S6 | 12 ms (the fixture's 576 frames) is four times it, 20 ms nearly seven |
| `p = ⌊10⁹ / (rate × channels)⌋` ns | S4 | 10 416 at 48 kHz stereo, 11 337 at 44.1 kHz stereo, 20 833 at 48 kHz mono, 45 351 at 22.05 kHz mono |
| `remaining = N·p + 1` | S4 | exactly `N` samples; `N / rate` as a `Duration` is **not**: `p` is truncated by up to 6.4 × 10⁻⁵ of itself |
| block sizes `2^6 … 2^13` frames | Vorbis I §4.2.2 (`decode.rs`'s `MAX_END_TRIM_FRAMES` = 8 192) | the plausibility window's `bs0`, `bs1` |

## What good sounds like

A loop of 220 whole cycles of 440 Hz is **one unbroken line** in a spectrogram, however many passes. With the padding there is a break at every wrap: 12 ms of near-silence in a 0.5 s pass, so a visible notch every 0.512 s. A waveform picture shows the break as a thin neck in the envelope. After the trim the pass is exactly 24 000 frames
and the first 64 samples of pass 2 are the first 64 of pass 1, offset by exactly one pass.

## Predictions (written before anything below was measured; iteration 112 had measured the fixture's decoded length, 24 576, and the tail's level over a library)

| # | prediction | how it is checked |
|---|---|---|
| P1 | `tests/fixtures/tone.ogg` (declares **24 000** frames) loops, through Bevy's pipeline as it is, with a **period of 24 576 frames** — 576 frames, **12.0 ms**, late at every wrap | find where the first 64 samples recur in `decoder.repeat_infinite()` |
| P2 | the 576 extra frames peak **below −40 dBFS** (the tone is −21 dBFS per channel); iteration 112's median was −69 | the peak of frames 24 000 … 24 575 |
| P3 | with `take_duration(N·p + 1 ns)` the period is **exactly 24 000** and the sample after the wrap differs from the sample before it by no more than **1.5× the tone's largest step** | the same recurrence search; the step at the wrap |
| P4 | `Duration::from_secs_f64(frames / rate)` — the obvious spelling — is **wrong by one sample or more** for most of a grid of rates {22 050, 32 000, 44 100, 48 000, 96 000} × channels {1, 2} × lengths; `N·p + 1` is **exact for every cell** | `TakeDuration` over a `SamplesBuffer` of each size, counted |
| P5 | on Kenney's **254 Ogg files**, between **20 % and 35 %** decode longer than they declare (112: 59 of 209), none by more than **1 024 frames**, none shorter; after the trim **254 of 254** are exactly their declared length | `rodio` decode against the page walk, every file |
| P6 | the page walk costs **under 100 µs** for an effect and **under 2 ms** for a 5 MB file (about 1 300 pages) — noise beside the decode it precedes | timed on the real files, scaled |
| P7 | a looping Ogg voice on Bevy's path has its `duration` **set in the frame its asset is present, before Bevy starts it**; a loop of any other format, a one-shot, and a chained Ogg get **none** | a headless app, real fixtures |
| P8 | the plausibility window **refuses nothing in the 254 real files** (it is a guard against a wrong granule, not a filter on right ones) | the same population |

## What this does not settle

- **Whether a person hears the 12 ms.** S6 says the threshold is ≈ 3 ms for noise and higher for low tones; nobody has listened to this crate's loops (iteration 112, 114). The render and the pictures are what a machine can show, and the review says so.
- **A stream that starts at a positive granule** (S1: *"a positive starting value simply indicates that this stream begins at some positive time offset"*) declares its end *position*, not its length. The reader does not detect it; the plausibility window refuses most such files and the rest are trimmed to a length that is at most the real one
  plus the offset. Named, not solved.
- **MP3** is gapless through `symphonia` only when the file carries a LAME/Xing tag (iteration 112 measured every length identical to `ffmpeg`'s on 259 files, which apply the same tag); a tagless MP3 is not, and this iteration does not touch it.

## Scorecard (written after the measurement; the wrong ones first)

**P1 — wrong, and the wrongness was the finding.** Predicted a period of 24 576 frames through Bevy's pipeline *as it is*. In this crate's own builds the pipeline was `rodio`'s `symphonia` Vorbis decoder (`Cargo.toml` named `vorbis`), and
`decoder.repeat_infinite()` of it produced **0 of 147 456 samples** — no loop at all (`evidence/before/ogg-loop-report-on-symphonia.txt`). The prediction came true only on `lewton`, Bevy's own default: a period of 24 576 frames = 512.000 ms
(`evidence/after/ogg-loop-report.txt`). The research read `audio_output.rs:165` and `mixer::looped` and wrote *"loops the decoder as it comes"* without a line that **looped an Ogg**: the claims audit asked whether any test read a
loop's length (none did) and not whether **any test looped an Ogg at all** (none did — every loop test used a WAV). That question is now in the build-loop skill.

**P5 — right on `lewton`, calibrated on the wrong number.** 51 of 254 (20.1 %) decode longer, by 27 – 936 frames, none shorter, and 254 of 254 are exact after the trim: inside 20 – 35 %, at its floor. The prediction extrapolated
iteration 112's `symphonia` count (59 of 209, built from a `File`, which tells the decoder the file's length); the same 254 files on `symphonia` built with `Decoder::new(Cursor)` — the mixer's construction — are **253** longer, by up to 1 019. **That is a different decoder, and a different construction, and I did not know which the bound was for**; whether the construction alone moves the count was not measured (the two were never run on one file set).

**P2 — right for the fixture, silent about the population.** The fixture's 576 frames peak at −56.6 dBFS (< −40 ✓), but across the 51 files the median padding peak is −48.9 dBFS and **the worst is −6.0 dBFS**: "near-silence"
is the typical case, not the guaranteed one.

**P6 — right, loose by three orders of magnitude.** 0.03 µs median (13.9 KB), 0.53 µs the slowest real file (188 KB), 8.2 µs for a 9.14 MB file, against bounds of 100 µs and 2 ms: the walk reads about 30 bytes a page.

**P3 ✓** period exactly 24 000 frames; the wrap step 0.0073 against the tone's largest step inside a pass 0.0054, 1.35× (< 1.5×). **P4 ✓** `frames / rate` wrong in 77 of 108 cells (worst +158 samples), `N·p + 1` wrong in 0 of 108
(a grid of 9 rates × 2 channel counts × 6 lengths, wider than predicted). **P7 ✓** `tests/ogg_loops.rs`. **P8 ✓** the walk understood 254 of 254.

Five of eight were right as written; the two that matter were wrong or mis-calibrated, and both because a number from a *different decoder* was carried into a prediction about this one.
