# Iteration 117 — plan: bytes the decoder refuses are refused with the cause, not handed to a decoder that unwraps them

## Ranked (the claims audit is in `eval.md`; the cue table has no red case)

| rank | item | why it ranks there |
|---|---|---|
| **1** | **Bytes that load and do not decode panic Bevy's path — the default build — and four sentences say they are refused.** Measured on the real `AudioPlugin`: six kinds of file, six `Played`, six panics in the frame the play was answered, at `bevy_audio-0.20.1/src/audio_source.rs:101` | wrong output of the worst kind (a crash of the game), deterministic, on the default configuration, from the commonest delivery mistakes (a Git LFS pointer, an Ogg *Opus* file, an empty or truncated file, an ADPCM WAV), with a message that does not name the file |
| 2 | The layers of a score start apart when their loads straddle a frame (iteration 116: 3 of 100 warm-cache starts of eight stems, worst one frame, permanent) | real, rare, and nothing the crate says is untrue because of it; design in iteration 116's `plan.md` |
| 3 | `Play::on(owner)` for an owner that no longer exists: Bevy *warns and removes* the `ChildOf`, so a loop plays on with no owner and no end (`bevy_ecs-0.20.1/src/relationship/mod.rs:207`) | **not measured** — read, not run; a race between the owner's despawn and the play; carried |
| 4 | The crate's own mixer answers `Played` then `Refused` (same frame) for bytes it cannot decode; Bevy's path will now refuse at the play | a divergence this iteration *narrows* (a panic → a refusal) and does not close; needs an `AudioOutput` hook (public) |
| 5 | A person listens | not mine |

**One sentence on the pick:** a deterministic crash in the default build on the commonest authoring mistakes outranks a 3 % one-frame skew that nothing the crate says is false because of.

## The design

Ask the decoder, **with the call Bevy makes**, once per asset, before Bevy does.

1. **`sounds::decoder_error(&AudioSource) -> Option<String>`**: `<AudioSource as Decodable>::Decoder::builder().with_byte_len(len).with_data(Cursor::new(source.clone())).build()`, `Err` mapped to the decoder's own text. Reaches `rodio` through Bevy's associated type, so **`rodio` does not become a dependency of the default build** (`Cargo.toml` and `ALLOWED_DEPS` unchanged; measured: the probe names no `rodio`).
2. **One cache, in `SirenSounds`, behind a `Mutex`** (`HashMap<AssetId, (key, verdict)>` of std and Bevy types only; a clone starts empty), **shared by every system that asks** — so a sound's decoder is built once, not once per system. (The plan's first form was a `Local` memo in each system; see *What happened to the plan*.) **Key = (buffer address, length, first 16 bytes)**: an edited file is a new buffer, so a reload is asked again; the first 16 bytes close the allocator-reuse hole (a buffer freed and another of the same length placed at its address) for any change to a file's header. Pruned (`retain` ids still in `Assets`) when it outgrows twice the asset count + 16, so a game that makes and drops audio does not grow it.
3. **`sounds::cannot_play(files, handle, bevy_decodes) -> Option<Unplayable>`** replaces `cannot_load`: `FailedToLoad(error)` (iteration 116, unchanged), `Undecodable(error)` (new). The new arm runs **only when the bytes are there and Bevy's path owns the voice** (`!(backend.0 && output.is_some())`, the same predicate `spawn_voice` uses): with an `Output`, `start_pending` is the authority, as before.
4. **At the play** (`answer`): `Refused { MissingSound { sound } }`, no `Played`, no voice, nothing counted — and the log sentence carries the cause: *"the bytes of sound `lfs.ogg` cannot be decoded: The format of the data has not been recognized. — a Git LFS pointer, not the audio file"*.
5. **In flight** (`playback::refuse_unplayable_voices`, the renamed `refuse_failed_loads`): a waiting voice **with an `AudioPlayer`** whose bytes are now there and undecodable is refused and despawned **in `Update`, before Bevy's `PostUpdate` decodes it** (`bevy_audio-0.20.1/src/lib.rs:113`): `Played`, then `Refused`, the two answers a failed load already gives.
6. **A hint, never a decision** (`sounds::explain(bytes)`): the decoder's words are the same six times (*"The format of the data has not been recognized."*), and the three commonest causes can be told apart by their first bytes — a Git LFS pointer (`version https://git-lfs.github.com/spec/v1`), a file with no bytes, an Ogg whose first packet is `OpusHead` (RFC 7845 §5.1: *Opus in Ogg*, which this build's Vorbis decoder does not read). The verdict is always the decoder's; a hint only changes the sentence.

**No new field, no new `Refusal` variant, no new public item** (`needs-decision/undecodable-sound.md`, option 4, as in iteration 116). `play_requests` keeps its sixteen parameters (`Files` is still one tuple: server, assets, memo). **Every number is a design default or a measurement**: the 16-byte head, the `2 × assets + 16` prune bound.

## Tests (each fails first on the unchanged tree)

`tests/undecodable.rs`, declared in `tests/all.rs`; Bevy's path forced by removing the `Output`; fixtures: the repository's, plus `opus.ogg` and `adpcm.wav` (`ffmpeg`, commands in the file).

1. **a play of bytes the decoder refuses is refused with the cause** — rows: a Git LFS pointer, no bytes, a 40-byte cut of `tone.ogg`, a header that lies, an Ogg Opus file, an ADPCM WAV. Each: 0 `Played`, one `Refused { MissingSound { "s" } }`, 0 voices, nothing counted, a log line naming the key, *"cannot be decoded"* and a non-empty cause; the three hinted rows say so.
2. **the verdict is Bevy's own** — for every row *and every good fixture*: the play is refused **exactly when `AudioSource::decoder()` panics** (`catch_unwind`). A universal property, so it holds in every feature set (an MP3 is refused in the default build and accepted where `rodio` has `mp3`). *No file Bevy can decode is refused; none it would panic on is played.*
3. **bytes that arrive after the play** (a reserved handle, the asset inserted next frame) — undecodable: `Played`, then `Refused` in the frame they arrive, the voice and its `AudioPlayer` gone; decodable: untouched.
4. **an edit is asked again** — bad → good (the next play sounds) and good → bad (the next play is refused), same handle.
5. **the memo is bounded** — a unit test in `src/sounds.rs`: many assets validated, most removed, one more validated: the memo is pruned.

Existing tests: the five `tests/mixer.rs` rows with placeholder bytes behind a fake `Output` and `examples/budget.rs` are untouched by construction (`Output` owns playback). **`tests/ogg_loops.rs` has one row that builds the very state that crashes a game** (`b"not audio at all"` as a Bevy-path loop, asserted in a headless app that never runs Bevy's decoder); that state is unreachable now and the row is replaced by a test of the refusal.

## Carved off

- **The crate's own mixer refusing at the play** — `AudioOutput::decodes(&self, bytes) -> bool` with a default of `true` (so a fake output is unaffected) and the mixer's own decoder construction, so a refusal is one answer on both paths. A public trait method (additive); the mixer's `Decoder::new(Cursor)` and Bevy's builder with `with_byte_len` may disagree on a file, which is why the Bevy-path verdict is not reused for it.
- **`SirenSounds::undecodable(&Assets<AudioSource>, &CueTable)`**, the pull API beside `missing` and `failed`, so the editor can list a dead file before anyone plays it.
- **Items 2 and 3 above.**

## What would change this plan

- If a file Bevy's decoder reads is refused by the check (P2/P6), the check is wrong and nothing ships.
- If the first-sight cost of a Vorbis (0.65 – 0.75 ms measured) shows in a frame budget the game cares about, validation moves to asset arrival or off-thread, with a bound on the wait.
- If an existing test needs an assertion changed about a *decodable* sound, the premise that `Output` absent ⇔ Bevy decodes is wrong.

## What happened to the plan

It held in outline — probe first, predictions first, tests first, the decoder's own call, Bevy's path only, no variant, no dependency — and the build changed four things, all of them mine:

1. **The memo was a `Local` in each of two systems, and a sound's first play built its decoder twice.** `refuse_unplayable_voices` runs after `play_requests` in the same chain, a new voice is visible to it in that frame, and its own empty memo asked again: **0.8 ms twice for a Vorbis**, where the plan, the perf table and the CHANGELOG
   all said *once per asset*. Found by reading the diff as a reviewer, not by a test — there was no way to count constructions. The memo moved into `SirenSounds` behind a `Mutex`, `Files` went back to the shared pair, and a test-only record of every buffer a decoder was built for
   (`DECODED`) made *"once"* checkable: `a_sound_is_decoded_once_by_the_crate_however_often_it_is_played` reads **14** with the memo defeated and 1 with it. The benchmark was re-run on the shipped design (the Mutex costs about 6 ns a hit).
2. **`tests/ogg_loops.rs` had three rows to change, not one** (`research.md`, P5): the garbage row, and in the default build the FLAC and MP3 rows too. The first edit left `let mut rows` unused in a build without `mixer`/`semantic` — **a warning the matrix showed on three of its commands and clippy with `--all-features` cannot see**; the rows now come
   from a helper with one definition per feature set.
3. **A headless voice is never started**, so an unstarted voice whose bytes are replaced with undecodable ones is refused in flight; the edit test takes it out of the way. True of any no-device run, and harmless.
4. **My first unit test of the key said *"the same head"* of buffers whose heads differed**; mutating the key (D) left it green, which is how I knew. It now uses buffers that share their first 16 bytes and their length.

Carved off, as planned: the crate's own mixer refusing at the play (`AudioOutput::decodes`), `SirenSounds::undecodable` for the editor's panel, the ownerless loop for a vanished `Play::on` owner (read, not run), the layers' start barrier.
