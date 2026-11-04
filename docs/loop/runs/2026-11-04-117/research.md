# Iteration 117 — research: bytes that load and do not decode are a refusal on one path and a crash on the other

*Written before any run. The predictions at the end are scored in the last section, wrong ones first.*

## What reading found (no program run yet)

- **`bevy_audio-0.20.1/src/audio_source.rs:96-102`**: `impl Decodable for AudioSource { fn decoder(&self) -> … { rodio::Decoder::builder().with_byte_len(self.bytes.len() as u64).with_data(Cursor::new(self.clone())).build().unwrap() } }`. **`.unwrap()` on a decode error.**
- `play_queued_audio_system` (`bevy_audio-0.20.1/src/lib.rs:113`, `PostUpdate`) calls `decoder()` for every entity with an `AudioPlayer` and no sink, whenever the device opened. Nothing in Bevy checks the bytes at load: `AudioLoader` returns `AudioSource { bytes }` and stops.
- This crate's voice on Bevy's path **is** an `AudioPlayer` entity (`play.rs:813`), `Played` is written for it, and its systems run in `Update` — **before** `PostUpdate`. The only place the crate decodes is `start_pending` (the crate's own mixer, behind `Output`): `play` returns `None` and the voice is refused (`playback.rs:92-105`). On the default build `start_pending` returns at once (`playback.rs:53-55`).
- **Four sentences say the crate refuses what the decoder refuses; on the default build it is a panic**: `Cargo.toml:158-159` (*"`Decoder::new` returns `UnrecognizedFormat` for anything else, which the backend warns about rather than panicking on"*), `playback.rs:8-10`, `play.rs:127-129` (*"the undecodable case when the bytes first load"*), `docs/guide/cues.md:147-149`. And the crate's own rule: *every play answers exactly one of `Played`/`Refused`; nothing plays on a refusal*.
- `tests/ogg_loops.rs` plays `b"not audio at all"` on Bevy's path and asserts what `trim_ogg_loops` does to it — **in a headless app, where no `AudioPlugin` runs the decoder**, which is why a state that crashes a game is a passing row.
- `bevy_audio` re-exports only `cpal::Sample`, `Source`, `ChannelCount`, `Sample`, `SampleRate` from `rodio` (`lib.rs:55`), **but `Decodable::Decoder` is `rodio::Decoder<Cursor<AudioSource>>`**, and `Decoder::builder()` / `DecoderBuilder::build()` are `pub` and fallible (`rodio-0.22.2/src/decoder/mod.rs:374`, `builder.rs:314`). So the exact call Bevy makes can be made **without** the crate depending on `rodio` in the default build (`<AudioSource as Decodable>::Decoder::builder()`).

## Sources

| # | source | what it gives |
|---|---|---|
| S1 | `bevy_audio-0.20.1/src/audio_source.rs:96-102`, `lib.rs:113` | the `unwrap`, and that it runs in `PostUpdate` for every unstarted `AudioPlayer` |
| S2 | `rodio-0.22.2/src/decoder/builder.rs:314` (`build`), `mod.rs:374`, and each decoder's `new(data) -> Result<_, R>` (`vorbis.rs:26`, `wav.rs:30`, `mp3.rs:30`, `flac.rs:34`) | a decoder declines by sniffing the first bytes and giving the reader back; `build` is `Err(UnrecognizedFormat)` when none accepts |
| S3 | Ousterhout, *A Philosophy of Software Design*, 2nd ed., **§10.8 "Just crash?"** (home-still `ousterhout_philosophy_of_software_design_2nd`) | crashing is right for errors that are *"difficult or impossible to handle and don't occur very often"* (out of memory, a disk hard error, an inconsistent data structure). A corrupt sound file is **easy** to handle (refuse the one play) and is **not rare** in authoring |
| S4 | same book, §10.7 (exception aggregation) | *"a series of requests … an exception that aborts the current request, cleans up the system's state, and continues with the next … clearly distinguished from exceptions that are fatal to the entire system"* — a play is a request |
| S5 | Kopetz's 12 design principles for dependable systems, in Marwedel, *Embedded System Design*, DOI 10.1007/978-3-030-60910-8, §5.6.4 (p. 296) | **3** *"fault-containment regions … faults in one FCR should not affect other FCRs"* (one bad file must not take the other sounds down); **11** *"every anomaly should be recorded"*; **12** *"provide a never-give-up strategy"* |
| S6 | this repository: `AGENTS.md` ("every play answers exactly one of `Played`/`Refused`. A refusal names what is missing; nothing plays on a refusal"), `docs/loop/needs-decision/undecodable-sound.md`, iteration 116's `sounds::cannot_load` | the design this extends: refuse at the play when the answer is known, refuse the voice in flight when it arrives later, the cause in the log, no new `Refusal` variant |
| S7 | formats, named and then **measured with real files** (not cited as numbers): an Ogg whose first packet is `OpusHead` instead of Vorbis's `\x01vorbis`; a Git LFS pointer (a ~130-byte text file with `version https://git-lfs.github.com/spec/v1`, `oid`, `size`); a WAV of a format tag `hound` does not read | the kinds of file that arrive with a good extension and bytes that are not audio the build can decode |

## The technique

**Ask the decoder, with the call Bevy makes, once per asset, before Bevy does.** `Decoder::builder().with_byte_len(len).with_data(Cursor::new(source.clone())).build()` is cheap (it sniffs a header and reads the stream's first packets) and is the *same code that would have panicked*, so the verdict cannot disagree with it. The crate's systems run in `Update`, Bevy's decode in `PostUpdate`, so a refusal in the frame the bytes are first seen keeps Bevy from ever being handed them. Not a heuristic on magic bytes (it would refuse a file the decoder reads, which silences a working sound, or accept one it does not); not `catch_unwind` (it prints the panic message anyway, needs `panic = "unwind"`, and treats a panic as flow control).

## Parameters

| thing | value | source |
|---|---|---|
| the decoder call | `Decoder::builder().with_byte_len(bytes.len()).with_data(Cursor::new(source.clone())).build()` | S1, copied exactly |
| when it runs | the first time a handle's bytes are seen at a play, or on a waiting voice; **once per `(asset, byte buffer)`** | design default |
| memo key | `(AssetId, buffer address, length, first 16 bytes)` — an edit replaces the buffer, so a reload is re-asked | design default |
| what is refused | `Refusal::MissingSound { sound }`, the cause in the log: *"the bytes of sound `x` cannot be decoded: …"* | S6 (option 4 of the decision record) |
| what is **not** touched | a handle with no bytes (a load in flight, a default handle) waits; the crate's own mixer (`Output` present) keeps deciding in `start_pending` | S6 |

## What good looks like

A bad file in the table: the **first play** of its cue is `Refused { MissingSound }` (no `Played`, no voice, nothing counted), the log names the key and carries the decoder's own words, **the app keeps running and the other cues play**, and a corrected file (a hot reload) plays on the next play. A good file is never refused. There is nothing to hear and no spectrogram to read: the evidence is a transcript, a panic before and none after.

## Predictions (written before any run)

- **P1.** On the unchanged tree, with the real `AudioPlugin` (this Mac has a device) and an LFS-pointer `.ogg`, the first `Play` writes `Played` and the app **panics within three frames**, the message containing `UnrecognizedFormat` and located at `bevy_audio-0.20.1/src/audio_source.rs:101`.
- **P2.** The decoder call, made through `<AudioSource as Decodable>::Decoder`, is `Err` for: an LFS pointer, a zero-byte file, a 40-byte cut of `tone.ogg`, an Ogg Opus file (95 %), an ADPCM WAV (80 %), a µ-law WAV (50 % — I do not know what `hound` reads), and `Ok` for `tone.ogg`, `tone-441.ogg`, a 16-bit WAV, the MP3 and the FLAC fixtures. **No file Bevy's own call can decode is refused.**
- **P3.** Cost: first sight of a ~20 KB Vorbis ≤ 300 µs, a WAV ≤ 60 µs, a memoised check at a play ≤ 100 ns, the new in-flight work with nothing waiting ≤ 0.1 µs a frame (iteration 116's system measured 0.055).
- **P4.** After the change, on the same real-device stack and the same files: **0 panics**, every bad row `Refused` at the play with the cause in the log, a good row `Played` and given a sink by Bevy in the next frame.
- **P5.** Exactly **one** existing test file needs a change — the `tests/ogg_loops.rs` rows that put undecodable bytes on Bevy's path, because that state can no longer exist — and no existing assertion about a *decodable* sound changes. The crate's mixer tests (`vec![0u8; 4]` placeholder bytes behind a fake `Output`) are untouched because `Output` owns playback there.
- **P6.** `rodio` need not become a dependency of the default build.

## Scorecard (written after the measurements; the wrong ones first)

**P3 — the first-sight cost was wrong by 2.5×.** I predicted ≤ 300 µs for a ~20 KB Vorbis; a Vorbis decoder is **635 – 750 µs to construct whatever the length** (4.5 KB, 5.7 KB and 105 KB files all within it — the setup header's codebooks, not the audio), **791 µs** in the benchmark with the insert. `hound`'s WAV is **0.1 – 0.2 µs**
(≤ 60 ✓), `symphonia`'s WAV, MP3 and FLAC 10 – 180 µs and *growing with the file* (a 5 MB WAV: 182 µs, past the 60 I predicted for a WAV in a build that has `symphonia`). The other two numbers were right: the memoised check **14.6 ns** (≤ 100), the system with nothing
waiting **0.04 – 0.11 µs** a frame (≤ 0.1, on the edge of a shared machine's noise).

**P2 — right about the six, wrong about two of the rest, and the reason is the finding.** The six bad rows are `Err` in every build. **µ-law is `Err` with `hound` (the default build) and `Ok` with `symphonia`'s WAV (`--features mixer`)**, which I had at 50 %; and **the MP3 and FLAC fixtures are `Err` in the default build** (Bevy's default features have no decoder for them) and
`Ok` where `rodio` has one — I had said `Ok`. *What a decoder reads is a property of the build*, so the permanent test is not a table of formats but an oracle: the crate refuses a play **exactly when `AudioSource::decoder()` panics**.

**P5 — right about the file, wrong about the rows.** Exactly one existing test file changed (`tests/ogg_loops.rs`) and the five mixer tests with placeholder bytes were untouched — but I predicted one row (*"bytes that are not audio"*); in the default build the **FLAC and MP3 rows** became refusals too (no decoder compiled in), so three rows changed and two of them now exist only where
the build decodes them.

**Right as written:** **P1** — six `Played`, six panics at `audio_source.rs:101:14`, `UnrecognizedFormat`, **in the first update after the play** (no `frame 1:` line is printed before the panic, so it was frame 1, inside the three I allowed). **P4** — 0 panics, six `Refused` at the play, a sink in frame 1 for the controls.
**P6** — `rodio` is not a dependency: the probe, the crate and its tests name only `<AudioSource as Decodable>::Decoder`.

**What the predictions did not contain, and the build found:** (1) a headless voice is never started (there is no device for Bevy to open), so a voice whose bytes are later *replaced* with undecodable ones is refused in flight — found by the edit test, which had to take the voice out of the way, and true of any no-device run;
(2) my first unit test said *"the same length and the same head"* of two buffers whose heads differed — found by mutating the key (D) and seeing the unit test stay green; (3) a benchmark of one system under Bevy's default executor reads 7 – 8 µs at zero voices: that is the executor, and the numbers above use the single-threaded one.
