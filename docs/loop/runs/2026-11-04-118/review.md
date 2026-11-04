# Iteration 118 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest pass it could affect.

## Pass 1 — Function

*Against the item's own acceptance — bytes the crate's own mixer cannot decode are refused at the play (no `Played`, no voice, no slot), the log carries the decoder's words and the hint, a game that hangs data on `Played::entity` does not panic, the question is asked once per buffer and the verdict is the asker's own — and against the cue table: 39 cases (38 held, one new), none red; the 21 rendered WAVs are byte-identical to iteration 117's (21 of 21).*

**Critique, first reading — things that were wrong in my own work:**

1. **My first statement of the invariant was too strong, and the test that held it was red for behaviour that is intended.** *"A `Played` entity exists when the frame ends"* failed on two scenarios I added after the first run: a voice stolen by a later request **in the frame it was announced**, and one stopped by a `StopCue` in it. `spawn_voice` gives every voice `Applied { db: SILENT_DB }`, `update_mix` skips a voice that is `Stopping`, and `apply_stopping` despawns a voice whose current level is already silent (read in `src/mix.rs`; the despawn itself is what the test measured) — so a voice that never rose is gone in the frame it was stolen, it never sounded, and the log says `stopped … stolen by …`. I did not predict it (P5 below). The promise was narrowed to what the crate can keep — **each play is answered once, and a `Played` entity that is gone at the end of its frame is explained by a `stopped` line** — the exception is written where a game reads it (`Played::entity`'s doc: `get_entity`, or `try_insert`), and the mechanism in `AGENTS.md`. I did not change `apply_stopping`: keeping the voice a frame longer would, **by reading** `spawn_voice`'s `PlaybackSettings` and Bevy's start system (not run), let Bevy's path start its `AudioPlayer` in `PostUpdate` at the play's full level — an audible blip for a sound that was refused its slot.
2. **The first edit of `Refusal::MissingSound`'s doc cut half a sentence and left a doubled word; the first `AGENTS.md` edit left a duplicated line.** Both were visible in the tool's echo of the lines and fixed at once; both are the kind a build does not catch. (A `cargo doc -D warnings` and a read of the echo did.)
3. **One explanation in an evidence file was written as fact.** `tag-refused-after.txt` said the probe's `played = []` was *"the PostUpdate tally the panic had cut short"*; I had not checked it. It is now marked `[INFERENCE]`.
4. **The `before` of the real backend was not run.** The unchanged tree was exercised with a fake output whose `play` answers `None`; the real `MixerBackend` full-stack run is an *after* only. The premise that the real `play` returns `None` for the five bad files is held by `the_real_mixers_decode_error_agrees_with_its_play`, whose `play` column is code this change did not touch. Said in `evidence/after/real-mixer-undecodable-files.txt`.
5. **Two predictions were wrong or half-wrong** (`research.md`; scorecard below). The probe that found the defect preceded them, as in iteration 116, and the research says so.
6. **The benchmark I had was blind to this change.** `perfcheck` ran 0.018 – 0.068 / 0.112 – 0.443 / 0.406 – 1.288 µs across three runs of one binary on a shared machine — a three-fold spread over a function this change does not call. The added work was timed directly instead (a memoised hit 20.7 ns; a first sight 0.63 – 0.66 ms for a Vorbis).
7. **A real defect was found and not fixed**, twice more: the quick start does not play a sound as written, and `Play::on` for a vanished owner leaves an ownerless loop. Both are in `eval.md` with their evidence and ranked in `plan.md`; this pass shipped one item.

**What passed.** The probe on the real stack — the crate's own mixer, a real device, the real asset server, fourteen real files, a game system tagging every `Played::entity` — **8 `Played`, 8 live voices, 6 `Refused` at the play** (the same frame as the plays beside them), the decoder's words and the hint in the log, 140 frames without a panic (`evidence/after/real-mixer-undecodable-files.txt`); the same probe with a fake that refuses and does *not* say so still panics, which is the documented limitation, and with one that says so survives (`tag-refused-after.txt`). **Bevy's decoder builder and the mixer's `Decoder::new` agreed on 14 of 14 files**, words included (`decoder-agreement-and-cost.txt`). The five guards were switched off in turn (below) and each reddened the tests that should notice it. Final matrix (`matrix-final.log`): default **217** (was 213), serde 226, semantic 535, mixer 232 + 20 lib, all-features 563 + 129 lib, doc 6, readme 1, clippy, fmt, rustdoc clean; the one warning is the known `example "mixer"` linker one. Render: 21 of 21 WAVs byte-identical to iteration 117's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix.png` read (an onset in the first 40 ms and three steady low lines; no vertical line, hole or shelf after it).

**Mutations** (each applied to the shipped code, the four new tests run, then restored):

| switch | red |
|---|---|
| M1 the hook ignored (Bevy's decoder always asked) | 3 of 4: the invariant test (the mixer scenario starts nothing), *asked once*, *the verdict is the asker's* |
| M2 the backend's answer ignored (always decodable) | 3 of 4, **including the panic** (`handler.rs:128`) and *one answer per play, got 1 played and 1 refused* (`evidence/before/one-answer-tests-with-the-backend-never-asked.txt`) |
| M3 one memo for both askers | exactly *the verdict is the asker's own* |
| M4 no memo for the backend | exactly *asked once however often it is played* |
| M5 `MixerBackend::decode_error` always `None` | `the_real_mixers_decode_error_agrees_with_its_play`: *an Ogg Opus: decode_error said None and play returned None* |

**Verdict: GREAT.** After the fixes above the item's acceptance is met on the crate's own mixer with a real device, and its guards are mutation-checked.

## Pass 2 — Zoom

*The source diff, line by line, as a reviewer who did not write it — `src/output.rs`, `src/mixer/mod.rs`, `src/sounds.rs`, `src/play.rs`, `src/playback.rs`, `tests/one_answer.rs`, `tests/mixer.rs` — and every sentence I wrote about it.*

**Critique, first reading:**

1. **A public trait gained a method.** Defaulted (`None`), so a fake output compiles unchanged and nothing in the repository was edited to accommodate it — **no existing test's code or assertion changed** (the doc comment of one, `a_sample_the_decoder_refuses_names_its_sound_key`, now says which path it exercises). Additive, recorded in `needs-decision/audio-output-decode-error.md` with its options and its limitation: **an output whose `play` can refuse and which does not override the method keeps the old two answers**, and the probe shows it still panics the tagging game. That is a contract on the implementor, said in the trait's doc, in `AGENTS.md` and in the changelog; the trait is documented as one a game never implements.
2. **`SirenSounds`'s private memo became a pair of maps** under the same `Mutex`: `Debug`, `Clone` and every public item are as they were. `Decode<'a>` is `pub(crate)` and does not appear in any public signature (`play_requests` gained no parameter and is still at Bevy's sixteen; `answer` is private; clippy's `private_interfaces` is clean).
3. **`verdict_with`'s signature did not change**, so the two memo unit tests from iteration 117 are untouched.
4. **`MixerBackend::decode_error` does not copy the file** (`Cursor` over the shared `Arc`); `play` still does (`to_vec()`, once per play). Not changed here: that is a cost of the loop path, listed in the log.
5. **The mixer's check is a second decoder construction.** Vorbis 0.63 – 0.66 ms, once per sound, on top of the one `play` makes; MP3 10 µs; FLAC 22 µs; WAV 7 – 21 µs; a refusal 0.7 – 3.6 µs. Twenty distinct Oggs first played in one frame would cost about 13 ms by arithmetic — **not measured**; nor is a debug build.
6. **`Decode::Skip` exists for exactly one caller**: the in-flight system's voice that has no `AudioPlayer` (the mixer's, still waiting for its bytes), which `start_pending` answers for. It returns before touching a memo (5.1 ns).
7. **Docs that said the old thing and were corrected on the read** (found by grepping the old answer, across `README.md`, `docs/guide/`, `docs/research/`, `AGENTS.md`, `Cargo.toml`, the doc comments in `src/`, `tests/` and `examples/`): `docs/guide/cues.md`'s *"the crate's own mixer decodes in `start_pending` and answers as it always did"*, `Cargo.toml`'s comment on the `rodio` line, `AGENTS.md`'s *"Only Bevy's path is asked"*, the `Refusal::MissingSound` doc, `start_pending`'s doc, and iteration 117's own CHANGELOG entry (annotated *"until iteration 118, the entry above"* rather than rewritten). R6 in `docs/research/siren.md` gained its contract sources and its two proofs.
8. **The README's and the crate root's *"both answer … identically"*** is now true for bytes a decoder refuses; it is not edited, and the README's quick start (a different defect) still does not play a sound as written — carried to the next pass with its evidence.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*Not the item's own cases. What would a game that did not read this iteration's notes meet?*

**Critique, first reading:**

1. **A game with `mixer` and a device and one corrupt file in its library**: the first play of it is refused with the decoder's words and a hint; every other sound plays; no panic in any system that tags the entity. Run (fourteen files).
2. **A game whose bytes arrive after the play** (a load in flight): still `Played`, and refused in the frame the bytes arrive — a second answer, documented on `Played`, `start_pending` and in the guide; a game that keeps the `Entity` past the frame uses `get_entity` or `try_insert`. Not new in kind: iteration 116 and 117 did this for Bevy's path.
3. **A game that tags `Played::entity` in a system that runs after `SirenSystems::Mix`**, for a voice stolen in the frame it was announced, can still meet a despawned entity: the doc says so and says what to use. **Not run**; the unit of the guarantee is the log line, not the entity.
4. **A looping music layer whose stem is an Opus file**: refused at the play on the mixer now (it was `Played`, then `Refused` in the same frame). `MusicState::blocked` cannot say *why*; the log can. Read, not run.
5. **A hot reload, good to bad**: the next play is refused (the memo is keyed by buffer address, length and the first 16 bytes; `an_edit_to_the_file_is_asked_again` is the Bevy-path test, and `a_sound_is_asked_of_the_mixer_once_…` replaces the asset and sees the mixer asked again once). A voice already playing keeps playing. **Bad to good**: the next play sounds.
6. **A game with an `Output` of its own** (the trait says a game never implements one): inherits `None`, as before, and is answered by `start_pending` as before.
7. **The README's quick start does not play a sound as written** (keys without an extension, `walk` and `hear` never scheduled). Found by the same probe, **not fixed**, first in the next pass. It fails loudly — every play is refused with the file the asset server looked for — which is what 116 bought.
8. **Nobody listened, and there is nothing new to hear**: a refusal makes no sound and the render's WAVs are byte-identical. **The Cauldron's audio panel was not run**: the editor pins `82d227e`, 221 commits back, with a 14 GB target on a disk with 4.7 GB free; no API the panel uses changed (the one trait change is additive and the panel does not implement it).

**Verdict: GREAT.**

## Prediction scorecard (wrong ones first)

| # | prediction (written before the fix) | result |
|---|---|---|
| P5 | the invariant test (*`Played` ⇒ the entity exists at the end of the frame*) fails on the unchanged tree **for exactly the mixer-refused scenario** and for no other | **wrong.** It failed for the refused scenario — and, once I added them, for two more that are *intended* behaviour (a voice stolen or stopped in the frame it was announced). The invariant was narrowed; the exception is documented. |
| P4 | no existing test is edited | **half-wrong**: no test's code or assertion changed; the doc comment of one did, and `tests/all.rs` gained a `mod` line |
| P1 | with a truthful fake: 0 `Played`, one `Refused { MissingSound }` at the play, 0 voices, no panic with the tagger | held (`tag-refused-after.txt`; `tests/one_answer.rs`) |
| P2 | the real backend's `decode_error` agrees with its `play` on every probe file; **and** Bevy's builder agrees with the mixer's `Decoder::new` (65 %) | held, both: 10 of 10 on the real backend; **14 of 14** between the two decoders (the 65 % was uncertainty, not a miss) |
| P3 | the memoised check ≤ 30 ns; a first sight costs one decoder construction (a Vorbis 0.6 – 0.8 ms) | held: 20.7 ns; 0.63 – 0.66 ms |

## Tool discipline

The user's standing instruction: read with `read`/`grep`, not bash. This pass read every file with `read` or `grep` (including the dependency sources, whose line numbers I cited from earlier reads), searched with `grep` and `glob`, and used bash for **programs**: `cargo` (build, test, clippy, fmt, doc, run), `ffmpeg` (probe files, loudness, the spectrogram), `python3` (the repository's transcript renderer), `git`, and `cp`/`rm` of files this pass made or of artifacts. Evidence that the tool kept is `artifact://…` copied with `cp`; the matrix's output arrived with the `wait` and was written with `write`. After each `cargo fmt` I re-read before the next `edit`. **The avoidable ones**: `grep -E` pipelines after `cargo test` to keep only the result lines — short, but a pipeline where an artifact would have done. I did not count the calls and will not give a number.
