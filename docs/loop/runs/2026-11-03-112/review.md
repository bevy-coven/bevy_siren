# Iteration 112 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest pass it
could affect.

## Pass 1 — Function

*Against the item's own acceptance — every format the scan calls audio decodes, from a real file, to a result checked against a decoder that
is not this crate's — and against the cue table: the 33 cases of `cues.yaml` are asserted by the matrix, unchanged, and the 21 rendered WAVs
are byte-identical to iteration 111's.*

**Critique, first reading — six things that were wrong, one of them mine twice:**

1. **I read a design choice as a decoder defect, and wrote it down in four places before I checked.** 22 of 51 files in the RPG pack differed
   from `libvorbis` by up to 3.5 % relative rms and 0.303 in a sample. I called it *"wrong samples in a minority of files"* in a decode doc comment,
   in an upstream-report draft, in two evidence files' names and headers, and in a pointer to a `needs-decision` file I had not yet written. **My own
   table gave it away**: the `peak` column was `1 + max|diff|` on every row, i.e. the crate's output was pinned at exactly 1.0 where `libvorbis` decoded
   to 1.3. `symphonia-codec-vorbis/src/dsp.rs:136` is `*s = s.clamp(-1.0, 1.0)`. The test: clamp `libvorbis`'s output the same way and all **209 files
   agree to 1.8 × 10⁻⁷** (`evidence/after/vorbis-clamp-explains-it.txt`). Retracted in each (the doc comment, the report, the evidence files, renamed), and the
   `needs-decision` file would have been the wrong home anyway: its README says only the public API, the message types and the definitions schema belong there.
2. **My control was wrong in eight files.** I compared against `ffmpeg`'s decode and took one outlier (`bookOpen`, 0.064) for the crate's error. It was the
   control's: `ffmpeg` returns up to 128 frames *fewer* than the stream declares in 8 files, and on a 0.15 s clip 128 frames moves a CLAP similarity by 0.06.
   `libvorbis` (libsndfile) is the reference, and the Ogg evidence is against it.
3. **A prediction written down before the run was half wrong.** `research.md` said MP3 at 128 kbit/s would move hit@1 by at most two points, zero-shot by at
   most one, and a typical similarity by under 0.01. Measured: **zero-shot −0.3 and −0.6 (inside)**, the typical similarity **0.005 (inside) and 0.033 at 64 kbit/s
   (outside)**, and **hit@1 +4 and +8 points (outside, and in the direction nobody predicted)**. The guide says so, with the cause marked as an untested inference.
4. **Two of my first Ogg assertions failed, and both were the test's fault**, not the decoder's: a plain zero-crossing count read 231 for a correct 220-cycle
   tone (a lossy codec chatters across zero), and the peak read 0.089 against an expected 0.125 (`ffmpeg -ac 2` puts a mono tone on each channel at −3 dB).
   Both were measured before being "fixed", and the tests now count cycles through a dead band and judge each file against the amplitude it was made with.
5. **The guide's *"the decoder costs no new crate"* was never true**, and the sentence the measurement contradicted was written before iteration 110's own
   `413 → 487` figure. `rodio` is in the default graph; its decoders are not. 16 of the 34 crates `semantic` adds are the decoders (ten `symphonia`), and
   the three places that said otherwise (`decode.rs`, `tests/leaf.rs`, `tests/library.rs`) now give the decomposition.
6. **Iteration 111's floor recommendation was contradicted by this repository's own library, which I had read two iterations ago.** The demo's five
   descriptions on fourteen macOS UI sounds scored nothing above 0.256 (*"a glass being tapped"* → `Glass` at 0.016): every positive floor would have returned an
   empty list. Found while planning the second library; the recommendation is revised to keep `0.0` and the default is untouched.

**Criteria, as met:**

| claim | test (failed on `7ddf91f`) | real-file evidence |
|---|---|---|
| every extension on the list decodes | `every_extension_the_scan_calls_audio_has_a_decoder` | `.flac`: *"The format of the data has not been recognized"* → decodes |
| FLAC | `a_flac_decodes_to_exactly_the_tone_that_was_encoded` (24 000 frames, 220 crossings, rms error < 1e-4) | 2 000 ESC-50 clips as FLAC: **232 000 similarities identical to the WAV's** |
| MP3 | `an_mp3_decodes_to_the_tone_that_was_encoded` | 62 Kenney MP3s vs `ffmpeg`: **259 of 259 pairs identical, every length exact** |
| Ogg | `an_ogg_vorbis_decodes_to_the_tone_…`, `an_ogg_is_cut_to_the_length_its_last_page_declares` (24 000 and 4 416, `libvorbis`'s own), `a_chained_ogg_is_not_cut_…` (red when the one-stream guard is weakened) | 209 Ogg effects vs `libvorbis`: samples to 1.8e-7; **tail up to 975 frames long in 59, now cut** — rank correlation min 0.727 → 0.998 on the RPG pack |
| the header | `the_header_of_each_format_gives_the_rate_and_the_channels_it_declares` | `(Some(768000), Some(1))` for a 48 kHz FLAC → `(48000, 1)` |
| the index | `a_folder_of_every_format_the_scan_calls_audio_indexes` | six files, six records, lengths right |
| the example | — (no test runs a model) | a 10 s FLAC waited 1.7 s for → 9.9 s; a 15 s MP3 5.8 s → 14.8 s |

**Verdict, first reading: NOT YET** (items 1 – 3 were claims that outran their evidence, and 1 had spread to four files). **After the fixes: GREAT.**

## Pass 2 — Zoom

*Read: the new `ogg_declared_frames` line by line against RFC 3533's page layout; `read_header`'s two new branches against both fixtures' hex; the two
spectrograms of the gate (`evidence/after/spectrograms/mix-spectrum.png`, `footstep-stone-spectrum.png`); the guide's new section and the rewritten decision file.*

**Critique.**

1. **`ogg_declared_frames` has three guards and a fourth would be tidier.** One logical stream (`BOS` count), the pages running unbroken to the end of the file
   (`offset == length`, which is also how a last page cut short shows, since `seek_relative` does not fail past the end), and the last page marked end-of-stream.
   A cap on the surplus (`MAX_END_TRIM_FRAMES`, 8 192, the longest Vorbis block) sits behind them. The chained-file test fails when the first is weakened
   (mutated and run: *"the first stream's audio is still there: 4416 frames"*); the cap has no test, because nothing a real file can do reaches it past the other three.
   Named, not hidden.
2. **The page walk costs a few small reads per page** on a file that is about to be decoded in full: a 5 MB Ogg is ~1 300 pages. Judged cheaper than reading the file's
   tail twice and scanning back for a capture pattern, which a packet's bytes can contain.
3. **`decode.rs`'s module doc had grown into a ledger** (what it costs, what it corrects, what it leaves). It is now three short paragraphs each starting with its subject.
4. **The first draft of the guide's lossy-copy paragraph said re-encoding "did not cost retrieval".** hit@1 rose, but zero-shot fell 0.3 – 0.6 points and the stranger's
   hit@10 fell four at 64 kbit/s. It now says *moved*, with all four.
5. **An aside, "I first read that as wrong samples", had got into the user guide.** Removed; it belongs here.

The audio gate: the master is a broadband attack in the first 25 ms and three steady tonal lines to the end — no click, gap, clipping edge or aliasing image; the stone footstep
is a 25 ms broadband attack, a harmonic series and a tail decaying to the floor. Both are **byte-identical** to iteration 111's WAVs (21 of 21, `cmp`), so they can only look the
way they did. Master **−21.5 LUFS, −7.5 dBFS true peak**, unchanged. Per-frame cost 0.062 / 0.354 / 1.258 µs cold and 0.018 – 0.020 / 0.106 – 0.116 / 0.396 – 0.427 warm at 8 / 64 / 256
voices: inside the recorded band (`perf.md`); no audio-path code changed.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*A different library, then every way to hold the tool wrong.*

**The different library.** ESC-50 is one library that people labelled; this iteration's evidence is mostly **Kenney's CC0 game audio** — 62 MP3s, 209 Ogg effects at two sample
rates, mono and stereo, 0.27 – 5 s — which is nothing like it, and the thing a game would actually point the crate at. The two did not tell the same story, which is the point:
on ESC-50 re-encoding *moved* retrieval by four to eight points of hit@1; on Kenney's effects the Ogg tail moved *rankings* (rank correlation 0.727 on the RPG pack), and the 22
files whose samples differed turned out to be a clamp, not a defect. Neither library alone would have shown both.

**The tool held wrong** (`evidence/after/edge-cases.txt`): an empty folder (*"no audio files in `empty` (the decoder reads: wav, ogg, oga, flac, mp3)"* — the list now comes from
`AUDIO_EXTENSIONS`, so the message cannot drift from the code again); a folder of a text file named `.mp3`, random bytes named `.flac`, a truncated `.ogg` and an empty `.wav`
(**0 indexed, 4 failed**, each named, exit 1, no index written); and a mixed folder (FLAC, Ogg, MP3 index and rank together; the text-as-`.mp3` is counted and named, exit 0).
**One thing that reads oddly and is not new**: the broken folder prints *"1 assets"* and *"none of the 1 files could be indexed"* beside *"4 failed"* — the scan skips three files by header
and counts one asset, which then fails to decode. Recorded in the log; not fixed here.

**Can a developer read the output without the code?** The example now names what it waits for (the decoded length), and the guide's new §2b says, per format, what it was checked
against and what the answer was. The one thing it cannot say is whether a played MP3 or FLAC *sounds* right: **nobody has listened**.

**Not covered, said plainly:** a GPU; **a person hearing** an Ogg, FLAC or MP3 through the mixer (`mixer` now decodes all five; no test plays a FLAC or an MP3); Opus, AAC and AIFF (not claimed,
so not tested); gapless loops; the second golden library (carved off, with its source found); the Cauldron's panel test (the editor pins `82d227e`, 215 commits behind, and its 14 GB target does
not fit); the upstream report (drafted, **not filed** — another project's tracker); the full ESC-50 *paraphrase* run at 64 kbit/s is one wording by one person, like every number in the set.

**Verdict: GREAT** (after the fixes in passes 1 and 2).
