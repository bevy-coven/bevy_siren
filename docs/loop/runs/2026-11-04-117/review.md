# Iteration 117 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest pass it could affect.

## Pass 1 — Function

*Against the item's own acceptance — bytes that load and cannot be decoded are refused with the cause on Bevy's path (no `Played`, no voice, no `AudioPlayer` for `PostUpdate`), a voice whose bytes arrive undecodable after the play is refused in the frame they arrive, and a file the decoder reads is never refused — and against the cue table: 38 cases (37 held, one new), none red; the 21 rendered WAVs are byte-identical to iteration 116's (21 of 21).*

**Critique, first reading — eight things that were wrong in my own work:**

1. **A sound's first play built its decoder twice, and every document I had written said *once*.** I gave each of the two systems that ask (`play_requests`, `refuse_unplayable_voices`) its own `Local` memo. The second runs after the first in the same chain, a voice spawned this frame is visible to it, and its empty memo asked again: **0.8 ms twice for a Vorbis**. Nothing could
   see it — there was no way to count constructions — and I found it only by reading the diff as a reviewer. The memo moved into `SirenSounds` behind a `Mutex` (shared by every system that asks), `Files` went back to the shared pair, a `cfg(test)` record of every buffer a decoder was built for made *"once"* checkable, and
   `a_sound_is_decoded_once_by_the_crate_however_often_it_is_played` reads **14 with the memo defeated and 1 with it**. The benchmark and `perf.md` were redone on the shipped design (21 ns a memoised verdict, 6.5 µs a frame at 256 voices that cannot start).
2. **The first edit of `tests/ogg_loops.rs` left a warning that clippy cannot see.** `let mut rows` is unused in a build without `mixer`/`semantic` (its `extend` is `cfg`-gated), and the matrix's clippy runs with `--all-features`. I had run clippy first after every edit this iteration, as promised — and it was clean — and the warning was still on three commands of the matrix log, which
   I found by grepping it for `warning` instead of reading only the exits. The rows now come from a helper with one definition per feature set. **Clippy with all features is not a check of the default build.**
3. **My first unit test said *"the same length and the same head"* of two buffers whose heads differed.** Mutating the key (D: the address forgotten) left it green, which is how I knew; it now uses buffers that share their length and first 16 bytes, and goes red.
4. **Three predictions were wrong or half-wrong** (`research.md`, scorecard): the first-sight cost of a Vorbis by 2.5× (0.64 – 0.8 ms, not ≤ 0.3), what a decoder reads (a property of the *build*: µ-law is `Err` with `hound` and `Ok` with `symphonia`; MP3 and FLAC are `Err` in the default build — which is why the permanent test is an oracle, not a table),
   and the number of `ogg_loops` rows (three, not one). The probe came *after* a reading that told me what to expect, but the predictions were written before any program ran.
5. **The first benchmark measured the executor.** One system in a default (multi-threaded) `Schedule` read 7 – 8 µs a run at zero voices — a hundred times iteration 116's number — which is how I knew it was not the system. Re-run single-threaded.
6. **The first probe's `frame` lines were filtered out by my own `grep -v`**, so the frame of the panic was not visible; re-run with them (no `frame 1:` line precedes the panic: it is the first update).
7. **Mutations C and D were run on the per-system design.** `verdict_with` and `bytes_key` did not change when the memo moved, and F (the memo defeated) was run on the final code, but **I did not re-run C and D after the move**.
8. **Small slips, all caught by a read or the tool**: an `AGENTS.md` edit whose body repeated a line from outside its range (the tool refused it and showed the real lines); a `CUT` that needed the whole range displayed; the first `ffmpeg` call for a 30-second Ogg (this build has no `libvorbis`; the native encoder was used); a multi-hunk edit refused whole.

**What passed.** The probe, before and after, one process per file, the real `AudioPlugin` on this Mac's device (`evidence/before/…probe…`, `evidence/after/undecodable-probe-after.txt`): a Git LFS pointer, an empty file, a 40-byte cut of a real Ogg, an Ogg Opus file, an ADPCM WAV and a µ-law WAV — **six `Played` and six panics at `audio_source.rs:101:14`
in the first update after the play; after: no `Played`, six `Refused`, 0 voices, 0 sinks, 0 panics**, the log naming the sound, carrying the decoder's words and — for the three it can tell — saying what the file is; the two controls `Played` and given a sink in frame 1, as before. **Four of the five integration tests are red on the unchanged tree and the control passes**
(`evidence/before/undecodable-tests-on-the-unchanged-tree.txt`), and one of them holds **that the crate refuses exactly the bytes `AudioSource::decoder()` panics on** (`catch_unwind`), for six undecodable rows and four good fixtures, in every feature set. Guards switched off in turn (`evidence/before/mutations.txt`): the play-time check
(3 of 5 red), the in-flight check (1), the memo's key check (1 + 1 unit), the address in the key (1 + 1 unit), the memo itself (1 unit, 14 constructions), and — the one that explains the design — **applying the check to the crate's own mixer reddened six existing `tests/mixer.rs` tests**. The final matrix (`matrix-final.log`):
see the table in `eval.md`. Render: 21 of 21 WAVs byte-identical, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, spectrogram read (`evidence/spectrograms/mix-spectrogram.png`); `perfcheck` 0.060 – 0.062 / 0.321 – 0.362 / 0.571 – 1.216 µs at 8 / 64 / 256 (inside iteration 116's band; it cannot see this system).

**Verdict: GREAT.** After the fixes above the item's acceptance is met on Bevy's path with a real device, and its guards are mutation-checked.

## Pass 2 — Zoom

*The source diff, line by line, as a reviewer who did not write it — `src/sounds.rs`, `src/play.rs`, `src/playback.rs`, `tests/ogg_loops.rs`, `tests/undecodable.rs` — and every sentence I wrote about it.*

**Critique, first reading:**

1. **`SirenSounds` is a public type and I changed it**: a private `Mutex` field, and `Clone` and `Debug` implemented by hand (a clone starts with an empty cache; `Debug` prints the handles and `..`). Every trait it had it still has, no field is public, and nothing in the repository reads its `Debug`; but it is the one public type this iteration touched, and the CHANGELOG's *"no new public item"* is
   true only in that sense. Said here and in the log.
2. **`play_requests` gained no parameter** (`Files` is still one tuple, now the pair again), and **`refuse_unplayable_voices` gained one and a `#[allow(clippy::too_many_arguments)]`**, which the file's other systems carry too. The old name is gone everywhere outside the historical records (`grep` for `refuse_failed_loads` and `cannot_load` finds nothing in `src/`, `tests/`, the guides, `AGENTS.md`, `Cargo.toml`).
3. **`explain` reads `bytes[26]` and a range from `27 + segments`**: both go through `get`, a cut file returns `None` (asserted: `explain(b"OggS")`), and nothing the sniff finds can change a verdict — it only changes a sentence.
4. **The `DECODED` record is `cfg(test)` only** and the unit test needs the dev-dependency `bevy` with `wav` (always true for `cargo test`); it does not read `tests/fixtures`, so a packaged crate's tests do not break on a path that is not shipped.
5. **`cannot_play` is asked twice per refused play** (once in `answer`, once for the sentence): two memo hits, 21 ns each, on a path that has just refused something.
6. **Sentences corrected on the read**: `cues.md` and the CHANGELOG said *"once per asset"* before the memo was shared (now true, and held by a test); iteration 116's CHANGELOG entry still said *"bytes that load and do not decode still panic Bevy's own path"* — it now says *fixed in the entry above*; `Cargo.toml`'s comment said the backend *"warns about rather than panicking on"* — true of the mixer, false of Bevy's path; `MissingSound`'s doc named only the mixer's place of discovery.
7. **`Refused.reason` still prints *"no handle or undecodable bytes for sound `x`"***, which a game printing the message reads for an LFS pointer. Left on purpose: `tests/play.rs:295` pins the wording and a variant is a public change (`needs-decision/undecodable-sound.md`, whose iteration-117 note adds that a variant could carry the hint). The picture shows the log line, which has the cause.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*Not the item's own cases. What would a game that did not read this iteration's notes meet?*

**Critique, first reading:**

1. **A cue of three recordings, one an LFS pointer.** Refused whenever that variant is picked, as a key with no handle already was. By reading, not run.
2. **A looping music layer whose stem is an Opus file**: refused at the play; the layer is a virtual voice, and `MusicState::blocked` cannot say *why* — the log can (iteration 116's note in `music.md` covers a dead file; an undecodable one is the same state). Not run.
3. **A game with `mixer` and a device**: the crate's mixer decodes in `start_pending` and answers as it always did; six existing tests with placeholder bytes behind a fake `Output` hold that, and one mutation of mine proves they do. **A game with `Backend(true)` and no `Output`** is on Bevy's path and is asked (the predicate is the one `spawn_voice` uses).
4. **Hot reload, good to bad**: the next play is refused (tested, including a corruption that keeps the length and the first 16 bytes); a voice already playing keeps playing — Bevy decoded it before the edit. **Bad to good**: the next play sounds.
5. **An MP3 in a default-feature game**: refused where `rodio` has no decoder for it (the property test, in the default build), played where it has one (`mixer`, `semantic`, or Bevy's own `mp3`). The asset loader would not have loaded it in the first place (iteration 116's path), unless the game put the bytes in `Assets` itself.
6. **The first-sight price**: 0.8 ms for a Vorbis, once per sound, on top of Bevy's own decode. Twenty distinct Oggs in one frame would be about 16 ms by arithmetic; **not measured**, nor is a debug build. The plan says what would move the check (to asset arrival, or off-thread).
7. **Still carried**: the crate's own mixer answers `Played` then `Refused` where Bevy's path now refuses at the play (an `AudioOutput` hook, public, carved off); a load that never resolves still waits; a `Play::on` for a vanished owner leaves an ownerless loop (**read in Bevy's relationship code, not run**).
8. **The Cauldron's audio panel was not run** (the editor pins `82d227e`, 221 commits back, and a 14 GB target on a disk with 5 GB free); no API the panel uses changed. **Nobody listened — there is nothing to hear**: a refusal makes no sound, and the render's WAVs are byte-identical.

**Verdict: GREAT.** The limits are recorded, with their routes, in `docs/loop/log.md`.

## Prediction scorecard

In `research.md`: **P3 wrong by 2.5×, P2 and P5 half-wrong, P1, P4 and P6 right as written.** The finding the table did not contain — that one memo per system built a decoder twice — is change 1 above.

## Tool discipline, as asked

The standing instruction is to read with `read` and `grep` and use bash for programs. Every file read and every search in this pass went through `read`, `grep` or `glob`; every edit through `edit` or `write`. Bash ran `cargo` (build, test, clippy, fmt, doc, the matrix, the probe and the benchmark), `ffmpeg`, `git`, `python3` for the transcript picture, and `cp`/`rm` of my own scratch.
**The avoidable ones:** `grep` pipelines after `cargo test` to keep only the result lines (short, but a pipeline where an artifact would do), and one `git status --short | head -30`. I did not count the calls and will not give a number.
