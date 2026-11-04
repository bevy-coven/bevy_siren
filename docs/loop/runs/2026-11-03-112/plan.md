# Iteration 112 — plan: read what the scan calls audio

## Ranked

Every open issue was re-read (`#32`–`#34` the editor, `#35` open on a person's anchor judgements, `#39` decided not to ship by ADR 0005), and
iteration 111's top five against what the code actually does:

| # | item | verdict |
|---|---|---|
| 1 | **Format support is a claim nothing tested.** `AUDIO_EXTENSIONS` lists `flac` and `mp3`; `rodio` is given `wav` and `vorbis`; three docs and an error message say otherwise; the header parse for Ogg and FLAC has never run on a file | **chosen** — a refusal that should not happen (rank 1) |
| 2 | A second golden library (game audio: UI, foley, sci-fi, jingles) and the floor re-checked against it | **next, and urgent for the owner's decision** — see below. Its source is found and fetched (Kenney's CC0 packs on OpenGameArt, 9 zips, 14.5 MB), one pack of which is MP3 and wants item 1 |
| 3 | model-backed `update` (one new file costs one clip) | later |
| 4 | resampler fidelity (rubato vs scipy) | later; the metrics are unaffected |
| 5 | the editor, `#35`'s anchor pairs, `#39`, GPU | waiting on a person, another repository or hardware |

**Why 1 beats 2.** Both came from the same place — planning a second library showed that the Digital pack is MP3, and that the scan would
call it audio and the decoder would refuse it. A library of MP3s is the commonest library there is; `.mp3` and `.flac` are scanned,
counted, then fail by name — loud, but unusable, in a crate whose docs say they work. **2 is the better *experiment* and 1 is the worse
*bug*.** The cost of leaving 2 for the next iteration is that `needs-decision/default-floor.md` recommends 0.35 with a counterexample sitting
in this repository's own log: iteration 110's demo scored the *right* answer for "a short electronic beep" at **0.206** — a 0.35 floor would
have returned nothing. That caveat is added now (below); the measurement that quantifies it is the next iteration.

## What is wrong, and the evidence (before any fix; each is a test that failed on `7ddf91f`)

| # | defect | evidence |
|---|---|---|
| a | **`.mp3` and `.flac` are scanned as audio and cannot be decoded** | `decode` → *"The format of the data has not been recognized."* for both fixtures; `indexer` on a folder of them: 3 `failed` |
| b | **FLAC header: a 48 kHz mono file reports 768 000 Hz** | `read_header`'s shifts are wrong by 16×: `(Some(768000), Some(1))` |
| c | **Ogg header: the rate is read three bytes late** | by S4's layout `p + 8` is the last byte of the rate and the first three of `bitrate_maximum`; the new test covers it with a real 44.1 kHz file |
| d | the Ogg decode was **never tested**; MP3 and FLAC either | no test in `tests/` opened a file that was not a WAV |
| e | **`examples/semantic` waits for a file's length estimated as 16-bit PCM from its header** | for FLAC it is ~16× short (and the header is wrong); for MP3 it is a flat 5.75 s — a longer file is cut off and the example says *done*. Verified after (a) is fixed, with a 5 s FLAC |

(c) and (d): the Ogg **samples** were fine and its **length** was not — which this plan did not know when it said "the Ogg decode is fine"; see *Changed
after the plan* below. The first two assertions I wrote for the tone test failed (231 crossings; a peak of 0.089) and both were **my test's** error — a lossy codec
chatters across zero, and `ffmpeg -ac 2` puts a mono tone on each channel at −3 dB, so the peak *should* be 0.0884. The tests were corrected to measure what the
fixture is.

## Design

**One change of substance per defect, each with a failing-first test.**

1. `Cargo.toml`: `rodio` features `+ "flac", "mp3"`. Cost, measured: **`--features semantic` 487 → 497 lines (two crates,
   `symphonia-bundle-flac` and `symphonia-bundle-mp3`); `--features mixer` 450 → 460; the default build 413, unchanged.** Pure Rust, no
   C toolchain; `mixer` plays both formats too, since it is the same decoder (`decode.rs`'s own invariant: *a file that plays is a file
   that indexes*).
2. `library::read_header`: Ogg rate at marker + 5; FLAC rate `(s[10] << 12) | (s[11] << 4) | (s[12] >> 4)`.
3. `examples/semantic.rs`: wait for **the decoded length** (`decode(path).seconds()`), not a header-and-size estimate.
4. Docs that were false become true or are corrected: `decode.rs`'s "costs no new crate", the guide's format list and `cargo tree` numbers, the
   example's *"WAV and Ogg Vorbis are what the decoder reads"*, `tests/leaf.rs` and `tests/library.rs`'s "adds nothing to the graph".

## Decided and recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| enable `flac` + `mp3` or drop them from the list | **enable** | the list is what libraries contain (S5); the cost is two pure-Rust crates (measured); dropping would turn a loud failure into a documented one for the commonest format |
| MP3 header: parse the frame header for rate and channels | **no** | the scan's contract is *unknown is said, not guessed*; the decoder is where the numbers come from, and nothing reads the scan's for correctness |
| new `DecodeProblem` variant for "format not compiled in" | **no** | rodio's own *"format … not recognized"* already names the file; a variant is a public-API change for no new information |
| tolerance on lossy length | **±5 %**, and a 0.1 s Vorbis file may be ~30 % long | **design default**; a Vorbis stream is rounded up to a whole block (measured: 5 573 samples for a 4 800-sample clip) |

## Fields

None on `CueTable` / `SirenSettings` / `MusicScore`; no public item added or changed. `Asset::{sample_rate, channels}` now carry the right
values for Ogg and FLAC (they were wrong).

## Tests

New, failing-first on `7ddf91f` (six, all failed there; the second column is what they said):

| test | said on the old crate |
|---|---|
| `library::the_header_of_each_format_gives_the_rate_and_the_channels_it_declares` | `(Some(768000), Some(1))` for a 48 kHz mono FLAC |
| `semantic_decode::a_flac_decodes_to_exactly_the_tone_that_was_encoded` | *the format of the data has not been recognized* |
| `semantic_decode::an_mp3_decodes_to_the_tone_that_was_encoded` | the same |
| `semantic_decode::an_ogg_vorbis_decodes_to_the_tone_that_was_encoded` | passes once its two assertions were corrected (see above) |
| `semantic_decode::every_extension_the_scan_calls_audio_has_a_decoder` | *`.flac` is on the scan's list of audio and does not decode* |
| `semantic_indexer::a_folder_of_every_format_the_scan_calls_audio_indexes` | 3 `Failure`s: `d.flac`, `e.mp3`, `f.flac` |

Fixtures: `tests/fixtures/tone{,-441}.{flac,ogg}` and `tone.mp3`, 41 KB in all, made with `ffmpeg 9.0.2`; the commands are in
`tests/semantic_decode.rs`.

**Real-library evidence** (no test: CI has no model and no network): FLAC at the index level — ESC-50 as FLAC must give *the same
similarities* as ESC-50 as WAV; MP3 — Kenney's 62-file Digital pack against an `ffmpeg` decode, and ESC-50 at 128 and 64 kbit/s against the
prediction in `research.md`; Ogg — Kenney's Ogg packs against `ffmpeg`; the example with a 5 s FLAC before and after (3).

## Carved off

- **The second golden library** and the floor re-check against it (item 2 above).
- **The caveat now:** `needs-decision/default-floor.md` and the guide gain one paragraph — the repository's own demo scored a correct match at
  0.206, below the recommended floor — so the recommendation does not sit unqualified for a synthetic or UI library.
- MP3 header parsing; gapless playback; Opus, AAC and AIFF (not claimed, so not tested).

## Changed after the plan, and why

The plan was written before any real library had been run through the new decoders. Four things changed once one had:

1. **A fifth defect, found by the parity run on real Ogg files: the end of the stream was returned up to 20 ms late** (59 of 209 files). `symphonia` keeps the
   padding of the last Vorbis block. On short effects it changed the top three of every description tried. `decode` now cuts an Ogg that is one stream ending
   cleanly to its last page's granule position — with a failing-first test whose expected lengths are `libvorbis`'s own, a chained-file test that goes red when its guard
   is weakened, and a measurement that the worst difference against `libvorbis` falls from 0.085 to 0.004.
2. **A retraction.** The 22 files that differed *mid-file* were read as a decoder defect and are the decoder clamping to ±1 (`dsp.rs:136`); `libvorbis` returns the overshoot.
   No decision file was needed, and none was written (`review.md`, item 1).
3. **The control was not the reference.** `ffmpeg` is up to 128 frames short of the stream's declared length in 8 Ogg files; the parity is against `libvorbis`.
4. **The floor caveat got stronger.** The plan said one query scored 0.206; the whole recorded table shows **nothing above 0.256** on fourteen UI sounds, so every positive
   floor would have returned nothing, and the recommendation was revised, not merely caveated.

And the decisions table above held except one row: *"a new `DecodeProblem` variant"* — still no; and *"tolerance on lossy length ±5 %, a 0.1 s Vorbis file may be ~30 % long"* —
**no longer true**: after the end trim a Vorbis file is exactly its declared length, and the indexer test says 4 800 – 4 815 for a 0.1 s file, not up to 6 200.
