# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Entries are **newest first**; iteration 0 is the repository's creation. Rules and traps are in
`AGENTS.md`; the design rules and their sources are in `../research/siren.md`; the fixed cases every iteration runs are
in [`cues.yaml`](cues.yaml); each iteration's own research, plan, evidence and three-pass review are in
[`runs/YYYY-MM-DD-NN/`](runs/README.md), and a question the loop will not decide itself goes to
[`needs-decision/`](needs-decision/README.md).

## Iteration 119 — 2026-11-04 — `Play::on` made the voice a child of its owner, and a child is the wrong tool: a panic, a sound at the wrong place and level, a loop that never ended, and a false log line

**Change.** `spawn_voice` inserted `ChildOf(owner)` to say *"the sound belongs to `entity`"*, and a relationship is three things at once — a lifetime, a transform and a hook. Run as a game would run it (`runs/2026-11-04-119/evidence/before/`), the states an owner can be in gave four defects from that one line: **(1) a panic** in Bevy's command handler (`handler.rs:128`, *"Entity not yet spawned"*) for an owner that is allocated and not yet spawned — the state every `Commands::spawn` returns before its command applies — because the hook queues a command on an entity that does not exist (an unordered system reproduces it, and `world.entity_allocator().alloc()` does deterministically): a game that spawns a projectile and plays its sound in one system crashed or not by an arrangement it does not control; **(2) a sound at *owner ∘ at*** — (200, 0, 0) for an owner at (100, 0, 0), 12.04 dB under the crate's law, and for an owner with no `Transform` (Bevy never propagates a child of a parent that has none) the listener's own position, where the crate's +80.00 dB panner compensation stands and **+52.04 dB, 400× the sample**, is written to the sink; **(3) a loop that lives for ever** on an owner already gone; **(4) a false `owner despawned` line** for a voice the *game* despawned.
Now a voice carries **`VoiceOwner(Entity)`** — data, not a relationship (additive; `needs-decision/play-on-owner.md`) — and **`owner::end_and_follow`**, straight after `play_requests`, reads the owner with `Entities::get_spawned`: *alive* keeps the voice, *pending* (allocated, its spawn command queued) is waited for, *gone* writes `stopped: owner despawned` and despawns it — in the frame it was played if the owner was already gone, **before `start_pending` can start it on the crate's mixer** (the output is asked to play 0 voices; it was 1). A play on a gone owner **takes nothing from anyone** (no steal, no slot, no duck: `doomed` in `play_requests`). **`at` stays a world position**, so the crate's law, its compensation and the panner see one place, and a placed voice on Bevy's path **follows its owner** (its offset in the owner's space, fixed the first frame the owner's `GlobalTransform` has been propagated; a lag of at most one frame). **Removed**: `mix::log_despawned_voices` (a `pub fn` whose only reading of *owner* was `ChildOf`) and the unused `ChildOf` element of `mix::EndedVoice`.

**Rationale.** Iteration 118 ran the documented use of a handle the crate *hands out*; this ran the other direction — the documented ways a game *hands the crate* an entity (`Play::on`, `Play::at`, and `Play`'s own example `.on(foot).at(..)`) — across the states that entity can be in (spawned, allocated-not-spawned, gone, no `Transform`, at the place asked for). A crash on a documented use, a sound up to 400× too loud and a leak, on the headline use of `on`, outranks the runner-up (the README's quick start, a loud failure since 116) and **absorbs 118's third carry-over** (a vanished owner) while finding three more defects of the same cause. Sources: Ousterhout, *A Philosophy of Software Design*, 2nd ed., §10.3 (*define errors out of existence* — an owner not spawned yet has not begun, one that is gone is already over); Hunt and Thomas, *The Pragmatic Programmer*, 20th ed., tip 38 (*crash early* is for what was supposed to be impossible, and a pending owner is the ordinary state of every entity in the frame it is created); Gregory, *Game Engine Architecture*, 3rd ed., §14.4.2 – 14.4.3 (a source has one position, for level and for pan); Zotkin, Duraiswami and Davis, IEEE Trans. Multimedia 2004, DOI 10.1109/tmm.2004.827516 (a sound ceases when its object is destroyed and tracks its position); `bevy_ecs-0.20.1` `entity/mod.rs:846-862` and `relationship/mod.rs:194-210`, `bevy_transform-0.20.1` `systems.rs:334`, `bevy_audio-0.20.1` `audio_output.rs:128-133`, `rodio-0.22.2` `source/spatial.rs:63`.

**Measured.** The probe, on the unchanged tree and the final one: no panic in either arrangement; the voice at **(100, 0, 0)** for an owner at the place **and** for one with no `Transform` (it was (200, 0, 0) and (0, 0, 0)); 0 voices for a gone owner (it was 1, for ever); no false line (`evidence/after/owner-probe-after.txt`, `docs/media/owner.png`). `owner::end_and_follow` costs **0.05 µs** a frame with 256 voices none owned and **1.5 – 1.6 µs** with 256 owned, placed ones (about 6 ns a voice; `evidence/after/owner-system-cost.txt`); `perfcheck` reads 0.057 – 0.062 / 0.304 – 0.359 / 1.053 – 1.225 µs, inside the band, as it should for a system it does not call. **`tests/owner.rs` (9): the first six are red on the unchanged tree**; six guards each switched off in turn reddened their own test (and M2 an existing one), and **the ninth** — a placed sound on an owner spawned in the same frame — **caught a bug in my first form of the follow** (`evidence/before/mutations.txt`).

**Predictions** (`research.md`, scorecard, wrong ones first): **two gaps, both found by tests** — a play on a gone owner would steal a playing voice, and the follow would measure against an unpropagated `GlobalTransform`; **P6 half-checked** (the lag is by design; the test waits three frames); **P3 held by an oracle that shares the model's panner**; **P5 held for code, four comments in two test files were edited**; P1, P2, P4, P7, P8, P9 right.

**The catches, because it was wrong first.** *The follow's first form measured the offset against an owner's default `GlobalTransform`* — the sound jumped to (200, 0, 0) a frame after a same-frame spawn; the first full matrix was green because no test built that arrangement; the ninth test was red, `Ref<GlobalTransform>::is_added()` now waits, and the matrix was run again. *The first design let a play on a gone owner steal a playing voice* — the red run on the unchanged tree already printed `stolen by engine (cue)` and I read past it. *Two slips in my own evidence* (a mis-transcribed probe block; "52 dB too loud" for what is +52 dB written to the sink and 80 dB over the law). *The disk reached 0 GB* and a linker failure looked like code for one command; `cargo clean -p bevy_siren` (this crate's, made this session) freed 6.8 GiB. *A level assertion that is partly circular* (it uses the model's mirror of `rodio`'s panner). All in `review.md`.

**Evidence.** [`runs/2026-11-04-119/`](runs/2026-11-04-119/) — `eval.md` (the claims audit: five rows, four defects found by running), `research.md` (six sources, nine predictions and their scorecard), `plan.md` (the ranking, the design, and what happened to it), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix-final.log` (and `matrix-final-first.log`, before the ninth test); `evidence/before/` (the probe on the unchanged tree, the six tests red on it, the mutations); `evidence/after/` (the probe after and its source, the system's cost, perfcheck, the render set); `evidence/spectrograms/`.

**Picture**: `docs/media/owner.png` (a transcript — a voice at the wrong place and a panic before, the right place and no panic after; nothing to hear). `cues.yaml` case **40**; `docs/guide/{buses,spatial}.md` (a new section *A sound that belongs to an entity*), the README's *place a sound* line, `Play`/`Played::entity`/`VoiceOwner` docs, `AGENTS.md` (the layout line and the trap *"`Play::on` made the voice a child (`ChildOf`), and a child is the wrong tool"*); the skill gained the lesson *a relationship is a lifetime, a transform and a hook at once — grep for `ChildOf`, and ask of each use which of the three it wants*.

**Baseline:** green on arrival — 9 + 217 / 9 + 226 / 118 + 535 / 20 + 232 / 129 + 563, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **9 + 226 / 9 + 235 / 118 + 544 / 20 + 241 / 129 + 572**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways; the only warning is the `example "mixer"` linker one. **No existing test's code or assertion changed** (five that read an owner's or a voice's despawn pass as they were; four comments that said *parented* were corrected). **The public surface**: `+ VoiceOwner`, `− mix::log_despawned_voices`, `mix::EndedVoice` loses its last element. **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit; `bevy_grimoire`'s crates contain no `Play::cue(..).on(..)` and no `ChildOf` beside a `Voice` (`Children` was not searched). **Not listened to** — there is nothing to hear: the render's 21 WAVs are byte-identical to 118's.

**Verdicts.** Function **GREAT** (after seven catches), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `1be4e0a`.

**Recorded, not fixed.** **A spatial cue played `.on(owner)` with no `.at(..)` is still flat** (`tests/play.rs::a_spatial_cue_without_a_position_is_still_flat`), so the most natural use of `on` — an engine on a vehicle — is not placed; and **the compensation of a moving emitter is fixed at the play** (the pan tracks, the level law does not). **A sound on an owner that is already gone now ends**, where it used to play: a one-shot on a bullet despawned in the same frame no longer sounds, and the log says `stopped: owner despawned` (`needs-decision/play-on-owner.md`, option 3, weighs a refusal; a new `Refusal` variant breaks every exhaustive match). **A game system after `Play` that hangs data on a `Played::entity` with `insert` panics for a play on a gone owner** (the despawn is flushed inside the `Play` set; `Played::entity`'s doc says `get_entity` or `try_insert`). **A pending owner is waited for with no bound.** **A placed owned voice does not follow on the crate's mixer** (no panner). **No device was involved**: the heard level is computed from the model's mirror of `rodio`'s formula, not captured. **The README's quick start still does not play a sound as written** (iteration 118's evidence, unchanged).

**Top 5 remaining.**

1. **Make the front door play a sound, and test that it does** — the README and crate-root quick start with keys that name files, the systems scheduled, the refusals shown; `docs/guide/cues.md:147-149` made to say what `load_table` does; and `tests/readme.rs` made to **run** the snippet against real fixtures instead of comparing text. Evidence: `runs/2026-11-04-118/evidence/before/front-door-probe-on-the-unchanged-tree.txt`, probe source in `runs/2026-11-04-118/evidence/after/front-door.rs.txt`.
2. **A spatial sound that belongs to a moving entity should be placed at it, and its level should follow it** — `.on(owner)` of a spatial cue with no `.at(..)` placed at the owner (today it is flat), and the distance law tracking the owner's movement (the compensation is fixed at the play, so only the pan follows). The mechanism is in place (`VoiceOwner`, `Follows`); the design question is that `update_mix` is the one writer of a voice's gain, so the follow would write a *target* it reads. Measure first: how many dB off is a 20 m/s emitter after a second?
3. **The layers of a score start together** — a barrier in `music::start_missing` that holds the batch while any stem is `Loading` (a failed or undecodable one does not hold it), with a bound on the wait that has a source and an additive accessor on `MusicState`; the design is in `runs/2026-11-04-116/plan.md`. Measured at 3 of 100 warm-cache starts of eight stems.
4. **A person listens** — a looped Ogg stem coming in on the bar, the wrap that was a 12 ms hole, a fade, a FLAC and an MP3 loop, **a sound on a moving owner** — on Bevy's path and on `mixer`; and a capture of the device's output if one can be made (it would also settle the heard level of a placed sound).
5. **Waiting on a person, another repository or hardware, and the owner's decisions:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; **`needs-decision/play-on-owner.md`** (a refusal for a play on a gone owner; `VoiceOwner` and the removal of `log_despawned_voices`) and **`undecodable-sound.md`** (the `Refusal` variants; a `SirenSounds::undecodable` for the editor's panel); `AudioOutput::decode_error` (additive, recorded); the default-floor decision and a hubness correction measured with a held-out bank; memory and copies on the loop path (`play`'s `bytes.to_vec()`); iteration 115's one-time Ogg-loop probe for a game with Bevy's `symphonia-vorbis` on; a third library; the CI gap (nothing runs the benchmark; the mixer's device tests return early with no audio device); an index stamp that does not name the decoder; an MP3 with no tag.

## Iteration 118 — 2026-11-04 — on the crate's own mixer, `Played` then `Refused` in one frame, the entity despawned — and a game that used `Played::entity` as documented panicked

**Change.** On the crate's own mixer, `play_requests` wrote `Played` and spawned the voice, and `start_pending` — the next system in the chain — decoded the bytes. When `rodio` refused them (a Git LFS pointer, an empty or cut file, an Ogg Opus, an ADPCM WAV) the entity was **despawned in the frame it was announced** and a `Refused` was the play's second answer. A game that hangs data on `Played::entity` — the use that field's doc names — **panicked in `bevy_ecs`'s command error handler** (`bevy_ecs-0.20.1/src/error/handler.rs:128`, *"Entity despawned: The entity with ID 45v0 is invalid"*), naming its own system and not the file. The README, the crate docs and `AGENTS.md` say *both backends answer `Played`, `Refused` and `Voice` identically*; Bevy's path had refused such bytes at the play since 117. Found by a probe that did what the doc says a game does.
Now: **(1)** `AudioOutput` gains `decode_error(&self, &Arc<[u8]>) -> Option<String>`, a **defaulted** method (`None`, so a fake output is unaffected; additive, `needs-decision/audio-output-decode-error.md`); `MixerBackend` overrides it with the construction its `play` makes, over a `Cursor` on the shared `Arc` — without `play`'s copy of the file; **(2)** `sounds::Decode { Skip, Bevy, Output }` says who decodes, and `cannot_play` takes it instead of a bool: `play_requests` asks the backend that owns the voice, at the play, once per buffer; **(3)** `SirenSounds` keeps **one memo per asker**, so a verdict is the asker's own; **(4)** a play whose bytes are loaded and refused is `Refused { MissingSound }` at the play — no `Played`, no voice, no slot — with the decoder's words and the hint in the log, and **no new `Refusal` variant**; a voice spawned while its bytes were loading is refused in the frame they arrive, as before; **(5)** `Played::entity`'s doc says what a game may rely on: it exists when the message is written, and **a voice stolen by a later request in the frame it was announced, or stopped by a `StopCue` in it, never sounded and is despawned in that frame** (`spawn_voice` gives it `Applied { db: SILENT_DB }`, `update_mix` skips a stopping voice, `apply_stopping` despawns a silent one), with a `stopped` line in the log — so a game that keeps the `Entity` uses `get_entity` or `try_insert`.

**Rationale.** Iteration 117 asked *which sentence is held only by a fake*; this asked it of the **front door** — *which documented use of a returned handle has any test run it as a game would?* — and ran each. A crash on a documented use, in a game's own system, naming no file, with the crate's own rule (*every play answers exactly one of `Played`/`Refused`*) and three sentences false, outranks the runner-up (the README's quick start does not play a sound as written, a loud failure since 116) and the third (a vanished owner's ownerless loop, a silent leak that needs a rarer order of events and whose safe fix is larger). Sources: Hunt and Thomas, *The Pragmatic Programmer* 20th ed., tip 37 (postconditions are the contract; failing one is a bug); Gregory, *Game Engine Architecture* 3rd ed., §16.5.3 (a handle can go stale — a system that hands one out must not hand out one it has destroyed); Barr et al., *The Oracle Problem in Software Testing*, DOI 10.1109/tse.2014.2372785, §4.1 (an invariant over scenarios is a partial oracle); `bevy_ecs-0.20.1` `error/handler.rs:128` and `system/commands/mod.rs:1411` (`insert` on a despawned entity panics by default; `try_insert` is silent). `docs/research/siren.md` R6 carries them.

**Measured.** On the real mixer, a real device and the real asset server — fourteen real files, one play of each, a game system tagging every `Played::entity`: **8 `Played`, 8 live voices, 6 `Refused` at the play, the decoder's words and the hint in the log, 140 frames, no panic** (`evidence/after/real-mixer-undecodable-files.txt`, `docs/media/one-answer.png`). The original probe with an output that says beforehand **survives**; with one that does not (`decode_error` left at its default) it **still panics** — the documented contract on an implementor, in the trait's doc, `AGENTS.md` and the changelog. **Bevy's decoder builder and the mixer's `Decoder::new` agreed on 14 of 14 files**, words included; the real backend's `decode_error` agrees with its own `play` on ten kinds of file. Cost: a first sight on the mixer path is one decoder construction *in addition to* `play`'s — Vorbis **0.63 – 0.66 ms**, MP3 10 µs, FLAC 22 µs, WAV 7 – 21 µs, a refusal 0.7 – 3.6 µs — and a memoised hit **20.7 ns**. Five guards switched off in turn (the hook ignored; the backend's answer ignored, which brings the panic back; one memo for two askers; no memo; the real check always `None`) each reddened the tests that should notice it and, for the two single-purpose ones, only those (`review.md`).

**Predictions** (`research.md`, scorecard, wrong ones first): **P5 wrong** — the invariant test *"a `Played` entity exists when the frame ends"* failed on the refused scenario and, once I added them, on two that are **intended** (a voice stolen or stopped in the frame it was announced), so the promise was narrowed to *each play is answered once, and a vanished voice is explained by a `stopped` line*; **P4 half-wrong** (no test's code changed, one doc comment did); P1, P2 (including the 65 % one: the two decoders agree), P3 right.

**The catches, because it was wrong first.** *The first invariant was too strong* (above). *Two doc edits left a cut sentence and a duplicated line*, visible in the tool's echo. *One explanation in an evidence file was written as fact* and is now marked `[INFERENCE]`. *The real backend's before-run was not made* (the premise is held by a test whose `play` column is untouched code). *`perfcheck` cannot see this change* (a three-fold spread between runs of one binary), so the added work was timed directly. All in `review.md`.

**Evidence.** [`runs/2026-11-04-118/`](runs/2026-11-04-118/) — `eval.md` (the claims audit: three defects found by running, five rows), `research.md` (seven sources, five predictions and their scorecard), `plan.md` (the ranking and what was carved off), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix-final.log`; `evidence/before/` (the probe on the unchanged tree; the tests red on it; the tests with the backend never asked); `evidence/after/` (the real-mixer run, the original probe after, the agreement and cost table, the probe's source, perfcheck, the render set); `evidence/spectrograms/`.

**Picture**: `docs/media/one-answer.png` (a transcript — a panic before, refusals after; nothing to hear). `cues.yaml` case **39**; `docs/guide/cues.md`, `Cargo.toml`'s `rodio` comment, `docs/research/siren.md` R6 and iteration 117's CHANGELOG entry corrected where they said the mixer *answers as it always did*; `AGENTS.md` gained the layout lines and the trap *"A `Played` followed in the same frame by a `Refused` and a despawn crashes the game that hangs data on `Played::entity`"*; the skill gained the lesson *run the documented use of a handle the crate hands out, across the scenarios that end the thing it names, and state the invariant you can honestly promise*.

**Baseline:** green on arrival — 9 + 213 / 9 + 222 / 118 + 531 / 20 + 227 / 129 + 558, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **9 + 217 / 9 + 226 / 118 + 535 / 20 + 232 / 129 + 563**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways; the only warning is the `example "mixer"` linker one. **No existing test's code changed.** **One public trait gained a defaulted method** (`AudioOutput::decode_error`); `SirenSounds`'s private memo became a pair of maps. **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit; no API it uses changed. **Not listened to** — there is nothing to hear: the render's 21 WAVs are byte-identical to 117's.

**Verdicts.** Function **GREAT** (after seven catches), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `b0e8ed3`.

**Recorded, not fixed.** **The README's and the crate root's quick start does not play a sound as written** — keys `step_01`/`step_02` and `load_table(.., "audio")` ask for `audio/step_01` with no extension, so every play is refused (with the file the server looked for, since 116); with `.ogg` in the keys 5 of 5 play; `walk` and `hear` are never scheduled; `tests/readme.rs` holds text equality over a `no_run`; `docs/guide/cues.md:147-149` says a key `step_03` is `audio/step_03.wav`. **`Play::on` for an owner that is already gone** leaves a voice with no `ChildOf` (Bevy warns and removes the relationship), and a `Mode::Loop` voice lives for ever on both backends, counted on its bus. **A voice stolen in the frame it was announced is announced `Played`** and never sounds (documented on `Played::entity`; whether it should be announced at all is a design question). **A backend whose `play` can refuse and that does not override `decode_error` keeps the two answers** (a contract on an implementor, which the trait says a game never is). **Console `warn!` output for a refused sound was decided against**: `src/log.rs:7` — *"A crate that prints is a crate a game cannot silence."* **`play`'s `bytes.to_vec()`** still copies the file on every play (the check does not). A first sight of twenty distinct Oggs in one frame costs about 13 ms by arithmetic — **not measured**, nor a debug build.

**Top 5 remaining.**

1. **Make the front door play a sound, and test that it does** — the README and crate-root quick start with keys that name files, the systems scheduled, the refusals shown; `docs/guide/cues.md:147-149` made to say what `load_table` does; and `tests/readme.rs` made to **run** the snippet against real fixtures instead of comparing text. Evidence: `runs/2026-11-04-118/evidence/before/front-door-probe-on-the-unchanged-tree.txt`, probe source in `evidence/after/front-door.rs.txt`.
2. **`Play::on` for an owner that does not exist** — a crate-private `Owner(Entity)` component and a system that stops a voice carrying `Owner` and no `ChildOf`, with the log saying so. **Not** an existence check at the play: an entity spawned in the same frame through deferred commands looks gone, and valid plays would be refused. Both backends; the control (a live owner) already logs *owner despawned*.
3. **The layers of a score start together** — a barrier in `music::start_missing` that holds the batch while any stem is `Loading` (a failed or undecodable one does not hold it), with a bound on the wait that has a source and an additive accessor on `MusicState`; the design is in `runs/2026-11-04-116/plan.md`. Measured at 3 of 100 warm-cache starts of eight stems.
4. **A person listens** — a looped Ogg stem coming in on the bar, the wrap that was a 12 ms hole, a fade, a FLAC and an MP3 loop — on Bevy's path and on `mixer`; and a capture of the device's output if one can be made.
5. **Waiting on a person, another repository or hardware, and the owner's decisions:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; **`needs-decision/undecodable-sound.md`** (the `Refusal` variants — both backends now refuse at the play, so a variant that carries the hint is cheaper than it was; and a `SirenSounds::undecodable` beside `missing` and `failed` for the editor's panel); `AudioOutput::decode_error` (additive, recorded); the default-floor decision and a hubness correction measured with a held-out bank; memory and copies on the loop path (`play`'s `bytes.to_vec()`); iteration 115's one-time Ogg-loop probe for a game with Bevy's `symphonia-vorbis` on; a third library; the CI gap (nothing runs the benchmark; the mixer's device tests return early with no audio device); an index stamp that does not name the decoder; an MP3 with no tag.

## Iteration 117 — 2026-11-04 — bytes that load and cannot be decoded panicked the game on Bevy's path, and four sentences said they were refused

**Change.** Bevy's `AudioSource::decoder` is `rodio::Decoder::builder()…build().unwrap()` (`bevy_audio-0.20.1/src/audio_source.rs:96-101`) and `play_queued_audio_system` calls it in `PostUpdate` for every voice with an `AudioPlayer`. The asset loader returns a file's bytes and stops, so a file that **loads and is not audio the build decodes** — a Git LFS pointer, an empty file, a cut Ogg, an **Ogg Opus** file, an ADPCM WAV — was answered `Played` and then **panicked the game in the same frame**, on the default build, naming no file. The crate's own mixer always refused such bytes (`start_pending`); `Cargo.toml`, `playback.rs`, `play.rs` and `cues.md` said the crate did; and no test could see the other path, because **a headless app has no device and so never runs Bevy's decoder**.
Now: **(1)** `sounds::decoder_error` makes **Bevy's own call**, minus the `unwrap`, through `<AudioSource as Decodable>::Decoder` — so **`rodio` does not become a dependency of the default build**; **(2)** `play_requests` refuses at the play when the bytes are there and Bevy's path owns the voice (no `Played`, no voice, no `AudioPlayer` for `PostUpdate`, nothing counted), and `playback::refuse_unplayable_voices` (the renamed `refuse_failed_loads`) refuses a voice whose bytes arrive undecodable **in the frame they arrive — `Update`, before that frame's `PostUpdate`**; **(3)** the log carries the decoder's words and, for the three deliveries it can tell apart by their first bytes (a Git LFS pointer, no bytes, `OpusHead`), **says what the file is** — a hint changes a sentence, never a decision; **(4)** what is refused is **a property of the build** (an MP3 is refused where `rodio` has no decoder for it and played where it has one), because the verdict is the decoder's; **(5)** the answer is asked **once per buffer**, from a cache in `SirenSounds` keyed by buffer address, length and first 16 bytes, so an edit is asked again. **No new `Refusal` variant** (option 4 of `needs-decision/undecodable-sound.md`). **Only Bevy's path is asked**: with an `Output` the crate's mixer decodes in `start_pending`, and applying the check there turned six existing tests red.

**Rationale.** Iteration 116's lesson — *a test fixture that is always in the happy state cannot see the other states* — asked of the sentences about **bytes**: which test builds a file whose bytes are not audio, on the path a consumer runs? None. The crate's own rule (*every play answers exactly one of `Played`/`Refused`, and nothing plays on a refusal*) was false for six kinds of file on the default build. A deterministic crash from the commonest delivery mistakes outranks the runner-up, a 3 % one-frame skew of layers that nothing the crate says is false because of. Sources: Ousterhout, *A Philosophy of Software Design* §10.8 (*Just crash?*: crash for errors that are *difficult or impossible to handle and don't occur very often* — a bad sound file is easy to handle and common) and §10.7 (a request aborted, the system continuing); Kopetz's principles 3 and 11 in Marwedel, *Embedded System Design*, DOI 10.1007/978-3-030-60910-8, §5.6.4; RFC 7845 §5.1 (`OpusHead`).

**Measured.** The probe, on the real `AudioPlugin` (this Mac's device) and the real `AssetServer`, one process per file: **six `Played`, no `Refused`, six panics at `audio_source.rs:101:14` in the first update after the play** (`evidence/before/`); after: **0 `Played`, six `Refused`, 0 voices, 0 sinks, 0 panics**, the log naming the sound and the decoder's words and the hint, and the two controls (an Ogg, a WAV) given a sink in frame 1 as before (`evidence/after/undecodable-probe-after.txt`, `docs/media/undecodable.png`). **Four of the five integration tests are red on the unchanged tree and the control passes**; one holds that the crate refuses **exactly the bytes `AudioSource::decoder()` panics on** (`catch_unwind`), in every feature set. Guards switched off in turn: the play-time check (3 of 5 red), the in-flight check (1), the key check (1 + 1 unit), the address in the key (1 + 1), the memo itself (the once-per-sound test reads **14 constructions where one is expected**), and the Bevy-path-only rule (six existing mixer tests). **Cost**: first sight of a Vorbis **0.8 ms** — a decoder construction, once per sound; Bevy pays the same on *every* play — a `hound` WAV 0.1 – 0.2 µs, `symphonia`'s WAV, MP3 and FLAC 10 – 180 µs; a memoised verdict **21 ns**; the system **0.06 – 0.11 µs a frame** with nothing waiting (6.5 µs at 256 voices that can never start, a headless app). **Render: 21 of 21 WAVs byte-identical** to 116's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, spectrogram read; `perfcheck` 0.060 – 0.062 / 0.321 – 0.362 / 0.571 – 1.216 µs at 8 / 64 / 256.

**Predictions** (`research.md`, scorecard, wrong ones first): **P3 wrong by 2.5×** (a Vorbis is 0.64 – 0.8 ms to construct whatever the length, not ≤ 0.3); **P2 half-wrong** (what a decoder reads is a property of the build — µ-law is `Err` with `hound` and `Ok` with `symphonia`; MP3 and FLAC are `Err` in the default build — which is why the permanent test is an oracle and not a table); **P5 half-wrong** (one file changed, but three rows); P1, P4, P6 right as written.

**The catches, because it was wrong first.** *A sound's first play built its decoder twice, and every document I had written said once*: a `Local` memo in each of two systems, found by reading the diff, fixed by one cache in `SirenSounds`, and a `cfg(test)` record that makes *"once"* checkable. *A warning clippy cannot see*: `let mut rows` unused in a build without `mixer`/`semantic`, on three commands of the matrix log — clippy runs with `--all-features`. *A unit test said "the same head" of buffers whose heads differed*; mutating the key left it green. *The first benchmark measured Bevy's executor* (7 – 8 µs at zero voices). *The first probe's `frame` lines were filtered by my own `grep -v`.* *Mutations C and D were not re-run after the memo moved.* All in `review.md`.

**Evidence.** [`runs/2026-11-04-117/`](runs/2026-11-04-117/) — `eval.md` (the claims audit: four sentences, one test row and one comment that were false or held nothing), `research.md` (seven sources, six predictions and their scorecard), `plan.md` (and what happened to it), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix-final.log`; `evidence/before/` (the probe on the unchanged tree, the red run, every mutation); `evidence/after/` (the probe after and its source, the cost and the benchmarked source, perfcheck, the render set); `evidence/spectrograms/`.

**Picture**: `docs/media/undecodable.png` (a transcript — a panic before, a refusal after, a control sounding; nothing to hear). `cues.yaml` case **38**; `docs/guide/cues.md`; `AGENTS.md` gained the layout lines and the trap *"Bevy's `AudioSource::decoder` is `….build().unwrap()`, and a headless app never calls it"*; the skill gained three lessons (read the `unwrap`s of the code your entity is handed to; test a verdict that depends on the build against an oracle; clippy with all features is not a check of the default build — run it first and grep the matrix for `warning`).

**Baseline:** green on arrival — 5 lib + 208 / 5 + 217 / 114 + 526 / 16 + 222 / 125 + 553, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **9 + 213 / 9 + 222 / 118 + 531 / 20 + 227 / 129 + 558**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways; the only warning is the `example "mixer"` linker one. **One existing test changed** (`tests/ogg_loops.rs`: a row of bytes that are not audio, a state that crashed a game, and two rows that exist only where the build decodes them). **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit; no API it uses changed. **Not listened to** — there is nothing to hear.

**Verdicts.** Function **GREAT** (after eight catches), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `9a339fa`.

**Recorded, not fixed.** **`SirenSounds` is the one public type touched**: a private `Mutex` field and `Clone`/`Debug` by hand (a clone starts with an empty cache); every trait it had it still has. **The crate's own mixer still answers `Played` then `Refused` (same frame) where Bevy's path now refuses at the play** — and a `Played` whose entity is despawned in the same frame is a hazard for a game that tags it (an `AudioOutput::decodes` hook, public, carved off). **`Refused.reason` still prints the one `MissingSound` sentence** for an LFS pointer; the cause is in the log (`needs-decision/undecodable-sound.md`, whose note now says a variant could carry the hint). **`MusicState::blocked` cannot say why a layer waits** (budget, a dead file, an undecodable one). **First-sight burst and a debug build are unmeasured**: twenty distinct Oggs in one frame is about 16 ms by arithmetic. A load that never resolves still waits. **A `Play::on` for an owner that no longer exists** leaves an ownerless loop — Bevy *warns and removes* the `ChildOf` (`bevy_ecs-0.20.1/src/relationship/mod.rs:207`) — **read, not run**. **Tool discipline**: files were read with `read`, `grep` and `glob`, and bash ran programs; the slips (`grep` pipelines after `cargo test`, one `head`) are in `review.md`.

**Top 5 remaining.**

1. **A person listens** — a looped Ogg stem coming in on the bar, the wrap that was a 12 ms hole, a fade, a FLAC and an MP3 loop — on Bevy's path and on `mixer`; and a capture of the device's output if one can be made. Until then every claim about how a loop *sounds* is a claim about samples.
2. **The crate's own mixer refuses at the play too** — `AudioOutput::decodes(&self, bytes) -> bool` with a default of `true` (a fake output is unaffected) and the mixer's own decoder construction, so a refusal is one answer on both paths and `Played`-then-`Refused` goes; with `SirenSounds::undecodable` beside `missing` and `failed` for the editor's panel. The mixer's `Decoder::new(Cursor)` and Bevy's builder with `with_byte_len` may disagree on a file — measure that first.
3. **The layers of a score start together** — a barrier in `music::start_missing` that holds the batch while any stem is `Loading` (a failed or undecodable one does not hold it), with a bound on the wait that has a source and an additive accessor on `MusicState`; the design is in `runs/2026-11-04-116/plan.md`. Measured at 3 of 100 warm-cache starts of eight stems; a cold disk is unmeasured.
4. **`Play::on` for an owner that does not exist** — probe it (a loop on a despawned owner, both backends): if it is an ownerless loop, refuse at the play or stop it with the owner's absence, in its own commit; and iteration 115's one-time Ogg-loop probe (a game with Bevy's `symphonia-vorbis` on loses every Ogg loop on Bevy's path with no word).
5. **Waiting on a person, another repository or hardware, and the owner's decisions:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; **`needs-decision/undecodable-sound.md`** (the `Refusal` variants — two causes now); the default-floor decision and a hubness correction measured with a held-out bank; memory and copies on the loop path (`play`'s `bytes.to_vec()`); a third library; the CI gap (nothing runs the benchmark; the mixer's device tests return early with no audio device); an index stamp that does not name the decoder; an MP3 with no tag.

## Iteration 116 — 2026-11-04 — a play whose file failed to load was answered `Played`, and nothing ever sounded

**Change.** Every refusal the crate can name is decided without a file — a cue the table lacks, a key with no handle, a bus not in the mix, bytes the decoder refuses — and a handle whose **file failed to load** was none of them. The handle exists, the asset server knows the load failed and says why (`LoadState::Failed`, `bevy_asset-0.20.1/src/server/mod.rs:2112`), and the crate waited for the bytes **for ever**: `Played` was written, the voice sat in the budget, and the log read *played* and *stolen by ui.click (cue)* — a healthy sequence for a sound nobody heard. Bevy's own start system never reads the state either (`audio_output.rs:109`).
Now: **(1)** `play_requests` refuses it at the play (`Refusal::MissingSound`; `sounds::cannot_load` — the state is `Failed` **and** there are no bytes to play), after the handle lookup and before the bus check, so a failed file claims no budget slot; **(2)** `refuse_failed_loads` (`playback.rs`, chained after `start_pending`, both paths) refuses a voice spawned while the load was in flight and whose load then fails — in the frame the server says so — and frees it (`Played`, then `Refused`: the two answers `start_pending` already gives undecodable bytes); **(3)** the log's sentence carries the server's own words: *"the file for sound `typo_click.ogg` failed to load: Path not found: …"*. **A file whose *reload* failed but whose previous bytes are loaded still plays** (a failed reload removes nothing). A load still in flight is not refused, and a handle the server has no record of waits as before. **No new `Refusal` variant** — option 4 of `needs-decision/undecodable-sound.md`; the cause is in the log.

**Rationale.** The claims audit asked *"which sentence about a voice and its file is held by a test that builds the state?"* The crate's own rule — *every play answers exactly one of `Played`/`Refused`, and nothing plays on a refusal* — was held by tests whose handles are `Handle::default()`, a Uuid handle `get_load_state` answers `None` for, so **a failed file was a state no test could build**. The probe on the real `AssetServer` with the default cue (`max_instances: 1`, which steals the dead voice at every play and so hides the leak): **six plays of a misspelt file, six `Played`, no `Refused`, three dead voices alive** (`evidence/before/`). Sources: Avižienis, Laprie, Randell and Landwehr 2004 (a failure the service interface does not signal is the worst mode), Gregory §14.5.8.4. The audit found a second *"a voice waits for its asset"* defect — the layers of a score start apart when their loads straddle a frame, **3 of 100 starts of eight 3.7 MB stems, worst one frame, permanent** — measured and **carved off** with a design (`plan.md`): rarer, and nothing in the crate's own answers was untrue because of it.

**Measured.** After, same probe: **0 `Played`, 6 `Refused`, 0 voices, 0 counted on `Effects`**, every log line naming the key and the path (`evidence/after/failed-load-probe-after.txt`); `docs/media/failed-load.png` is the before/after transcript. On the unchanged tree **three of the first four tests are red** (four of five with `mixer`), the control passes. The system costs **0.06 µs a frame** with no waiting voice, 0.06 with 256 running ones, **0.77** with 256 that never start; the check at a play is **8.5 ns** for a loaded handle, 138 ns for a failed one (first cut — the shipped check returns after one `Assets::get`, so these are upper bounds; `perfcheck` runs model arithmetic only and cannot see the system: 0.059 – 0.064 / 0.336 – 0.355 / 1.189 – 1.245 µs at 8 / 64 / 256). **Render: 21 of 21 WAVs byte-identical** to 115's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, spectrogram read.

**Predictions** (`research.md`, scorecard): five of five right as written; P1 was a regression statement (the probe came first). **What the table did not contain** is the state the build found: a failed reload with the bytes still loaded.

**The catches, because it was wrong first.** *My first cut refused a play whose reload had failed* although the old bytes were loaded — found by asking what `Failed` with bytes present means; the sixth test went red on it and `cannot_load` replaced the bare predicate. *The matrix's first run was red on clippy alone, for the second iteration running* (`private_interfaces`: a crate-private `SystemParam` in a public signature, a warning on every build line I did not read); a tuple alias fixed it and the matrix ran again. *A premise of my tests was a race* — a loader that slept 150 ms to hold a load in flight — and became a gate the test opens; 23 of 23 runs green, eight with every core busy (no before/after flake rate: I did not run the sleeping version loaded). *The mutations first ran on the first cut*; re-run on the final code and again on the gated tests. *A multi-file `edit` is not atomic* (one file applied, two refused). And five sentences corrected on a line-by-line read (`failed()`'s doc, a count in case 37, a scorecard heading, stale `Slow`/`Files::` names, a missing paragraph in `music.md`). All in `review.md`.

**Evidence.** [`runs/2026-11-04-116/`](runs/2026-11-04-116/) — `eval.md`, `research.md`, `plan.md` (and what happened to it; the carved-off barrier's design), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix-final.log`; `evidence/before/` (the probe on the unchanged tree, every red run and every mutation); `evidence/after/` (the probe after, the stem-skew program, the benchmark and its source, the stability run, the render set); `evidence/spectrograms/`.

**Picture**: `docs/media/failed-load.png` (a transcript — nothing to hear). `cues.yaml` case **37**; `docs/guide/cues.md` (the third cause of `MissingSound`, the log line), `docs/guide/music.md` (a dead stem is the same virtual voice as a budget refusal); `AGENTS.md` gained the layout lines and the trap *"a handle is more than a handle"* with the gated-loader method; the skill gained *"a test fixture that is always in the happy state cannot see the other states"* and *"tool discipline is part of the loop"*.

**Baseline:** green on arrival — 203 / 212 / 521 + 114 / 216 + 16 / 547 + 125, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **208 / 217 / 526 + 114 / 222 + 16 / 553 + 125**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways (the matrix's third full run; the first was red). **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit; no API it uses changed. **Not listened to** — there is nothing to hear.

**Verdicts.** Function **GREAT** (after seven catches), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `c810111`.

**Recorded, not fixed.** **`Refused.reason` still prints the generic *"no handle or undecodable bytes"* sentence** for a typo'd file — `tests/play.rs:295` pins the wording and a variant is a public change; `needs-decision/undecodable-sound.md` now argues for two variants under option 1. **`MusicState::blocked` cannot say *why* a layer waits** (budget or a dead file); the log can. **A load that never resolves still waits for ever** (no source for a timeout). **Bytes that load and do not decode still panic Bevy's own path** (`AudioSource::decoder` unwraps) where the mixer refuses them. **Tool discipline**: files were read with `read`, `grep` and `glob`, and bash ran programs; the slips (an `ls -l | awk` over my own scratch, a parallel `read` that raced a `cp`, edits with stale anchors after `cargo fmt`) are listed in `review.md`.

**Top 5 remaining.**

1. **A person listens** — a looped Ogg stem coming in on the bar, the wrap that was a 12 ms hole, a fade, a FLAC and an MP3 loop — on Bevy's path and on `mixer`; and a capture of the device's output if one can be made. Until then every claim about how a loop *sounds* is a claim about samples.
2. **The layers of a score start together** — a barrier in `music::start_missing` that holds the batch while any stem is `Loading` (a failed one does not hold it), with a bound on the wait that has a source and an additive accessor on `MusicState` for a panel; the design and its tests are in `runs/2026-11-04-116/plan.md`. Measured at 3 of 100 warm-cache starts of eight stems; a cold disk is unmeasured.
3. **What cannot sound should say so at startup, on Bevy's path**: bytes that load and do not decode panic (`AudioSource::decoder`), where the mixer refuses them — pre-validate when `rodio` is present; and a game with Bevy's `symphonia-vorbis` on loses every Ogg loop with no word (iteration 115's one-time probe, an embedded ~5 KB Ogg).
4. **Memory and copies on the loop path**: `play` copies the whole sample (`bytes.to_vec()`) on every play, `Repeat` keeps the decoded loop as `f32`, `looped`'s box is unmeasured, and a long stem wants a streaming source.
5. **Waiting on a person, another repository or hardware, and the owner's decisions:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; **`needs-decision/undecodable-sound.md`** (the `Refusal` variants — two causes now); the default-floor decision and a hubness correction measured with a held-out bank; a third library; the CI gap (nothing runs the benchmark; the mixer's device tests return early with no audio device); an index stamp that does not name the decoder; an MP3 with no tag.

## Iteration 115 — 2026-11-04 — an Ogg loop produced no samples, and the one that did was longer than its stream

**Change.** The item was iteration 114's first: *an Ogg loop is not sample-exact on either path*. The first measurement found something under it — **`decoder.repeat_infinite()` of a real Ogg produced 0 of 147 456 samples.** `Cargo.toml` named `rodio`'s `vorbis` feature, which is the `symphonia` decoder; it reports a span of length zero
until its first sample (a Vorbis stream's first packet carries no audio), `Buffered` reads that as *an empty source* (RustAudio/rodio#775, open; Bevy's 0.18 → 0.19 migration guide lists *"looping"*), and `repeat_infinite` is Bevy's own loop as well as the mixer's. On the real `MixerBackend` a loop of a 0.1 s Ogg **had ended before 350 ms**; `Played`
had been written and nothing was heard. It was there from the first commit and no test saw it: **every loop test used a WAV**. Cargo unifies features and `rodio` compiles `lewton` out when `symphonia-vorbis` is on, so the same line swapped the decoder of a game's **own** `AudioPlayer`. Now: **(1)** the line names **`lewton`**, Bevy's default; **(2)** the mixer's loops
go through `OpenSpan` (a decoder that reports an empty first span still loops), for a game that turns on `symphonia-vorbis` itself; **(3)** the loop is **as long as the stream declares** — `src/ogg.rs` reads the last page's granule position (one walk of the pages, refusing a chained or grouped file, a file cut short, a first packet that is not Vorbis, a stream the packet count cannot account for),
and **both paths cut their loop to it**: `trim_ogg_loops` sets `PlaybackSettings::duration` on Bevy's, `Shape::of` trims `looped` on the mixer, and `semantic::decode` reads its length through the same walk. The duration is **not `frames / rate`**: `rodio`'s `TakeDuration` is exact for `N·p + 1` ns (`p = ⌊10⁹ / (rate × channels)⌋`).

**Rationale.** Not the backlog and not the audit's first question: the claims audit asked *"does any test read a loop's length?"* (none) and not **"does any test loop an Ogg at all?"** (none). The plan (iteration 114's, costed) was kept as item 3 and became the third of three; the empty loop outranks it because a voice that does not sound is wrong output of the worst kind and a gap of 12 ms is not.

**Measured.** `tests/fixtures/tone.ogg` declares 24 000 frames; the decoder returns 24 576. Through Bevy's decoder call, on `lewton`, **before: period 24 576 frames (512.000 ms), a 563-frame (11.7 ms) near-silent run at every wrap; after: exactly 24 000, a 0-frame run**; the spectrogram's clicks at the wraps fall to a faint residue and the 70 ms waveform crop goes from a 12 ms hole to a continuous line (`evidence/spectrograms/`, **read**).
On Kenney's 254 CC0 Ogg effects, **51 decode longer than they declare** (27 – 936 frames, median 412 = 9 ms, **the loudest padding at −6 dBFS**) and **254 of 254 are exact after the trim**. `frames / rate` is wrong in **77 of 108** rate × channel × length cells (by up to 158 samples); `N·p + 1` in **0**. The page walk costs **0.03 µs** for a typical effect and **8.2 µs** for a 9 MB file; `trim_ogg_loops` **0.07 – 0.09 µs a frame**.
**The decoder swap**, against `libvorbis` on the same 254: `lewton` within 7 × 10⁻⁷ everywhere, hot masters included; `symphonia` clamped 25 files at ±1 and was off by up to 0.30. `lewton` decodes **1.8× slower** (23 against 41 million samples a second, 240× real time for stereo 48 kHz), costs two crates (`semantic`: 497 lines / 183 crates → 493 / 185), and the **Kenney golden benchmark with the real CLAP model is unchanged to four decimals on all seven metrics**.

**Predictions** (written before the run, `research.md`): five of eight right as written. **P1 was wrong in the way that mattered** — the loop was empty, not 12 ms late — and **P5 and P2 were right on one decoder and silent about another** (a count measured on `symphonia` built from a `File` carried into a claim about `lewton`; the construction was never compared on one set).

**The catches, because it was wrong first.** *The matrix's first run was red on clippy alone* (`very complex type used` on the new system's query; a `type` alias, then the whole matrix again). *A test held nothing*: `first == second` over an empty loop is two empty vectors — it now asserts the length first and fails on the old line (*"produced 0 of 48 000"*). *My first headless test did not force Bevy's path* (`Backend(false)`
alone; `start_pending` reads `Output`) and failed for another reason on a machine with a device. *Three sentences in docs I had not measured* (a tagless MP3, who writes a positive granule, a duration in `plan.md`) were rewritten; *a symphonia count was stated without its construction* in four places. All in `review.md`.

**Evidence.** [`runs/2026-11-03-115/`](runs/2026-11-03-115/) — `eval.md`, `research.md` (seven sources, eight predictions and their scorecard), `plan.md` (and what happened to it), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix-final.log`; `evidence/after/` (`ogg-loop-report.txt` and `ogg-loop.rs.txt`, `vorbis-backend-parity.txt` and its script, `goldenbench-kenney-lewton.txt`, `trim-system-cost.txt`, the render set and loudness); `evidence/before/` (every red run and every mutation); `evidence/spectrograms/` (the wrap before and after, the worst file, the standard mix).

**No clip**: the pictures are the evidence (`docs/media/README.md`, "No clip: iteration 115"). `cues.yaml` cases **35** (*an Ogg loop sounds*) and **36** (*it is as long as the stream declares*); `AGENTS.md` gained the layout lines for `src/ogg.rs` and the Vorbis-backend trap; the skill gained *"a test of a thing that loops needs a loop of every format the crate says it plays, and must compare a length"*.

**Baseline:** green on arrival — 200 / 209 / 517 + 109 / 6 + 211 / 542 + 115, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **203 / 212 / 521 + 114 / 16 + 216 / 547 + 125** (plus 5 lib on the default and serde builds), 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways; **render 21 of 21 WAVs byte-identical** to 114's, −21.5 LUFS, −7.5 dBFS TP, spectrogram read;
perfcheck 0.039 – 0.057 / 0.237 – 0.333 / 0.411 – 1.220 µs a frame at 8 / 64 / 256 (inside the band; it measures the mix, which this does not touch). **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit.

**Verdicts.** Function **GREAT** (after seven catches), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `8d547bc`.

**Recorded, not fixed.** **Nobody has listened** to any loop: the evidence is samples and pictures (`docs/guide/listening.md` is the procedure). **A game that turns on Bevy's own `symphonia-vorbis` gets silent Ogg loops on Bevy's path whatever this crate does** — documented, not detected. `looped`'s `Box<dyn Source>` costs one allocation a loop start and a virtual call a sample on the first pass; unmeasured. An index built on the old decoder is not invalidated (the stamp is not a decoder stamp) and
differs only for the 25 clamped files. A stream that starts at a positive granule is not detected; an MP3 with no encoder tag was not measured. **Tool discipline**: files were read with `read`, `grep` and `glob`, and bash ran programs; the slips (one `sed -i` on `Cargo.toml`, a `cat` of a log, four `ls | awk`, `head`/`tail`/`tee` on program output) are listed in `review.md`.

**Top 5 remaining.**

1. **A person listens** — a looped Ogg stem coming in on the bar, the wrap that was a 12 ms hole, a fade, a FLAC and an MP3 loop — on Bevy's path and on `mixer`; and a capture of the device's output if one can be made. Until then every claim about how a loop *sounds* is a claim about samples.
2. **Detect an Ogg loop that cannot sound at startup.** A game with Bevy's `symphonia-vorbis` on loses every Ogg loop on Bevy's path and nothing says so: a one-time probe (an embedded ~5 KB Ogg through `rodio`'s decoder, `repeat_infinite().take(2n).count()` against `2n`) would warn. The cost is a binary asset in `src/`; the benefit is the failure this iteration spent its first hour finding.
3. **A hubness correction, measured with a bank held out from the evaluation** (iteration 113's item 1), on both libraries in both wordings; **the default-floor decision** and the other `needs-decision/` files (owner's); resampler fidelity at 44.1 kHz; a third library.
4. **Memory and copies on the loop path**: `play` copies the whole sample (`bytes.to_vec()`) on every play, `Repeat` keeps the decoded loop as `f32` (about 23 MB a minute of 48 kHz stereo, by arithmetic), `looped`'s box is unmeasured, and a long stem wants a streaming source.
5. **Waiting on a person, another repository or hardware:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; `find_compatible_*` speed; `needs-decision/headless-discovery.md`; iteration 105's `#[non_exhaustive]`; the CI gap — nothing runs the benchmark, and the mixer's device tests return early on a runner with no audio device; **an index stamp that does not name the decoder** (a schema decision); an MP3 with no tag.

## Iteration 114 — 2026-11-03 — a looping voice on the crate's own mixer ignored every gain change after its first pass

**Change.** `MixerBackend::play` built a loop as `VoiceGain { decoder }.repeat_infinite()`, and `rodio`'s `repeat_infinite` **stores what it is given** (a span of up to 32 768 samples at a time, `buffered.rs:98-125`) and replays the stored values — so the voice's gain was the
gain of the instant its first pass was decoded, **for ever**. A layer decoded silent never came in; a stop's 120 ms fade and a stolen loop's fade were clicks; nothing refused or logged, because `update_mix` did store the number and `SirenMix` said the layer was at 0 dB. **Bevy's path was never affected**
(a sink's volume sits outside its repeat; `audio_output.rs:165`). Now `looped` repeats the sample **then** applies the gain, and `connect` is the one function that makes a voice. **Six unit tests** in `src/mixer/mod.rs` (the first in the crate to read a sample its own mixer
produces; **four fail on the old order**), **one real-device test** in `tests/mixer.rs` (**no test had ever played a loop through the real `MixerBackend`**), `cues.yaml` case 34, an `AGENTS.md` trap, and the module doc.

**Rationale.** Not the backlog: **the claims audit outranked it**. The sentence *"both backends answer identically; only the sample path differs"* appears in four files and was held by a fake `AudioOutput` that records the calls it receives — the sample path itself had never been read by anything. Reading `play`
beside `rodio`'s source, then a probe with the crate's own `VoiceGain` copied verbatim and the real decoder, showed pass 2 at 0.08837 where 0.02209 was commanded. All five open issues were re-read and are blocked on a person, another repository or the owner (`#32`–`#34` the editor, pinned 221 commits back;
`#35` anchor pairs; `#39` decided not to ship); no `consumer` issue is open.

**Measured** through the real `Decoder`, a `Player` and a bus `Mixer`, pulled offline — what the device is handed, minus the device — **every output compared sample by sample with what the schedule commands**. A stem commanded to 0.25 on pass 2 played at 1.0 (worst difference **0.094** on a 0.125-peak stem; after: **0.000000** on every pass).
A two-stem score, combat decoded silent and moved to 1.0 on pass 2: **660 Hz at −∞ dBFS on all four passes before; −∞, −18.99, −18.06, −18.06 after** (commanded the same), and the calm stem, commanded 12 dB down on pass 4, stayed at −18.06 before and is −27.81 after. A gain stored inside the first pass reached
the speaker **1 to 32 767 samples late** before and **0** after. **The cost is unchanged**: 3.243 / 3.233 / 3.215 ns a sample before, 3.227 / 3.226 / 3.214 after (three runs of best-of-20; an earlier single run read +4.9 % and was noise).

**The catches, because it was wrong first.** *The probe came before the predictions* — P1 and P2 are regression statements, and the one that could have gone the other way (the cost) held. *My first tests built the `Player` by hand*, so a later `play` could have diverged; *my first end-to-end test ran in the bus's own format* where nothing converts
(production is 44.1 kHz stereo on a 48 kHz bus — now a test, red on the old order: *"0.17648624 then 0.17648618"*, the level did not move); *the first waveform pictures were blank* (`colors=white` on a transparent PNG; the skill is fixed); *the first spectrogram's streaks* were my 10 ms gain staircase, not the fix (the output equals the
commanded signal to 0.000000). **And the fourth matrix was red, not on my code**: two indexer tests both named their scratch directory `"formats"`, every test is one process, and run together they **failed ten times in ten** — a flake in the baseline since iteration 112. All six `Scratch` helpers now give every call its own directory; ten of ten pass.

**Evidence.** [`runs/2026-11-03-114/`](runs/2026-11-03-114/) — `eval.md`, `research.md` (seven sources, five predictions), `plan.md` (the ranking and **the carved-off Ogg route**), `perf.md`, `review.md`, `baseline-matrix.log`, `matrix.log`, `matrix-red-flake.log`, `matrix-final.log`; `evidence/after/loopgain.txt` and `loopgain.rs.txt` (the program), `loudness.txt`,
`unit-tests-on-the-fixed-order.txt`, `indexer-scratch-race.txt`, the render set; `evidence/before/` (the unit tests on the old order, the race); `evidence/spectrograms/` (the score and the loop, before and after, **read**).

**No clip**: the mixer path is not what `examples/render` renders; the pictures are the evidence (`docs/media/README.md`, "No clip: iteration 114").

**Baseline:** green on arrival — 200 / 209 / 517 + 109 / 210 / 541 + 109, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **200 / 209 / 517 + 109 / 6 + 211 / 542 + 115**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples both ways; **render 21 of 21 WAVs byte-identical** to 113's, −21.5 LUFS, −7.5 dBFS TP, spectrograms read;
perfcheck 0.057 – 0.064 / 0.322 – 0.344 / 0.797 – 1.101 µs a frame at 8 / 64 / 256 voices (inside the band; the mixer feature is off there). **The Cauldron's panel test was not run**: the editor pins `82d227e`, 221 commits back, and its 14 GB target does not fit.

**Verdicts.** Function **GREAT** (after nine catches, four of them in my own tests), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `cc38692`.

**Recorded, not fixed.** **Nobody has listened** to the mixer path: the evidence is every sample of an offline render, not an ear (`docs/guide/listening.md` is the procedure). A loop at `speed` ≠ 1 has no test. `Repeat` keeps the decoded loop in memory as `f32` — about 23 MB a minute of 48 kHz stereo, by arithmetic and not by a watched RSS — and decodes its first span
on the audio thread. `play` copies the whole sample (`bytes.to_vec()`) on every play. The example's *"1 assets"* beside *"4 failed"* (112). **Tool discipline**: reads and searches outnumbered bash more than two to one; three bash calls did what `glob`, `write` and `edit` are for — one of them a Python rewrite of a scratch file — and two `edit` ranges came from memory instead of the last read (`review.md`).

**Top 5 remaining.**

1. **Ogg loops are not sample-exact, on either path** (symphonia keeps the last block's padding: up to 975 frames, 20 ms, a pass — iteration 112 — against a gap threshold near 3 ms; Brennan et al. 2018). **The route is found and costed** (`plan.md`): bevy_audio already builds `decoder.take_duration(duration).repeat_infinite()` from `PlaybackSettings::duration`, so the default path needs no custom decoder; the mixer uses the same `take_duration`
   before `looped`; `TakeDuration` emits exactly `N` samples when `remaining = N·p + 1 ns`, `p = ⌊10⁹ / (rate × channels)⌋`; the Ogg reader (`semantic::decode::ogg_declared_frames`) needs a bytes-based home outside `semantic`, and a positive starting granule (Vorbis I A.2) is not handled and must be named.
2. **A person listens to the mixer path** — a looped layer entering on the bar, a fade, an Ogg, FLAC and MP3 — and a capture of the device's output if one can be made; until then every claim about how it *sounds* is a claim about samples.
3. **A hubness correction, measured with a bank held out from the evaluation** (iteration 113's item 1): on both libraries in both wordings, the criterion written first. The crate change is the owner's.
4. **The default-floor decision** and the other `needs-decision/` files (owner's); **resampler fidelity** at 44.1 kHz; **a third library**; `play`'s per-play copy and `Repeat`'s memory.
5. **Waiting on a person, another repository or hardware:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 221 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; `find_compatible_*` speed; `needs-decision/headless-discovery.md`; iteration 105's `#[non_exhaustive]`; and the CI gap — nothing runs the benchmark, and the mixer's device test returns early on a runner with no audio device.

## Iteration 113 — 2026-11-03 — the second golden library: game audio answers the floor question the other way

**Change.** `#35`'s second library — **303 game sounds from Kenney's CC0 packs**, 29 classes, WAV, MP3 and Ogg Vorbis in one folder, 221 of the 303 under a second — run through the real CLAP
model. `golden/kenney/` (a manifest pinning seven archives and 303 files by SHA-256, the labels, two files of 47 queries), `tools/fetch_kenney.py` (one command; a stray, damaged or wrong-bytes
file is refused by name), **`goldenbench --set <name>`** with a baseline per set, and three additive public items: `semantic::eval::{centred, separation, Separation}`. The labels and queries were
**committed before the model ran** (`c303915`), with eight predictions. 12 tests (`tests/semantic_golden.rs`), each red first. **ESC-50's identity string is byte-identical and its baseline still
compares: no regression, all seven metrics +0.0000.**

**Rationale.** Rank 1 of iteration 112's own list: the evidence the floor decision was waiting on, and the one question 112 raised — *are a game's correct scores near 0.2 or near ESC-50's 0.47?*

**What it found** (maintainer's / stranger's wording; ESC-50 in brackets). Zero-shot **37.3 %** (87.8); hit@1 **31 / 31 %** (90 / 72); hit@3 **51.7 / 37.9 %** (100 / 84); the right answers' median cosine
**0.191 / 0.193** (0.468 / 0.402); the weakest top result of an answerable description **0.261 / 0.238** (0.492 / 0.405). *A helicopter* finds an `engine` at **0.490**, above the best result of 27 of 29 answerable
descriptions; one class is the first result for 22 of 47 descriptions (a hub, skewness 4.78 against ESC-50's 3.82). **13 of 29 descriptions have a best result under 0.35** (Wilson 95 %: 28 – 62 %; 6 of 29 in
the stranger's wording) against **0 of 50** (0 – 7 %): iteration 111's `0.35`, free on ESC-50, **returns nothing at all** for them, and loses a fifth to a half of the hits. **0.20 is the largest floor that cost
neither library an answer in either wording, and it silences no no-answer query** — the largest floor that cost nothing here, not a safe one. A floor *relative to the query* (`eval::centred`) *lowers* Kenney's
AUC (0.759 → 0.626; the interval of the difference excludes zero in both wordings). The default stays `0.0`; `needs-decision/default-floor.md` **recommends keeping it** and withdraws `0.35` for good.

**The retractions and catches, because the work was wrong first.** *Three of eight predictions were wrong and one half right*, all optimistic about the model on game audio (retrieval predicted 60 – 85 % at hit@1, measured 31 %; centring
predicted to help, it hurt). *Four counts in the guide, typed from memory, were wrong* (21 of 29 → 13; five classes at 0 % → seven; a third to a half → a fifth to a half; "within one query" → exact, one, two) — caught by recomputing from the
dumps before the commit. *The Python cross-check disagreed with the crate twice*: `np.median` against the crate's nearest rank, and an **`f32` AUC that printed an exact 0.9625 as 0.962** (now an `f64`). *The finding left six stale claims*
(`semantic.md`, two doc comments, the README, the guide's own first section, ADR 0009): rewritten, and a grep for `0.35` is clean. *The plan's fallback — "a project sets a floor from its own no-answer queries" — was refuted by its
own data* (the strongest no-answer top is above 27 of 29 answerable tops): the guide now says to use the answerable ones. And **a bootstrap over the descriptions, asked on reading `separation` rather than before,
showed the AUC alone does not separate the two libraries in the stranger's wording** (Kenney 0.65 – 0.94, ESC-50 0.78 – 0.96); the count of descriptions under 0.35 does. The guide, the decision file, `semantic.md`, the CHANGELOG
and the `#35` comment say so.

**Checked by something else.** A Python recomputation agrees on the seven metrics, the separation and 55 floor cells on **each of four runs** (both libraries, both wordings); against `transformers` on all 303 files the
48 kHz clips are **identical**, the 44.1 kHz ones differ by a resampler's amount (median 1.1 × 10⁻³, mean over all similarities 0.004), and hit@1 and hit@3 are exact. Padding (`pad` against the model's own `repeatpad`) gave **no
consistent gain**, so nothing changed. Ten wrong uses of `goldenbench` (a partial library, a padded one, another set's labels, `--bless` with the wrong queries file …) each exit 2 with one sentence, writing nothing.

**Evidence.** [`runs/2026-11-03-113/`](runs/2026-11-03-113/) — `research.md` (eight sources and the predictions), `plan.md`, `eval.md` (before and after), `perf.md`, `review.md`, `matrix.log`; `evidence/after/` (both `goldenbench` outputs
per library and wording, the ESC-50 refactor proof, `kenney_parity.py` and its output, `normalisations.py` (hubness; the gallery-side normalisation, exploratory), `bootstrap.py`, `goldenbench-edge-cases.txt`, the render set,
`spectrograms/`).

**Clips:** none; the visible change is two terminal stills, [`golden-kenney.png`](../media/golden-kenney.png) and the re-captured [`golden.png`](../media/golden.png) (new sections 3b and 4b).

**Baseline:** green on arrival — 200 / 209 / 505 + 109 / 210 / 529 + 109, 6 doc, 1 readme, clippy, fmt, rustdoc. After: **200 / 209 / 517 + 109 / 210 / 541 + 109**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples
both ways; **render 21 of 21 WAVs byte-identical** to 112's, −21.5 LUFS, −7.5 dBFS TP, two spectrograms read; per-frame 0.064 – 0.065 / 0.296 – 0.343 / 1.044 – 1.217 µs at 8 / 64 / 256 voices, three runs on a shared machine at
load 2.4 (inside the band). **The Cauldron's panel test was not run**: the editor pins `82d227e`, ~216 commits back, and its 14 GB target does not fit.

**Verdicts.** Function **GREAT** (after five claims that outran their evidence), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `ee020d3`; comment on `#35`.

**Recorded, not fixed.** The example still prints *"1 assets"* beside *"4 failed"* for a folder of broken files (112). `mixer` plays the near-silent Ogg padding and **nobody has listened** to an Ogg, FLAC or MP3 through it. The classes are
the author's file names, grouped, and nobody has judged them. At least three bash calls in the last stretch read files with `sed`/`grep`/`ls` against the standing instruction (`review.md`).

**Top 5 remaining.**

1. **A hubness correction, measured with a bank held out from the evaluation** — on both libraries in both wordings, with the criterion written first: *some* threshold on the normalised score keeps every answerable description's top
   result and silences a real share of the unanswerable. The exploratory result (Kenney AUC 0.76 → 0.93) used the set's own descriptions as the bank and is transductive. The crate change (an index-time statistic, a public type) is the owner's.
2. **The default-floor decision** — `needs-decision/default-floor.md`: *keep `0.0`*; `0.20` is the only positive value the data supports and it is cosmetic. Owner's.
3. **The mixer path**: listen to Ogg, FLAC and MP3 through `MixerBackend`; decide whether it should cut the Ogg padding too (a `Completes` timing question); the upstream report in `runs/2026-11-03-112/upstream-symphonia-vorbis.md` is ready to file.
4. **Resampler fidelity at 44.1 kHz** (rubato against scipy and soxr, which is nearer what the model was trained on), a model-backed `update_from_model` so one new file costs one clip, and **a third library** of a different recording style to
   tell *"the answer is about game audio"* from *"the answer is about Kenney"*.
5. **Waiting on a person, another repository or hardware:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned ~216 commits back); `#39` (decided not to ship, ADR 0005); `#36`'s GPU figure; `find_compatible_*` speed;
   `needs-decision/headless-discovery.md`; iteration 105's `#[non_exhaustive]`; and the CI gap — nothing runs the benchmark, so both baselines are compared by whoever runs `goldenbench`.

## Iteration 112 — 2026-11-03 — two of five formats could not be decoded, and nothing had ever tried

**Change.** `library::AUDIO_EXTENSIONS` lists `wav ogg oga flac mp3` and calls them *"the formats the crate can decode"*; `rodio` was given
`wav` and `vorbis`. Every `.flac` and `.mp3` was scanned as audio and refused (*"The format of the data has not been recognized"*), in a crate
whose own error message says *"a library of `.mp3` files is indexed"* — **because no test had ever opened a file that was not a WAV.** Now:
`rodio` has `flac` and `mp3` (two pure-Rust crates: `--features semantic` 487 → **497** lines, 181 → **183** crates; default build unchanged at
413 / 149); `library::read_header` is fixed (a 48 kHz FLAC reported **768 000 Hz**; an Ogg's rate was read three bytes late); `semantic::decode`
**cuts an Ogg to the length its last page declares**; `examples/semantic` waits for the decoded length instead of a 16-bit-PCM estimate (a 10 s FLAC
was waited 1.7 s for, a 15 s MP3 5.8 s — "done" over a sound still playing); and `every_extension_the_scan_calls_audio_has_a_decoder` decodes a real
`ffmpeg`-made fixture of every entry on the list, so the list and the decoder cannot drift again. 8 tests (7 in `semantic`), 5 fixtures, 41 KB.

**Rationale.** Rank 1 beat the second golden library because both came from the same place — planning that library showed Kenney's Digital pack is
**62 MP3s** — and a refusal that should not happen outranks a better experiment. The second library is carved off, its source found: Kenney's CC0 packs
on OpenGameArt (kenney.nl's download API refuses scripts and was not circumvented), nine zips, MP3 and Ogg.

**Checked against decoders that are not this crate's, on real files, through the whole pipeline** (`formats_parity.py`: the crate on a file against the
crate on the same file decoded by the reference, so a difference is the decoder's alone). **FLAC**: ESC-50 as FLAC gives *identical* similarities to the
WAVs — all 232 000, largest difference 0.0. **MP3**: Kenney's 62 MP3s match `ffmpeg` on **259 of 259** pairs, every length exact. **Ogg**: 209 effects match
`libvorbis` to 1.8 × 10⁻⁷ — after one real defect: `symphonia` keeps the padding of the last Vorbis block, so **59 files came back up to 975 frames (20 ms)
long**, which on short effects moved a similarity by up to 0.085 and changed the top three of **all 12** descriptions tried (rank correlation 0.727); cut to
the declared length the worst difference is 0.004. A chained file is left alone, and the test goes red when that guard is weakened (I mutated it).

**A retraction, because I got it wrong first.** 22 of 51 files in one pack differed from `libvorbis` by up to 3.5 % rms. I called it *"wrong samples in a
minority of files"* in a doc comment, an upstream-report draft, two evidence file names and a pointer to a decision file I had not yet written. **My own
table gave it away**: the `peak` column was `1 + max|diff|` on every row — the output was pinned at 1.0 where `libvorbis` decoded to 1.3.
`symphonia-codec-vorbis/src/dsp.rs:136` is `*s = s.clamp(-1.0, 1.0)`; clamp `libvorbis` the same way and all 209 agree. A design choice, harmless, moved a
similarity by at most 0.004. Retracted everywhere. And my first control, `ffmpeg`, is up to 128 frames short of the stream's length in 8 files; the reference is `libvorbis`.

**The prediction I wrote down first was half wrong.** MP3 at 128 kbit/s: zero-shot ≤ 1 point (−0.3 / −0.6 measured ✓), a typical similarity < 0.01 (0.005 ✓; **0.033
at 64 kbit/s ✗**), hit@1 within ±2 points (**+4 and +8 ✗**, upward). **The golden set's baseline is the WAV encoding, and the encoding moves the answer by about as
much as four queries** — stated in the golden-set guide, the semantic guide and `#35`. Why hit@1 rose is an untested inference.

**The second finding came from reading my own repository.** Iteration 111 recommended a floor of **0.35**. Iteration 110's demo — fourteen macOS UI sounds,
five descriptions — scored **nothing above 0.256** (*"a short electronic beep"* → `Sosumi` 0.206; *"a glass being tapped"* → `Glass` 0.016): every positive floor
would have returned an empty list. `needs-decision/default-floor.md` now recommends keeping `0.0` until a second library of game audio is measured; the guide and two
doc comments say so; the default is unchanged. Also corrected: *"costs no new crate"* (`decode.rs`, `leaf.rs`, `library.rs`) was never true — `rodio` is in the default
graph and its decoders are not, **16 of the 34 crates `semantic` adds**.

**Evidence.** [`runs/2026-11-03-112/`](runs/2026-11-03-112/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`, `upstream-symphonia-vorbis.md` (drafted, **not filed**),
`matrix.sh` + `matrix.log`; `evidence/before/` (the example's wait on a FLAC and an MP3), `evidence/after/` (`formats_parity.py` and its seven outputs, `format_cost.py`,
four `goldenbench` runs × two wordings, `decode-three-way.txt`, `vorbis-clamp-explains-it.txt`, `dependency-graph.txt`, `edge-cases.txt`, the render set, `spectrograms/`).

**No clip:** no visible feature; the example's output changed by one line. Tests: `tests/semantic_decode.rs` (+6), `tests/library.rs` (+1), `tests/semantic_indexer.rs` (+1).

**Baseline:** green on arrival — 199 / 208 / 209 / 109 + 497 / 109 + 521 / 6 doc, clippy, fmt, rustdoc — and the disk guard **did not run** (it called `df -g`, which this
macOS rejects; fixed, in the skill). After: **200 / 209 / 505 semantic / 210 mixer / 529 all-features**, 6 doc, 1 readme, clippy `-D warnings`, fmt, rustdoc `-D warnings`, examples
both ways; **render 21 of 21 WAVs byte-identical** to 111's, −21.5 LUFS, −7.5 dBFS TP, two spectrograms read; per-frame 0.018 – 0.062 / 0.106 – 0.354 / 0.396 – 1.258 µs at 8 / 64 / 256
voices (inside the band). **The Cauldron's panel test was not run**: the editor pins `82d227e`, 215 commits back, and its 14 GB target does not fit.

**Verdicts.** Function **GREAT** (after three claims that outran their evidence), zoom **GREAT**, fresh eyes **GREAT**. **Commit** `c6b7600`.

**Recorded, not fixed.** The example prints *"1 assets"* and *"none of the 1 files could be indexed"* beside *"4 failed"* for a folder of broken files (the scan skips three by
header and counts one asset). `mixer` still plays the near-silent Ogg padding. **Nobody has listened** to an Ogg, FLAC or MP3 through the mixer.

**Top 5 remaining.**

1. **The second golden library** — Kenney's CC0 packs (UI, foley, sci-fi, casino, jingles; already fetched, MP3 and Ogg now decode): ~35 describable classes, two wordings,
   ~18 no-answer queries, `goldenbench --set <name>` and a separation metric (top score, top − library median). It is the evidence the floor decision is waiting on, and
   it tests the one question this iteration raised: **are a game's correct scores near 0.2, or near ESC-50's 0.47?**
2. **The default-floor decision** — `needs-decision/default-floor.md`: now *keep `0.0`* (revised); a floor relative to the library (option 6) if item 1 says the scales differ. Owner's.
3. **Resampler fidelity**, and a model-backed `update_from_model` so one new file costs one clip (carried).
4. **The mixer path**: listen to Ogg, FLAC and MP3 through `MixerBackend`; decide whether the backend should cut the Ogg padding too (a `Completes` timing question); the upstream
   report in `runs/2026-11-03-112/upstream-symphonia-vorbis.md` is ready for the owner to file.
5. **Waiting on a person, another repository or hardware:** `#35`'s anchor pairs; `#32`–`#34` the editor (pinned 215 commits back); `#39` (decided not to ship, ADR 0005);
   `#36`'s GPU figure; `find_compatible_*` speed; `needs-decision/headless-discovery.md`; iteration 105's `#[non_exhaustive]`; and the CI gap — nothing runs the benchmark or the export script.

## Iteration 111 — 2026-11-03 — the golden set, and the claim the first real measurement overturned

**Change.** `#35`'s part that a machine can do: **recall, precision and the similarity floor, measured on 2 000 recordings people
labelled.** `golden/esc50/` (a SHA-256 manifest of every file at a pinned commit, two files of 66 text queries each — the maintainer's
wording and a paraphrase that avoids the class names — and a recorded baseline); `tools/fetch_esc50.py` (the clips fetched 16 at a time,
each verified as it arrives: **79 s**; the `licences.csv` sidecar the crate already reads, failing unless every clip has a row);
`semantic::golden` (a loader that refuses a mistake **by line**); `semantic::eval` (set-valued relevance, `floor_curve`,
`score_bands` — additive); `examples/goldenbench` (the real model over all of it, with a baseline that refuses a different corpus,
model or question set); ADR 0009; `docs/guide/golden-set.md`; 19 tests.

**Rationale — the critical path, re-read.** Every open issue was read in full. `#32`–`#34` are the editor, built by `bevy_grimoire`'s own
loop (none is filed there with `consumer:bevy_siren`); `#39` was never "blocked on a decision" — **ADR 0005 (accepted) already decided it
does not ship** (MERT and MuQ are CC BY-NC), and the issue text had not caught up; `#35` is the first link of what is left (`#35` →
`#39`, `#45`'s real corpus), and its text half was buildable once iteration 110 made the real model run. The guide still opened with
*"recall against a labelled set is still the design defaults"*.

**The first real measurement overturned a documented conclusion.** The default floor (`0.0`) was justified as *"a measurement, not
timidity"*: six synthetic clips whose right and wrong bands overlapped, so *"no single threshold separates them"* and *"a floor of 0.5
rejects every correct match"*. On 2 000 real recordings the right answers' median is **0.468** against the wrong answers' **0.070**; the
bands overlap only in their tails; a floor on the top result separates a description the library can answer from one it cannot with an
**AUC of 0.963**. The six synthetic tones' *best* match (0.429) is below the real median — **a floor calibrated on synthetic audio was
calibrated on nothing.** The claim is corrected in `rank.rs`, `weights.rs`, `ensemble.rs`, the guide and `semantic-config.md`; the
default is **unchanged** and is now `needs-decision/default-floor.md`. Four tests that pinned the old story are deleted (tautologies on
literals; a default-pin whose stated reason — *"no labelled set exists yet"* — became false).

**And then my own headline was refuted by a second wording.** The first numbers — hit@1 90 %, hit@3 **100 %**, and a floor of **0.45**
as the knee — were measured on descriptions *I* wrote. The fresh-eyes pass asked the same 50 classes in a stranger's words (*"a furry
pet yapping at the door"*, *"ah-choo"*): **hit@1 72 %, hit@3 84 %**, the weakest top result falls 0.492 → 0.405, and **0.45 already
leaves two descriptions with no result**. The recommendation became **0.35**, and every "every description finds a right answer" in
the README, guide, CHANGELOG and five doc comments now says in whose words. Had the second wording not been asked for, the friendly
figures would have shipped.

**Proved against the reference.** Zero-shot accuracy **87.8 %** (the paper: 88.0 – 91.0 % on an overlap-excluded subset — I had predicted
"at or above"; it is a little under) — and an independent `transformers` run on the same **44.1 kHz** audio, resampled by scipy, gets
**87.8 % too**. 232 000 similarities: mean difference 8 × 10⁻⁴, 99.9th percentile 0.03, **max 0.089** (a resampler against a
resampler; not digital silence, Spearman −0.21; which is nearer the model's own pipeline is unmeasured). A Python implementation of
every metric agrees with the Rust one to the printed precision. On the real sidecar the crate's provenance reads **1 770 licences that
permit commercial use, 170 that do not, 60 unknown** — unknown never shown as permissive.

**Found by running it.** The first download route (GitHub's archive) streamed at ~100 KB/s — two hours; killed after 30 minutes; the
per-file route took 79 s. The dataset's own `LICENSE` has two quirks (`5-141683-Ad`, `[DrinkingCan[Norberd]](url)`) that the fetch's
"every clip has a licence row" rule refused and now names. The baseline compared a different question set as a regression until its
identity included the queries. A blind line-number edit broke a script I then repaired from a read. And `semantic/mod.rs`'s stage table
said *"index and query: not yet"* — and its links resolved from the crate root, so `model` pointed at the wrong module.

**Cost, corrected.** 2 000 five-second 44.1 kHz recordings index at **105 ms a clip** (9.5 clips/s, under load) — iteration 110's *five
minutes for 5 000* was for one-second clips; the five-second figure is about nine. The extra 45 ms is not decomposed.

**Not met, and not a machine's.** `#35`'s third criterion — anchor pairs judged by a person, with a reason. The guide says how to add
them; the loader and the metric are deliberately not built (`eval::agreement` cannot score an anchor queried with itself), so they are
decided with the first real judgements in hand. ESC-50 overlaps the model's training data, is one library of environmental sounds,
and has no music; the guide's "what it cannot say" is the first thing to read.

**Evidence.** [`runs/2026-11-03-111/`](runs/2026-11-03-111/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`;
`evidence/after/` (`goldenbench-first-run.txt`, `goldenbench-run.txt`, `goldenbench-paraphrase.txt`, `goldenbench-edge-cases.txt`,
`golden-parity.txt` + `golden_parity.py`, `floor-analysis*.txt` + `floor_analysis.py`, `fetch-repair.txt`, the render set),
`evidence/spectrograms/`.

**No clip:** the program prints text; `docs/media/golden.png` is its verbatim output drawn (commands in `docs/media/README.md`). Tests:
`tests/semantic_golden.rs` (19).

**Baseline:** red on **disk** — 4.6 GB free; `cargo clean -p bevy_siren` (this crate's artifacts, all built this session; 765 files,
11.0 GiB) → 14 GB; twice more mid-iteration as the matrix filled it (once to 662 MB). Otherwise green: 199 / 208 / 209 / 109 + 482 / 109 + 506
/ 6 doc. After: 199 / 208 / 209 / 109 lib + **497** semantic / **521** all-features / 6 doc; clippy, fmt, rustdoc clean; **render 21 of
21 WAVs byte-identical** to 110's, −21.5 LUFS, −7.5 dBFS TP; per-frame 0.059 – 0.074 / 0.308 – 0.359 / 0.436 – 1.320 µs at 8 / 64 / 256
voices under a load of 6 (one 8-voice sample 6 % above the old band; no claim). **The Cauldron's panel test was not run**: no disk for its
14 GB target, and nothing the panel reads changed.

**Verdicts.** Function **GREAT** (after four claims that outran their evidence), zoom **GREAT** (after three fixes), fresh eyes **GREAT**
(after three, one of which rewrote the headline). **Commit** `9d9a67e`.

**Top 5 remaining.**

1. **The default-floor decision** — `needs-decision/default-floor.md`: keep `0.0`, or `0.35` (recommended), with the table for both
   wordings. Waits on the owner; blocks nothing.
2. **A second golden library** — the guide's "most useful next addition": game foley, music, speech. Every figure here is about ESC-50 until
   one exists, and a per-library floor (option 6) needs it to be designed against. **A person's** second and third wording of the same
   descriptions are the cheapest probe of all.
3. **Resampler fidelity.** The crate's `rubato` and scipy's polyphase filter disagree by up to 0.03 mean cosine on ~1 % of clips; which is
   nearer `librosa`'s `soxr` (what the model trained on) decides whether the crate has a small defect, and `goldenbench`'s baseline is
   now the instrument that would show a fix. `pip install soxr` is the cheapest first step.
4. **A clip's cost.** 105 ms for five seconds against 60 ms for one: decompose the extra 45 ms and the 26 ms mel stage, and a model-backed
   `update_from_model` so one new file costs one clip, not 5 000.
5. **Waiting on a person, another repository or hardware:** `#35`'s anchor pairs; `#32`–`#34` the editor (`bevy_grimoire`'s loop); `#39`
   (decided not to ship, ADR 0005 — commented, left open for the owner to close); `#36`'s GPU figure; `find_compatible_*` speed; the
   cross-process audition hop; `needs-decision/headless-discovery.md`; iteration 105's `#[non_exhaustive]`; and the CI gap — nothing runs
   the benchmark or the export script.

## Iteration 110 — 2026-11-03 — the first real run, and what 109 tests could not say

**Change.** `#38`: **`examples/semantic`** — index a folder, describe a sound, read the top results with the reason for each, and
hear one through `SirenPlugin`'s own `Play` — and **`tools/export_clap_onnx.py`**, which makes the two graphs, the tokenizer and an
export-version line from the weights in a Hugging Face cache, checking each wrapper against the model before it writes.
`ModelStamp::clap(version)` and `--model-version` (on `siren-tools` and the example) let an index name the export that made it.
`docs/media/semantic.png` is the example's verbatim output, drawn.

**Rationale — the weights were on this Mac the whole time.** Iteration 109's closing said a real run *"cannot [be had] on this
Mac: that needs the CLAP audio and text graphs"* and that `search_music` *"has run only against a 16 KB fixture; the 501 MB graph is
not on this Mac"*. Both were read off a directory listing, not from looking for the weights: `~/.cache/huggingface` holds
`laion/clap-htsat-fused`, and torch, transformers and onnx are in the system Python. The export takes **6 s**. So `#38` was never
waiting on a GPU host or on `#33`, and item 3 of that list came free with it.

**The first real run found what 109 tests could not.** (1) **0 of 14 files indexed** — ORT: `Reshape_2` … *"Input shape
{0,96,64,3,21}, requested {0,96,64,-1}"*. HF's extractor marks one clip of every fused batch `is_longer = True`, even a lone
one-second clip, which switches on HTSAT's local-mel fusion branch; the trace baked that branch in, and the crate sends `false`
(it chunks past ten seconds), which ran it on an empty batch. `get_audio_features` and the path the crate runs differ by cosine
**0.72–0.83**. The script now traces `is_longer = false` and keeps it a real input — the exporter drops an unused one, which was
a second failure (*"Invalid input name: is_longer"*). (2) **The example had written an empty index** and then answered *"nothing
matched"*, blaming the description for a graph that could not run; it now refuses to write one.

**Proved against the reference.** 5 phrases × 14 real files, `transformers` computed pair by pair: largest difference
**4.89 × 10⁻⁴** (the example prints three decimals), the same ranking order on every phrase (`evidence/after/parity.txt`). A real
played run: `Played audition.89f4c994c2850cba on the Music bus at +0.0 dB, variant Sosumi.wav`, then the voice ended and the run
exited 0. `siren-tools` driven with the real text graph: start-up **0.9 s**, `search_music` **4.6–5.6 ms**, the first hit's score
the example's to eight digits.

**Two documented claims the run falsified.** The README said *"a 5 000-clip library costs seconds, not minutes"*; indexing 280 clips
with the real tower is **16.6 clips/s — 60 ms a clip — so about five minutes for 5 000**. Of the 60 ms, **35 ms is the tower and 26 ms
is this crate's own mel spectrogram**: I had written *"the tower is nearly all of it"* before timing it. And the README and the
guide said `cargo tree` reads *"the same 413 lines"* with and without `semantic`; it is 413 and **487**. Both are corrected where
they were said.

**Fresh eyes found more.** `--k 101` printed *"A k of 0 asks for nothing"* — `ToolProblem::BadK`, the message an agent over
`siren-tools` reads — fixed, with a test that **fails on the old message**. An empty description was refused only after the 501 MB
text graph had loaded; it is now checked first. The script's docstring called `onnx` optional, and the other Python on this Mac
disproved it (*"Module onnx is not installed!"*).

**Recorded, not fixed.** The example **rebuilds whole** when the folder changes: `incremental::update` takes a closure and the
library has no model-backed form of it. The mel stage is 42 % of a clip and runs in series with the tower. A GPU figure (`#36`):
no GPU here. The script has run in **one** environment (Python 3.9, torch 2.8.0, transformers 4.57.6, onnx 1.19.1) and not on
transformers 5.x. **No test runs the example or the script** — CI has no model — so a regression is caught by the next person who
runs them. A person has not heard the played result; the evidence is `Played` and a voice that ended with no *"no audio device"*
warning.

**Assumed decision (`#38`, second criterion).** *"No GPU prints what is missing and exits non-zero"* is read as *a run that cannot
work says why and exits non-zero*: a missing model is the failure. This build has no CUDA path (`Device::available()` is `Cpu`,
unconditionally), so a no-GPU error would be every run; the first line says the run is CPU fp32.

**Evidence.** [`runs/2026-11-03-110/`](runs/2026-11-03-110/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`;
`evidence/after/` (the render set, `refusal-no-model.txt`, `first-real-run.txt`, `parity.txt`, `parity.py`, `play-real.txt`,
`edge-cases.txt`, `siren-tools-real-model.txt`, `drive_siren_tools.py`, `throughput-280.txt`), `evidence/before/`,
`evidence/spectrograms/`.

**No clip:** the program prints text, and the still is that transcript drawn (`docs/media/semantic.png`, regenerated by the commands
in `docs/media/README.md`). Tests: `a_stamp_names_the_export_that_made_the_vectors`,
`an_index_from_another_export_is_served_once_its_version_is_given` (a real `siren-tools` process) and
`a_bad_k_is_explained_by_its_own_case`.

**Baseline:** red on **disk** — 4.6 GB free, the skill's first rank; `cargo clean -p bevy_siren` (1 329 files, 15.0 GiB, provenance
checked: nothing older than today) → 18 GB. Then green: 199 / 208 / 209 / 109 + 479 / 109 + 503 / 6 doc. After: 199 / 208 / 209 /
109 lib + **482** semantic / **506** all-features / 6 doc; clippy, fmt and rustdoc clean. **Render: 21 of 21 WAVs byte-identical** to
109's, −21.5 LUFS, −7.5 dBFS true peak. Per-frame cost 0.018–0.063 / 0.114–0.364 / 0.408–0.765 µs at 8 / 64 / 256 voices, inside the
recorded bands. **The Cauldron's panel test was not run**: the skill's 14 GB target directory does not fit in the 3.9 GB left, and
nothing the panel reads changed.

**Verdicts.** Function **GREAT** (after four claims that had outrun their evidence were fixed), zoom **GREAT**, fresh eyes **GREAT**
(after three fixes). **Commit** `111a900`.

**Top 5 remaining.**

1. **A model-backed incremental update** — `update_from_model`, the filled seam beside `build_from_model` (additive). At 60 ms a clip,
   one new file in a 5 000-clip library costs five minutes through the example and 60 ms through `incremental::update` with a
   closure; the example and any host would use it.
2. **The mel stage: 26 ms of a 60 ms clip, in series with the tower.** Measure first whether it can be cheaper at the same 0.0545 dB
   agreement, or overlapped with the tower, and whether `fusion_channels` computes four spectrograms for a window the graph reads
   one of. It is the cheapest cut to a five-minute index.
3. **`needs-decision/headless-discovery.md`** — a model-free `siren-tools` cannot be started by an agent; the recommended bounded
   `list_assets` is a public-surface change, so it waits for the owner.
4. **Waiting on a person, another repository or hardware:** `#35` (a person judges at least 50 pairs and writes 20 queries),
   `#32`/`#33`/`#34` (the editor panel, in `bevy_grimoire`), `#39` (a music encoder and its licence audit), and `#36`'s GPU figure.
5. `find_compatible_*` speed (18 ms at 10 000 assets against a 10 ms budget), the cross-process audition hop (ADR 0006's step 3),
   iteration 105's `#[non_exhaustive]` decision, and **the CI gap** — nothing runs `examples/semantic` or the export script.

## Iteration 109 — 2026-11-03 — the tool host, which was never blocked

**Change.** `#28`: **`siren-tools`**, an MCP server over stdio, in this crate behind `semantic` — `semantic::server` (the host,
as a library, so a test drives it with no process), `src/bin/siren-tools.rs` (the loop around it), `serde_json` behind
`semantic` with its `leaf.rs` row, and **six tools listed** (ADR 0007): five wired, `audition` `unavailable` as the ADR says.
**One table, `SPECS`, draws the listing, the schema and the validation**, so what a client reads and what its call meets cannot
disagree. A bad call is a **tool result a model can read** (`isError: true`, ADR 0007's `{code, message, retryable}`), not a
protocol error it never sees; a notification is never answered or executed; a line over 1 MiB is drained so the stream stays
aligned. ADR 0008 records the transport and every place ADR 0007 was silent.

**Rationale — and the mistake that made it wait seven iterations.** Iteration 108's closing found that ADR 0006 (**accepted**)
already decided *"one `[[bin]]` behind `semantic`, a workspace rejected"*, and that my `needs-decision/mcp-tool-surface.md`
(iteration 102) re-asked it, recommended the rejected option, and listed `#28`–`#31` as blocked. `needs-decision/README.md` says
only the public API, the message types and the definitions-file schema belong there; a feature-gated binary is none of the
three, and its dependency is a `leaf.rs` row the ADR says to argue *"like any other"*. So this iteration built it. The
iterations between were real work — 103–105 found real defects — and then three drift guards, while the one open issue with a
clear path sat unbuilt.

**The one real decision: hand-written stdio, not `rmcp`.** `bevy_grimoire`'s server uses `rmcp` + `tokio` + `schemars`, the
org's convention, and I looked at it first. Measured with two probe crates: **`serde_json` alone is 6 crates, that stack is
58.** The work here is synchronous and the protocol is five methods, so a runtime would be a thread pool around a loop that
reads a line and writes a line. The cost is owning the framing, which is why it is tested against the specification's own rules,
against 13 hostile lines, and by a client that is not mine.

**Proved by breaking it.** Schema loosened to 101 while the check stops at 100 → `search_music.k refused its own maximum`.
A handler arm renamed → `SPECS and its handler disagree`. The handler ignoring `weighting` → *"under the weighting that ranked
it, the explanation is the result"*. And one that fired unasked: `weights_json` first compiled without `Weights::compatibility`
and the exhaustive destructure refused it (**`E0027`**) — a field my own grep had missed.

**The first real session found a defect no test had.** Reading `evidence/before/session.txt`, the transcript of the real binary:
`find_compatible_sfx` scored a pair with `acoustic` at **1.2** and `explain_match` of the **same pair** said **0.5** — the
library's explanation was hard-wired to the music weights, so an effect's explanation did not add up to its own score. Fixed:
`Toolset::explain_match_with(…, &Weights)` and an optional `weighting: "music" | "effects"` on the tool. The same read found
*"1 near-identical variants"* (fixed) and, in the process test, that **`indexer::INDEX_FILE` is `siren-index`, not the
`siren-index.txt` two docs said** (fixed).

**`#28`'s pitfall, answered.** `examples/hostbench`: **34 ms and 20.5 MB to start** at 10 000 assets, then 0.02 ms for
`set_anchor` and `explain_match`, 5.3 ms for a search before the encode, **18 ms** for `find_compatible_*` — over the 10 ms
budget `search_ranked` meets, because it scores every candidate before truncating. **A long-lived server is the expectation.**
Not measured: the 501 MB graph, which is not on this machine.

**Interop.** The **official MCP Inspector (TypeScript SDK)** connected over stdio to the real binary, listed the six tools and
called three: `evidence/after/interop.txt`. One third-party client is not every client, and the review says so.

**Before/after.** No audio changed: **all 21 rendered wavs byte-identical** to iteration 108's, master **−21.5 LUFS, −7.5 dBFS
true peak**. `cargo tree` default **413 lines / 149 crates, unchanged**; with `semantic` 482 → 487 lines, 177 → 181 crates.
Per-frame cost 0.059–0.063 / 0.323–0.444 / 0.543–1.129 µs at 8 / 64 / 256 voices on a load average of 13–15 — inside the
recorded bands but for one 64-voice sample at 0.444; no claim.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. The zoom read `footstep-grass.png` (a 27 ms broadband
attack ending on a sharp edge near −90 dBFS, three sustained partials, a −90 dBFS tail, no click or hole) and the whole wire
transcript, before and after. **Fresh eyes found a design gap the tests cannot see because every test supplies the ids:** a
server with no model has no tool that returns an `asset_id`, so an agent cannot start. Filed as
`needs-decision/headless-discovery.md` (a seventh tool is a public-surface change) with a recommendation.

**Recorded, not fixed.** `find_compatible_*` at 18 ms (exact speedups probably exist and were not tried); ADR 0007's stale
*"cannot be built yet"* paragraph (corrected); `AGENTS.md`' four-name dependency list (corrected to `ALLOWED_DEPS`).

**Evidence.** [`runs/2026-11-03-109/`](runs/2026-11-03-109/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/after/` (the render set, `session.txt`, `interop.txt`), `evidence/before/session.txt`, `evidence/spectrograms/`.

**No clip:** the change is a protocol, and its picture is the transcript. The tests are `tests/semantic_server.rs` (41) and
`tests/semantic_tools.rs`.

**Baseline:** nothing red — 199 / 208 serde / 209 mixer / 109 lib + **436 semantic** / **460 all-features** / 6 doc. After:
**479 semantic / 503 all-features**; clippy and fmt each caught one thing of mine, fixed.

**Top 5 remaining.** *(Each open issue was re-read this time rather than trusted from the old summary — the lesson of `#28`.)*

1. **`#38` the playable example — buildable here, and wrongly filed as waiting on the editor.** Its *"blocked by `#33`"* is an
   ordering in the issue graph, not a technical dependency: everything in its scope (`examples/semantic.rs`, behind `semantic`;
   no-model run that says what is missing and exits non-zero; an entry in `docs/media/README.md`) is this crate's. What it
   cannot have on this Mac is a **real run**: that needs the CLAP **audio and text** graphs, and `tests/fixtures/` holds only the
   text one. So the plumbing is verifiable here (an index crafted from the fixture model, a synthesised library, the
   no-model refusal) and the README picture waits on a machine that has the graphs.
2. **`needs-decision/headless-discovery.md`** — a model-free `siren-tools` cannot be started by an agent. Option 1, a bounded
   `list_assets(category?, k)`, is recommended; it is a public-surface change, so it waits for the owner.
3. **`search_music` against the real CLAP graph.** It has run only against a 16 KB fixture; the 501 MB graph is not on this Mac
   and `big` has no checkout. That run also gives the model-load and encode figures `hostbench` could not.
4. **Genuinely waiting on a person, another repository, or a model I do not have:** `#35` (a person judges at least 50 pairs and
   writes 20 queries), `#32`/`#33`/`#34` (the editor panel, in `bevy_grimoire`), `#39` (a music encoder and its licence audit).
5. **`find_compatible_*` speed** (18 ms at 10 000 assets against a 10 ms budget; exact speedups not tried), **the cross-process
   audition hop** (ADR 0006's step 3), and iteration 105's **`#[non_exhaustive]`** decision.

## Iteration 108 — 2026-11-03 — a guard that fired, and a variant that got through anyway

**Change.** `problems_names_each_mistake` — the test whose job is *every problem a table can have is named, in a sentence* —
produced six of `CueProblem`'s seven variants and stayed green. `InertAvoidRepeat` (iteration 105) is tested on its own
(`tests/play.rs`, `tests/model.rs`), but no fixture cue here produced it, so it never went through the *"is a sentence, not a
debug print"* loop. `tests/table.rs` now lists every variant **once**, in a `kinds!` macro that generates the exhaustive
`match` and the count together, and fails — `no fixture cue produces ["InertAvoidRepeat"]` — until a fixture cue produces
each listed kind. **No change to `src/`.**

**Rationale.** Iteration 107's recorded follow-up, the third of three lists that can drift (106 settings, 107 music, 108
cues). This one returns an enum rather than `Vec<String>`, so no `const` was needed — but an exhaustive `match` turned out
not to be the guard either.

**My first design was wrong, and I found out by reading `git diff`.** I wrote *"no guard exists"*, added a second exhaustive
`match`, added a tenth variant, saw `E0004`, and counted it as proof. `HEAD`'s `named` closure already listed all seven
variants with no wildcard, so `HEAD` fails to compile identically — and had **already fired in iteration 105**, which is the
`E0004` quoted in `needs-decision/diagnostic-enums-are-extensible.md`. **That firing is how the gap happened:** the compiler
demanded an arm, I wrote the arm, nothing demanded a fixture. *An exhaustive `match` forces acknowledgement, not coverage.*
The closure was also vacuous (implied by `problems.contains` one line above) and its comment was false (it said it did not
enumerate the variants). It is deleted, and iteration 105's decision file, which called the test "fixed to match the
documented property", now says it was given an arm.

**Proved the guard fails, on the real state.** A variant added and unlisted → `E0004`. Added and listed with no fixture →
`no fixture cue produces ["Tenth"]` (`HEAD` passes this). And iteration 105's own state replayed on the real variant — listed,
its cue and row removed → `no fixture cue produces ["InertAvoidRepeat"]`. All reverted; `src/cue.rs` and `tests/table.rs`
returned to their original content hashes.

**Before/after.** No audio changed: all **21 rendered wavs byte-identical** to iteration 107's, master **−21.5 LUFS, −7.5
dBFS true peak**. Per-frame cost 0.063 / 0.355 / 1.218 µs at 8 / 64 / 256 voices on a load average of 3.71 — inside the
recorded band; no claim.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. The zoom read `footstep-stone.png`: a 27 ms broadband
attack ending on a sharp edge near −90 dBFS (about 40 dB under the partials beside it), three sustained low partials decaying
to a −95 dBFS tail, no click, shelf or hole.

**Found while closing — the more important finding: `#28`'s MCP server is not blocked, and never was.** ADR 0006, **accepted**,
decides *"one library crate, one `[[bin]]` behind the `semantic` feature … a workspace was considered and rejected"*, and
`#28`'s own scope says *"a server binary behind the `semantic` feature"*. My `needs-decision/mcp-tool-surface.md` (iteration
102) re-asked that question, **recommended the option the ADR had rejected**, and listed `#28`–`#31` as blocked on it.
`needs-decision/README.md` says only the public API, the message types and the definitions-file schema belong there; a
feature-gated binary is none of the three, and its dependency is a `leaf.rs` row the ADR says to argue *"like any other"*.
The file now carries a banner saying so. Iterations 103–105 found real defects; 106–108 were drift guards — while the one open
issue with a clear path sat unbuilt. Iteration 109 builds it.

**Evidence.** [`runs/2026-11-03-108/`](runs/2026-11-03-108/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/after/` (32 files), `evidence/spectrograms/`.

**No clip:** a test-only change, nothing to see or hear. Its test is `tests/table.rs::problems_names_each_mistake`.

**Baseline:** nothing red — 199 / 208 serde / 209 mixer / 109 lib + **436 semantic** / **460 all-features** / 6 doc, clippy,
fmt and docs clean.

**Top 5 remaining.**

1. **`#28` the MCP server — iteration 109, unblocked.** `src/bin/siren-tools.rs` behind `semantic` per ADR 0006, one
   `serde_json` row in `leaf.rs` argued the way the ADR says, a stdio JSON-RPC host that lists tools, validates arguments
   and returns typed errors. Everything it wraps exists and is tested.
2. **`#29`/`#30`/`#31` wire forms** — follow `#28`; their library halves are done.
3. **ADR 0007's "the server cannot be built in this crate yet" paragraph** is now false and needs correcting with `#28`.
4. **`#35` golden set** (labelled real recordings), **`#32`–`#34`/`#38`** (the editor, in `bevy_grimoire`), **`#39`** (a music
   encoder and a licence audit) — each waits on a person, another repository, or an asset.
5. **`#[non_exhaustive]` on the print-only diagnostic enums** — iteration 105's decision file, now with the one cost it missed.

## Iteration 107 — 2026-11-03 — the same list, in the other score

**Change.** `MusicScore::problems()`'s doc said *"Nine checks, and this list is the whole of them"*. The nine were
correct — **by hand**, checked only when someone counted. Measured: adding a tenth check to `problems` left the suite green
and the sentence still claiming completeness. `music::MUSIC_CHECKS` is now the one list; the doc names it and
`a_score_reports_every_fault_its_doc_lists` walks it.

**Rationale.** Iteration 106's recorded criticism, and the same **Information Leakage** (Ousterhout): the set of checks is
one decision written in the doc, the bullets and the test, with nothing forcing agreement.

**Proved the guard fails.** Adding a tenth check **and its name** gives `9 != 10`, naming the whole list. Reverted; green.

**Iteration 106's criticism was half wrong, and the record now says so.** It claimed this type was *"one inspection away
from the same drift"*. There **was** a test, with nine needles, whose own doc records that the list once said four while the
code performed nine. The gap was narrower — the counts agreed, nothing kept them agreeing — and a criticism that overstates
misleads as much as a doc that does.

**A measurement of mine was wrong, and it is the most useful thing here.** My first attempt added the tenth **check** without
a tenth **name**, the test passed, and I wrote that the guard had failed. It had not: the constant *is* the list, so ten
checks and nine names is a state the constant does not describe. **A check not in `MUSIC_CHECKS` is not a documented
check.** The research file records the mistake in its own section rather than quietly fixing the sentence.

**Before/after.** No audio changed — **all 21 rendered wavs byte-identical**, master **−21.5 LUFS, −7.5 dBFS true peak**.
Per-frame cost unchanged and inside the band iteration 102 recorded; the change adds a `const` and a test comparison.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. The zoom read `music-calm.png`: a 1.55 s looping
bed, flat sustained partials to the end with **no decay** — correct for a loop, the one clip shape where a missing decay is
right — and no vertical seam at the wrap point.

**Recorded, not fixed.** `CueTable::problems` is the third of three with this shape and still has no constant. Two
iterations have done this pattern one type at a time; the third is mechanical and remains the follow-up.

**Evidence.** [`runs/2026-11-03-107/`](runs/2026-11-03-107/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red — 199 / 6 / **436 semantic** / **460 all-features** / 109 lib / 6 doc, clippy, fmt and docs
clean.

**Top 5 remaining — all blocked outside this repository.**

1. **`#28` the MCP server** — one line of `Cargo.toml` and one `leaf.rs` row. Everything it wraps exists and is tested.
2. **`CueTable::problems` gets the same constant** — the third of three, this iteration's recorded follow-up.
3. **`#35` golden set** — labelled real recordings; also what `#23`'s reverb measure and `evalbench`'s `corpus()` wait on.
4. **`#32`–`#34` the editor panel** — everything it needs is built, in `bevy_grimoire`.
5. **`#[non_exhaustive]` on the print-only diagnostic enums** — iteration 105's decision file.

## Iteration 106 — 2026-11-03 — a guarantee the test did not provide

**Change.** `SirenSettings::problems()`'s doc said *"**Twelve checks**, and `tests/mix.rs::problems_names_every_mistake`
walks all twelve so this list and the code cannot drift apart"*. Measured by counting:

| | count |
|---|---|
| checks the code performs | **13** |
| needles the test asserts | **11** |
| checks the prose claims | **12** |

The check missing from **every** list is the `by_db == 0.0` one — whose own comment records that it was written for a
**measured** bug. So the doc's central claim, *"this list and the code cannot drift apart"*, was false in the direction
that matters: a maintainer adding a check would have read it, added a sentence, and believed something enforced the
pairing.

**Rationale.** Ousterhout, *A Philosophy of Software Design*, Summary of Red Flags: **Information Leakage** — *"a design
decision is reflected in multiple modules"*. One decision (how many checks there are) was written in three places, and the
third copy **asserted the other two agreed**.

**The fix.** `mix::SETTINGS_CHECKS` — thirteen names, the only list. The doc **points at it** rather than restating it, so
the prose cannot be wrong. `problems_names_every_mistake` asserts `needles.len() + ASSERTED_ELSEWHERE.len() ==
SETTINGS_CHECKS.len()` and names any check with a fixture elsewhere — because one check (a bus asking for more voices than
the table allows) is *hidden* by `max_voices: 0` in that fixture, as its own comment says. `ASSERTED_ELSEWHERE` is itself
asserted to contain real checks, so a deleted check cannot leave a stale exemption.

**Proved the guard fails.** Appended a fourteenth name: the test failed naming the whole list. Reverted; green.

**A second find on the way.** The fixture had **no duck at `by_db: 0.0`**, so the check the drift hid was not exercised by
the test the doc pointed at at all. Adding `Duck::new("Music","Effects").by_db(0.0).hold_s(0.25)` closed that.

**Before/after.** No audio changed — **all 21 rendered wavs byte-identical**, master **−21.5 LUFS, −7.5 dBFS true peak**.
Per-frame cost unchanged and inside the recorded band; the change adds a `const` and a test comparison, and nothing on the
mix path.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: `MusicScore::problems`
claims the same shape (*"Nine checks, and this list is the whole of it"*) and is one inspection from the same drift —
mechanical, carved off so the finding stays legible. Also recorded: one gate image (`bus-music-wave.png`) is **blank and
useless**, because `bus-*.wav` is a *gain over time* — a constant — and `showwavespic` draws nothing for it. Kept and named
rather than deleted, because a blank image that looks like a finding is worse than no image.

**Evidence.** [`runs/2026-11-03-106/`](runs/2026-11-03-106/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red — 199 / 6 / **436 semantic** / **460 all-features** / 109 lib / 6 doc, clippy, fmt and docs
clean.

**Top 5 remaining — all blocked outside this repository.**

1. **`#28` the MCP server** — one line of `Cargo.toml` and one `leaf.rs` row. Everything it wraps exists and is tested.
2. **`#35` golden set** — labelled real recordings; also what `#23`'s reverb measure and `evalbench`'s `corpus()` wait on.
3. **`MusicScore::problems` gets the same treatment** — this iteration's recorded criticism.
4. **`#32`–`#34` the editor panel** — everything it needs is built, in `bevy_grimoire`.
5. **`#[non_exhaustive]` on the print-only diagnostic enums** — iteration 105's decision file.

## Iteration 105 — 2026-11-03 — a repeat rule that refuses nothing

**Change.** `Repeat::AvoidRepeat { avoid: 0 }` refused **nothing**. `Repeat::avoid()` returns the field verbatim and
`VariantPicker::pick_from` refuses `min(avoid, len)` recent sounds, so a window of zero is **byte-identical to
`Shuffle`** — while the variant's own doc promised *"never one of the last `avoid` variants … kills the machine-gun
footstep"*. `CueProblem::InertAvoidRepeat` now reports it.

**How it was found.** Reading `Repeat::avoid()` beside `pick_from`'s refusal arithmetic. Fourth iteration in a row where a
bug surfaced from comparing a documented claim with the code rather than from a failing test.

**Why it ranks first.** `cues.yaml` case 1 is *"variants do not repeat back to back"*. A cue written `avoid: 0` fails it
**while looking perfectly normal**: the play succeeds, no refusal is raised, and only a listener hears the machine gun. A
generated table computing the window from a variant count (`avoid: variants - 1`) reaches it mechanically.

**Proved both ways.** `a_zero_avoid_window_refuses_exactly_nothing`: two pickers, one seed, 24 draws — the `avoid: 0` and
`Shuffle` sequences are **equal**, and the `avoid: 0` sequence **contains** a back-to-back repeat while `avoid: 1` contains
none. `avoid_repeat_zero_is_reported_because_it_refuses_nothing`: reported for `0`; **not** for `1`, `2`, `4`, `255`, nor
for `Shuffle`.

**Reported, not repaired.** Coercing `0` to `1` would change a legal table's behaviour silently — the same failure the
defect already is. A `problems()` sentence is the crate's existing tool for *legal but almost certainly not what you
meant*, as `UnlimitedInstances` already is, and a panel that already shows `problems()` displays this with no UI work.

**Before/after.** No audio changed — **all 21 rendered wavs byte-identical**, master **−21.5 LUFS, −7.5 dBFS true peak**.
Per-frame cost unchanged and within the band iteration 102 recorded; the change is one `matches!` per cue at table load.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Two self-inflicted lessons recorded rather than
hidden: adding the variant **broke `tests/table.rs`'s exhaustive `match`** (a test about sentences, not about the set of
kinds — now reads the name instead), and I first asserted an `InertAvoidRepeat` row in a fixture that has no `avoid: 0`
cue, which failed under `--all-features`.

**Filed.** `needs-decision/diagnostic-enums-are-extensible.md`. This is the **fourth** diagnostic enum here to make a
consumer pay for a new variant, so it proposes `#[non_exhaustive]` for the print-only enums, keeps `Refusal` closed because
a game branches on it, and writes the rule down: *an enum a consumer branches on is closed; an enum a consumer prints is
open*.

**Evidence.** [`runs/2026-11-03-105/`](runs/2026-11-03-105/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red — 198 / 6 / **436 semantic** / **109 lib** / 460 all-features / 6 doc, clippy, fmt and docs
clean.

**Top 5 remaining — all blocked outside this repository.**

1. **`#28` the MCP server** — one line of `Cargo.toml` and one `leaf.rs` row. Everything it wraps exists and is tested.
2. **`#35` golden set** — labelled real recordings; also what `#23`'s reverb measure and `evalbench`'s `corpus()` wait on.
3. **`#[non_exhaustive]` on the print-only diagnostic enums** — this iteration's decision file; four enums, one break.
4. **`#32`–`#34` the editor panel** — everything it needs is built, in `bevy_grimoire`.
5. **The `Repeat` surface generally** — `Sequential` does not exist; a fixed-order variant list is a rule a table cannot
   currently express.

## Iteration 104 — 2026-11-03 — a floor that could not be set, a category that never matched (#43)

**Change.** Three defects in `semantic::tools` and `semantic::weights`, all in code this loop wrote and **closed issues
against**. Found by reading, not by a failing test — the third iteration running where that is how a bug surfaced.

1. **The similarity floor was ignored.** `tools.rs` compared against `Floor::default().min_score`, which is `0.0`, so the
   branch could never fire: `Tally::below_floor` was documented, rendered by `describe()`, and **permanently zero**, and a
   caller had no way to set a stricter filter. `Weights::min_similarity` is now a field (**design default `0.0`**,
   `Floor::min_score`'s existing default — and `0.0` because the measured semantic bands overlap, so a floor of 0.5 would
   reject every correct match).
2. **The category filter was case-broken.** `source_of` lowercased the haystack; `contains(want)` compared a **raw**
   needle. `Some("Footsteps")` against a pack recorded as `Footsteps` matched **nothing**, and the tally blamed the
   category. **My own iteration-97 test passed because it happened to use the lowercase form** — a test that passed for a
   reason adjacent to its claim.
3. **A bad compatibility multiplier named the wrong field.** Three checks pushed `BadDropFraction`, so `clash_penalty: 1.5`
   produced *"`drop_fraction` is 1.5"* — sending a person to edit a field they never touched. `BadMultiplier { field, value }`
   now carries it.

**Source.** Sun, Min, Chang & Bisk (`10.48550/arXiv.2406.19228`) name both halves of this class: **failure detection**
(*"recognizing failures"*) and **fault assignment** (*"identifying which tool caused the error"*). All three defects fail
the second — the tally said the acoustic rules excluded nothing while the category removed everything.

**Before/after.** No audio changed — **all 21 rendered wavs byte-identical**, master **−21.5 LUFS, −7.5 dBFS true peak**.
Five new tests: four spellings of one category all match (`Footsteps`, `footsteps`, `FOOTSTEPS`, `ootste`); a raised floor
drops and **counts**; the default keeps everything; a floor beyond a cosine is refused; both multiplier fields name
themselves and **not** `drop_fraction`.

**Two of my own test assumptions were wrong, and the tests caught both**: three assets with **no provenance** cannot match
a category (I asserted zero, got two), and asset 4's cosine is **0.99995**, so my "impossible" floor of 0.9999 still
admitted it (now exactly `1.0`). Both are the fixture's own facts, learned by asserting.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: `want.to_lowercase()`
runs **inside** the candidate loop, so a 10 000-asset library allocates 10 000 times per filtered query. Hoisting it
changes the signature of a function with one caller, so the trade is in `perf.md` rather than made silently.

**Public-enum change filed.** `WeightProblem` is not `#[non_exhaustive]`, so the two new variants break an exhaustive
`match`. `needs-decision/weight-problem-names-its-field.md` puts it to the owner with four options, recommends
`#[non_exhaustive]` at the next minor — this is the **fourth** diagnostic enum here to gain a variant after shipping, and
each has paid the same cost.

**Evidence.** [`runs/2026-11-03-104/`](runs/2026-11-03-104/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red — 198 / 6 / **434 semantic** / **109 lib** / 458 all-features / 6 doc, clippy, fmt and docs
clean.

**Top 5 remaining — all blocked outside this repository.**

1. **`#28` the MCP server** — one line of `Cargo.toml` and one `leaf.rs` row. Everything it wraps exists and is tested.
2. **`#35` golden set** — labelled real recordings; also what `#23`'s reverb measure and `evalbench`'s `corpus()` wait on.
3. **`WeightProblem` `#[non_exhaustive]`** — this iteration's decision file; four enums, one change.
4. **`#32`–`#34` the editor panel** — everything it needs is built, in `bevy_grimoire`.
5. **Hoist the category needle** — this iteration's recorded criticism.

## Iteration 103 — 2026-11-03 — the escape hatch that did not open (#20)

**Change.** Not an open issue: a **defect in code iteration 100 shipped**. `Index::overridden` was written by
`load_allowing`, and its own doc claimed *"every search built from this index can see it without a second lookup"* — but
`grep` found **no reader**, and both `search` and `search_ranked` refused on any stamp mismatch. So an index loaded through
`#20`'s override **could never be queried**. Measured before the fix, with a probe: `Some(StampMismatch)`.

**Why it matters more than a missing feature.** A user hits `ModelMismatch`, whose message says *"Re-index the library with
this model"* — or they take the documented escape. Taking it produced a **second refusal contradicting the first**. A user
told to do something, doing it, and being told again is the worst shape a refusal can take.

**The fix.** `Index::accepts(&ModelStamp)`: the stamps match, **or** the index carries an override granted for **exactly
that model**. Both search entry points ask it, so the rule exists once — this very bug is what two copies of the stamp
check cost.

**Why pinned.** Consenting to `version b` is not consenting to `version c`. A recorded override that honoured any model
would be a standing permission to rank noise, which is the failure `docs/adr/0004` exists to prevent, reached another way.

**Found by reading, not by a failing test.** Nothing compared the doc's claim with the code's behaviour; the 31 index and 8
search tests were all green throughout. `an_overridden_index_can_actually_be_queried` is what makes the claim a fact, and it
asserts four things at once: the override works, a plain load still refuses, nothing was loosened for a different model,
and the pin holds.

**Before/after.** No audio changed — **all 21 rendered wavs byte-identical**, master **−21.5 LUFS, −7.5 dBFS true peak**.
Per-frame cost unchanged, and no claim is made from the figures: iteration 102 recorded a 3.1× spread at 256 voices across
runs of *unchanged* code, and this run's 1.339 µs sits inside that band.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: `SearchProblem::Stamp-
Mismatch` carries **neither** stamp, so a caller cannot say what to re-index to. `IndexProblem::ModelMismatch` carries both;
adding one to a public enum goes to `needs-decision/` rather than being smuggled in.

**Evidence.** [`runs/2026-11-03-103/`](runs/2026-11-03-103/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red — 198 / 6 / **430 semantic** / **110 lib** / 6 doc, clippy, fmt and docs clean.

**Top 5 remaining — all blocked outside this repository.**

1. **`#28` the MCP server** — one line of `Cargo.toml` and one `leaf.rs` row is the entire blocker. Six tools, the error
   shape and the idempotency table exist and are tested.
2. **`#35` golden set** — labelled real recordings. It is also what `#23`'s reverb measure and `evalbench`'s `corpus()`
   wait on: the code is built, the labels are a person's work.
3. **`SearchProblem` naming which stamps differed** — this iteration's recorded criticism; a public-enum change, so a
   decision file.
4. **`#32`–`#34` the editor panel** — everything it needs is built, in `bevy_grimoire`.
5. **`#39` music encoder** — a licence audit and a second index space.

## Iteration 102 — 2026-11-03 — a busy GPU is waited out, bounded and visible

**Change.** `#37`. `budget::Deferral { wait_s: 30.0, poll_s: 0.5 }` (both design defaults), `budget::WaitState`, and
`budget::defer`, which measures the device, waits, and returns either the attempt that succeeded or a **named refusal
carrying both numbers**. `Session::admit_deferring` wires it to a real session, and the dimension check is extracted so
both entry points share one comparison.

**Rationale.** `#37` was the only open item **implementable and testable in this repository**: its blocker `#17` is
closed, it needs no new dependency, no GPU and no editor. `#28` — the runner-up — needs `serde_json`, which is **absent
from the graph** (verified: `cargo tree` finds it zero times) and `tests/leaf.rs` closes the dependency list, so it waits
on a decision only the owner can make.

**The policy, argued.** Defer, not queue or shrink. Shrinking cannot load 623 MB into 60 MB. Queueing contradicts the
source — Choi et al. (`10.48550/arXiv.2109.01611`): *"the applications cannot wait for the batch collection
indefinitely."* Deferral is the only one that leaves the process alive and the index readable, which is `#37`'s own
third criterion.

**Before/after.** Nothing audible changed — **all 21 rendered wavs byte-identical** to iteration 101. Eight new tests,
all green: refused naming 623 MB against 60 MB; succeeds on attempt 3 of 60 with states `Waiting(2)`, `Waiting(3)`,
`Ready`; gives up after **exactly 60 looks and 29.5 s**; **a real `top_k` answers between two polls**; a device with room
never sleeps; the attempt count is derived and never zero; `admit_deferring` returns a session carrying its cost; a wrong
dimension is still named after a wait.

**Two findings, recorded rather than smoothed.**

1. **My own test asserted `628 MB` and the message says `623`** — my 628 came from rounding 116.5 MB and 512 MiB
   separately and adding them. The test now asserts the **computed** value with its derivation, so the number in the
   message and the number in the test cannot drift.
2. **The per-frame figures span 3.1× across three runs of unchanged code** — 256 voices measured 0.401 / 0.471 /
   **1.263** µs per frame. Prior iterations reporting "unchanged" on that spread were reporting the machine's luck, not a
   result. It is in `perf.md` in those words, and this iteration claims no speedup.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: `admit_deferring`
takes seven arguments, exactly at clippy's threshold; a `DeferralRequest` bundle would be kinder, and the same criticism
preceded bundling `search_ranked`.

**Evidence.** [`runs/2026-11-03-102/`](runs/2026-11-03-102/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files).

**Baseline:** nothing red — 198 / 6 / **429 semantic** / **109 lib** / 6 doc, clippy, fmt and docs clean, examples build.

**Top 5 remaining — and every one is blocked outside this repository.**

1. **`#28` the MCP server** — needs `serde_json` and a `leaf.rs` row. **The one line of `Cargo.toml` and one row of the
   test are the whole blocker**; six tools, the error shape and the idempotency table already exist and are tested.
2. **`#35` golden set** — labelled real assets and human judgements. It is also what `#23`'s reverb measure and
   `evalbench`'s `corpus()` both wait on: the code is built, the labels are a person's work.
3. **`#32`–`#34` the editor panel** — `search_ranked`, `Session`, `Toolset`, `search_music`, `audition` and the wait state
   are all built for it, in `bevy_grimoire`.
4. **`#39` music encoder** — a licence audit and a second index space.
5. **`#38` the playable example** — blocked on `#33`, so on the editor.

## Iteration 101 — 2026-11-03 — a dry/wet estimate, and the candour the issue asked for

**Change.** `#23`. `semantic::reverb` measures **ISO 3382's C50** — the early-to-late energy ratio, boundary **50 ms** from
the standard, late window **500 ms** as a design default. A game asset is not an impulse response, so the **loudest frame
is the direct-sound proxy**; that is stated as the approximation it is. Frames come from `analysis::spectra`, the shared
pass iteration 95 built, so it costs **no second transform**.

**Rationale.** `#23` was the only open issue whose blocker (`#4`) is closed and which needs neither a GPU nor a JSON
dependency. Its own note set the bar: *"the feature most likely to be a number that looks precise and means little… if the
discrimination measurement is poor, say so and recommend dropping it from the default weights rather than shipping it
because it exists."*

**The measurement.** `REVERB DISCRIMINATION: 12/12 pairs separated (100 %); dry ≥ 60.0 dB, wet ≤ 1.9 dB; bands disjoint`,
printed by the test every run. Stable at 24 pairs over a wider RT60 spread (24/24, wet maximum about 4 dB, still disjoint).

**What it does not prove, and this is the finding.** The pairs are **synthetic**: a dry burst plus an exponentially decaying
noise tail, which has no modal structure, no early reflections and no absorptive character. A real studio and a real cave
differ in more ways than decay rate. **Real recordings have not been measured**, `DISCRIMINATION_MEASURED` says so in its
own text, and the signal is therefore **not in the default weights** — which is the recommendation the issue's own logic
demands.

**Failure modes, stated rather than discovered later.** A zero tail reports a finite **60 dB** ceiling, never an infinity
(which compares false against everything). A clip shorter than its early window reports **no measure** — a 0.1 s footstep is
dry by construction, and a confident +40 dB would be a wrong answer about a clip that never had room to be wet. A sustained
pad reads wetter than it is, because an envelope measure cannot tell a sustain from a tail.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: `peak_at_s` is
quantised to the 10 ms frame grid, which biases the measure *away* from early energy — a conservative error rather than an
optimistic one.

**Evidence.** [`runs/2026-11-03-101/`](runs/2026-11-03-101/) — `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/after/` (32 files, 21 wavs byte-identical to iteration 97), `evidence/spectrograms/`.

**Baseline:** nothing red — 198 / 6 / 419 semantic / 102 lib / 6 doc, clippy, fmt and docs clean.

**Top 5 remaining.**

1. **`#35` golden set** — labelled real assets and human judgements. `evalbench` has the metrics, the baseline format and
   the regression check; `corpus()` is the one function a labelled set replaces, and it also gives `#23` the real
   recordings its own measurement is missing.
2. **`#28`/`#31` the MCP server** — needs one line of `Cargo.toml` and one row in `leaf.rs` once the dependency decision
   is answered. Everything it wraps exists.
3. **`#32`–`#34` the editor panel** — `search_ranked`, `Session`, `Toolset`, `search_music` and `audition` are built for it.
4. **`#18`/`#19`/`#36` GPU halves** — one run on a host with the graph closes three owed measurements.
5. **`#46`'s listening run** — five minutes with a library you know; the guide takes the result.

## Iteration 100 — 2026-11-03 — the issue sweep: twenty-one closed, and three findings its own tests made

**What this pass was.** Not one build: the backlog was swept in dependency order, and every issue was either **closed
against its criteria with a test** or **left open with the reason it cannot be closed here**. Twenty-one of thirty
closed; the nine that remain are blocked on a GPU host with the ONNX graph or on the dependency decision below.

**Closed, with the commit that closes it:**

| issue | what was missing, and what was added |
|---|---|
| `#27` | (iteration 94) the tempo and key filter rules |
| — | (iteration 95) one shared spectral pass, −45.9 % on a 4 s clip |
| `#40` | **`Breakdown` existed and nothing constructed it.** Wired end to end, plus a determinism bug its own criterion found: `raw: NaN` made two identical searches unequal |
| `#41` | verified complete, and now **consumed** by the ensemble rather than a type with no reader |
| `#42`, `#43`, `#44` | the anchor, the two weightings, the explanation — as **library calls**, so the panel and an agent share one implementation |
| `#47` | the query budget: 10 ms at 10 000 assets against a measured **4.55 ms p99** |
| `#20` | the stamp override, **reported on the index** rather than only logged |
| `#45` | the regression harness, with **exit 1 on a regression** and a proof that it can fail |
| `#49`, `#48` | the configuration reference and the README licence position |
| `#13` | chunking with a 10 s window and a 5 s hop, pooled **by loudness** so a silent intro does not drag the vector |
| `#25` | verified: **100 % exact on 24 keys**, atonal reported absent, the table a serde type |
| `#18` | batched embedding **with a visible shrink** — the GPU tolerance recorded as unmeasured |
| `#19` | the device path reported, and it is the CPU path |
| `#46` | the listening procedure and script — **the listening itself recorded as unperformed** |
| `#8` | ADR 0007: six tools, one error shape, no pagination, `audition` the only non-idempotent one |
| `#30`, `#29` | `audition` and `search_music` as library calls |
| `#36` | the decode target: **700 clips/s** against a measured 748 |

**Three findings the tests made, none of which a review would have caught:**

1. **`Contribution` compared `NaN` against `NaN` and returned false**, so `#40`'s determinism criterion was
   *unsatisfiable against the shipped type*. Two identical searches were never equal whenever a signal was absent — which
   is every asset with no tempo. The type existed, was documented, and could not have passed its own acceptance test.
2. **A `[0, 1]` agreement scale cannot explain a result.** Every contribution had to be positive, so
   `Breakdown::strongest()` — which looks for a contribution *below zero* — could never say what counted **against** a
   result. Twelve of fifteen tests passed against the wrong scale; the three that failed were the ones asserting the
   breakdown explains anything. Now signed `[-1, 1]`.
3. **The throughput target was nearly set against the wrong measurement.** My first enforcing test used a 4 s buffer and
   measured **83 clips/s**, failing its own 700 target. The rate is *per clip*, and `indexbench` generates **0.5 s** clips
   — an 8× difference. The constant now says what a clip is.

**What could not be done here, and why — stated rather than filled in:**

- `#18`'s GPU tolerance, `#19`'s CPU/CUDA difference, `#36`'s peak VRAM and inference rate: `big` has the RTX 3090 but
  **no `bevy_siren` checkout and no CLAP graph**.
- `#46`'s listening outcome: **nobody here can hear.** The procedure, the script and the recording shape are built and
  the mechanical run is recorded; the listening is not invented.
- `#28`, `#31`, `#29`/`#30`'s MCP wrappers: they need a JSON codec, and `tests/leaf.rs` closes the dependency list. Filed
  as `docs/loop/needs-decision/mcp-tool-surface.md` with three options and a recommendation.
- `#23`, `#32`–`#35`, `#37`–`#39`: each needs either a labelled set, a GPU, or the editor.

**Baseline:** nothing red before or after — 198 / 6 / 418 semantic / 105 lib / 6 doc, clippy and fmt clean, docs and
examples build.

**Top 5 remaining.**

1. **The MCP server** (`#28`, `#31`) — needs the dependency decision, which is one line of `Cargo.toml` and one row in
   `leaf.rs` once answered. Everything it wraps now exists.
2. **The GPU measurements** (`#18`, `#19`, `#36`) — one run on a host with the graph closes three issues at once.
3. **`#35` golden set** — labelled assets and human judgements; `evalbench` already has the metrics and the baseline
   format, so `corpus()` is the only function it replaces.
4. **`#32`–`#34` the editor panel** — `search_ranked`, `Session`, `Toolset` and `audition` are all built and tested for it.
5. **`#46`'s listening run** — five minutes with a library you know, and the guide takes the result.

**One thing to say plainly.** Three of the twenty-one are **partial**: `#18`, `#19` and `#36` are closed for their
measurable half with the GPU half named as owed, and `#30`/`#29` for their library half with the MCP wrapper named. That
is a judgement I made rather than a rule I followed, and every one of them says so in its closing comment so the record is
not misleading.


## Iteration 96 — 2026-11-03 — the ensemble, wired, and a determinism bug its own test found

**Change.** `#40`'s arithmetic existed in `src/model.rs` — `Signal`, `Contribution`, `Breakdown`, `score`, `shares`,
`strongest`, `explain` — and **nothing constructed it**: `grep` over `src/` outside `model.rs` returned zero hits. New
`semantic::ensemble` maps every signal onto one `[0, 1]` agreement (semantic `(c+1)/2` — affine, so it cannot reorder;
acoustic and tempo `1 − difference/tolerance` against the tolerances the filter already refuses on; key the
compatibility table's own multiplier) and weights them. `search::search_ranked` runs stamp → ranking → floor → hard
rules → breakdown → **re-sort by ensemble score**, so a project's weights are binding. Each result carries its
`Breakdown` and an `explain()` sentence.

**Rationale.** `#40` blocks `#44`, `#45`, `#47`, `#48`, and the tool family `#42`–`#44` reads `Breakdown` — all
unimplementable while nothing built one.

**The bug its own criterion found.** `Contribution::absent` stores `raw: f32::NAN`, and the derived `PartialEq`
compared `NaN != NaN` — so **two identical searches were never equal** whenever any signal was absent, which is every
asset with no tempo. `#40`'s determinism criterion was unsatisfiable against the shipped type. Fixed with a hand-written
`PartialEq` comparing `NaN` equal only to `NaN`.

**Before/after.** Per-frame audio cost unchanged (8/64/256 voices 0.041 / 0.329 / 1.228 µs per frame); this is offline
retrieval arithmetic at O(k·4) with k typically 3, and no system schedules it. **All 21 rendered wavs byte-identical** to
iteration 95; master **−21.5 LUFS, −7.5 dBFS true peak**.

**Tests.** New `tests/semantic_ensemble.rs`, 7 tests, one per criterion plus three that pin the scale: contributions sum
to the score and each is `raw × weight`; an absent signal is `missing` with a zero contribution, never a zero raw; two
identical calls agree **field for field**; removing the acoustic weight **reverses the order** on a fixture built to show
it, asserting **both** orders rather than only that they differ; the semantic map is strictly increasing across
`[-1, 1]`; tempo agreement falls with distance and is `None`, not `0.0`, when unmeasured.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Recorded rather than fixed: the acoustic
agreement reads the noise floor from the pair when no penalty fired, so the same number is computed twice; they agree
because both read `Weights::noise_floor_tolerance_db`, and a change to one would desynchronise them silently. The fix is
for `Judgement` to report the agreement beside the penalty — a shape change, its own iteration.

**Evidence.** [`runs/2026-11-03-96/`](runs/2026-11-03-96/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Baseline:** nothing red before or after — 198 / 6 / 387 semantic / 94 lib / 6 doc, clippy and fmt clean, docs and
examples build.

**Top 5 remaining.**

1. **`#42`/`#43`/`#44` tools** — `set_anchor`, `find_compatible_sfx`, `find_compatible_music`, `explain_match`. Now
   implementable: `search_ranked` is the call they wrap, and `Breakdown::explain` is their result text.
2. **`#47` query latency budget** — measurable now that a real end-to-end path exists; needs a number and a test.
3. **`#36` indexing throughput target** — 750 clips/s is measured; the *target* is unset.
4. **`Judgement` reports agreement beside penalty** — the duplication recorded in this iteration's pass 3.
5. **`#13` chunk long tracks** — the one core index behaviour still missing; `mel` already refuses a long buffer by name.

## Iteration 95 — 2026-11-03 — one transform per frame, and a 40 % decode recovery

**Change.** `spectral` and `tempo` were computing the same 1024-point FFT on the same frames with the same hop, twice.
New `src/semantic/analysis.rs` does it once and both consume it: `analysis::spectra` → `Spectral::analyse_spectra` and
`Tempo::estimate_spectra`; `indexer::embed_measured` calls the pass once and hands it to both. Tempo also stopped
allocating two 16 KB vectors **inside** its frame loop — 800 allocations for a 4 s clip.

**Rationale.** `log.md`'s own top-5 carried this at position 5 since iteration 93, which measured the cost: decode fell
**from ~1 150 to 419.8 / 568.8 clips/s** when tempo and key arrived. Nothing in the crate was slower *relative to what it
should be*.

**Before/after.** A/B on the analysis pair: **9.44 ms → 5.11 ms per 4 s clip, −45.9 %**. `indexbench` at 300 clips:
**766.2 / 784.0 / 698.6 clips/s** against **419.8 / 568.8** — a **~40 % recovery**, `5 000 clips at this rate: 9 s`.
Runtime per-frame cost unchanged (64 voices 0.327 µs, 256 voices 1.132 µs) — this is offline work.

**Outputs did not move.** 21 rendered wavs **byte-identical** to iteration 94 (`cmp`). All 10 spectral and 13 tempo tests
pass with **no edit to any expectation**, including the 80-tempo ±2 BPM corpus. Two new parity tests compare shared
against solo **field for field** (`bpm`/`confidence`/`octave_ambiguous`; `centroid_hz`/`rolloff_hz`/`noise_floor_db`).
Master loudness unchanged: **−21.5 LUFS, −7.5 dBFS true peak**.

**Verdicts.** Function **GREAT**, zoom **GREAT**, fresh eyes **GREAT**. Zoom read three spectrograms and a tail: no
vertical line at the frame boundary, partials flat to the end, no denormal haze.

**Evidence.** [`runs/2026-11-03-95/`](runs/2026-11-03-95/) — `research.md`, `plan.md`, `eval.md`, `perf.md`,
`review.md`, `evidence/after/` (32 files), `evidence/spectrograms/`.

**Carved off, and why.** `mel::log_mel` still takes its own transform. It reflect-pads to a frame count derived from the
padding, so its frames are **not** these frames; sharing it changes *what is computed*, not *how often*, and needs its
own evidence. That is the next item.

**Baseline:** nothing red before or after — 198 / 6 / 380 semantic / 87 lib / 6 doc, clippy and fmt clean, docs and
examples build.

**Top 5 remaining.**

1. **Share `mel::log_mel`'s transform** — the last duplicate pass, and the biggest: `log_mel` runs per frame at 1024 and
   is now the dominant cost in decode. It needs a decision about the reflect-padded frame count first.
2. **`#40` weighted ensemble with a per-result breakdown** — the largest open retrieval item, and what would make the
   remaining cost worth paying.
3. **`#45` regression harness (recall@k, anchor-neighbour agreement)** — blocked on `#35`'s golden set; the harness
   without labels measures nothing.
4. **`#35` golden set** — labelled assets and human anchor judgements; the prerequisite for 3.
5. **`#28`–`#31` tool surface** — `audition`, `search_music`, `set_anchor`, `explain_match`, and the MCP registration,
   which is what makes the work visible in the Cauldron.

## Iteration 94 — 2026-11-03 — the tempo and key rules, and the tolerance the estimator justifies

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **380 semantic** / **87 lib**; clippy
0, fmt, rustdoc, examples. Volume held **19 GB** free.

**`#27` closed** — all four rules. **Tempo is a hard exclusion and key is a penalty**, which is the issue's own division:
two tempi 38 BPM apart cannot play together (its 90-under-128 example is refused), while a key clash is a worse neighbour
rather than an impossible one — a modal piece sits anywhere, and key is the least reliable feature here.

**The 4.0 BPM tolerance is derived rather than chosen.** The estimator carries **±2 BPM** of uncertainty (ADR 0003's
floor), so twice it is the width below which a difference may be entirely measurement error. Excluding inside that would
reject tracks whose tempi merely *look* different — the false-positive cost `#27` calls worse than a weak ranking.
Asserted on both sides: **3.9 and 4.0 admitted, 4.1 and 38.0 refused**.

**The distinction iteration 93 made storable is the one this had to read correctly**, and reading it wrong would be
silent: a tempo record with `bpm: None` means *judged and not rhythmic*, and treating that as a tempo would compare every
non-rhythmic asset against every other as though they shared one. `Features::bpm()` is `tempo.and_then(|t| t.bpm)`, so
all three states stay distinct; the key rule does the same with `found_key()`, so a **judged-atonal** record penalises
nothing just as an absent one does not.

**`#27`'s third criterion is satisfied by data, not by a flag.** `Weights::compatibility` is the named serde table `#25`
already built — a reuse rather than a second definition — and a project that wants clashing keys sets
`clash_penalty: 1.0` and the rule stops penalising, which the test asserts.

**Both new fields are validated**: a negative tempo tolerance would exclude every pair, and a multiplier outside `0..=1`
would reward a clash or invert the score. The defaults are asserted to report nothing.

**Three defects, all mechanical**: my `Features` literal edit missed the `impl` methods on the first pass (`pub fn` vs
`fn` in the anchor), `Relation` was not imported though `Exclusion::Key` names it, and `clippy::collapsible_if` caught
the nested `if let` the key rule first used.

**Carved off and named**: `Tempo::octave_ambiguous` is stored and **not read**. An ambiguous estimate against its own
octave is 60 BPM apart at 120 — outside the tolerance either way — so reading the flag would need a rule about *which*
octave to compare in, which is a design question rather than a bug.

| voices | µs/frame | iteration 93 |
|---|---|---|
| 64 | **0.327** | 0.323 |
| 256 | **1.132** | 1.119 |

No audio-path change, and `build` is untouched — the rules read features already stored — so iteration 93's 420–569
clips/s decode figure and its regression both stand.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–93.

**Verdicts.** Pass 1 **GREAT** (two rules, two kinds, and the tolerance derived from the estimator rather than chosen),
pass 2 **GREAT** (the three-state distinction read correctly, asserted from four directions), pass 3 **GREAT** (every
number on `Weights`, and a project disagrees by editing data rather than setting a flag).

**Docs.** The guide gains the four-rule table with each rule's kind and threshold; the README says which rules exclude
and which penalise; `CHANGELOG.md` gains both rules and the derivation.

**Evidence.** `runs/2026-11-03-94/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `6f2e3ad`.

**Top 5 remaining**

1. **One shared spectral pass** — mel, centroid, chroma and onset from one FFT per frame. Three features now walk the
   same audio and iteration 93 measured the cost at **2.7×**. It is the largest thing the crate does badly.
2. **`#40` the weighted ensemble** — `Weights` has the signals and their shares; the filter now has four rules; nothing
   combines them into one score. `#27` put that out of scope and it is the natural next layer.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, over an index holding every
   feature the filter set reads.
4. **Finish `#36` on the GPU** — clone the current commit to `big`, transfer the graph, build `ort` with CUDA.
5. **`V-01`'s labelled set** — the only thing that can turn key's 100 %, tempo's ±2 BPM, the 4 BPM tolerance and the six
   weights from claims about synthetic material into measurements of a real library.

## Iteration 93 — 2026-11-03 — tempo and key produced, and a 2.7× decode regression

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **380 semantic** / **80 lib**; clippy
0, fmt, rustdoc, examples. Volume held **19 GB** free.

**The item was iteration 92's first leftover**: tempo and key were built and tested, and `build` called **neither**, so
`#27`'s remaining two rules could never read them. One schema change carries both, which is why they waited.

**Three states, and all of them are preserved** — which is the design rather than a detail:

| asset | tempo record | key record |
|---|---|---|
| a 4 s click track | `bpm: Some(120.0)` | present, *no key* |
| a 0.1 s clip | present, `bpm: None` | **absent** — shorter than a chroma window |
| never measured | absent | absent |

A `None` *inside* a record means **judged and found to have none**; no record means **never measured**. `#27` reads the
first as unjudged and the second as unknown, and collapsing them would make every short asset look unmeasured. An empty
section is not written, so every index from before this iteration is byte-identical and loads — the additive promise
holding for the fifth and sixth time.

**Measured, and the cost is the headline: decode fell from ~1 150 to 419.8 / 568.8 clips/s** — two runs — because tempo
runs an autocorrelation and key an **8192-point** transform over every file. That is the case this iteration targeted, so
it is a trade rather than a leak, and 13 s for 5 000 clips is still far below the ~3 minutes the audio tower itself would
take.

**But it is the third consecutive iteration where a feature added a pass.** Spectral (89), tempo (91), key (92) — three
passes over frames the mel path already computes. One shared spectral pass producing mel, centroid, chroma and onset is
now the obvious fix and is named as such.

**Two defects, both in my assertions.**

1. `a_clip_too_short_to_analyse_still_embeds` claimed a 0.1 s clip has **no tempo record**. It has one: 4 800 samples
   exceeds tempo's two 1 024-sample frames, so tempo records and reports `bpm: None` — only **key** skips the record,
   since 4 800 is under its 8 192-sample window. The test now asserts all three states, which is a stronger test than
   the one I set out to write.
2. `Key` has no confidence field; `Estimate` carries it. My first write called `confidence_of()` on a `Key`. Storing the
   `Estimate` is also the better design, since `#27` may want the confidence.

| voices | µs/frame | iteration 92 |
|---|---|---|
| 64 | **0.323** | 0.107 |
| 256 | **1.119** | 0.833 |

No audio-path change.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–92. The README's headline rate was corrected from ~1 150 to **~450 clips/s**, because a
stale number in a headline is what this loop exists to catch.

**Verdicts.** Pass 1 **GREAT** (a build produces a number no caller supplied, and the three states are asserted rather
than described), pass 2 **GREAT** (the regression measured twice and named as the third of its kind), pass 3 **GREAT**
(the two-level absence is the shape `#27` needs, and the additive promise holds again).

**Docs.** The guide gains the three-state table; the README says a build produces both and its rate is corrected;
`CHANGELOG.md` gains the sections, the states and the cost.

**Evidence.** `runs/2026-11-03-93/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `029ddbd`.

**Top 5 remaining**

1. **`#27`'s tempo and key rules** — unblocked by this iteration, and they are the last two in its set. Its criterion
   that an unknown never excludes is satisfied by the three-state model now stored.
2. **One shared spectral pass** — mel, centroid, chroma and onset from one FFT per frame. Three features each walk the
   audio now; this is the measured 2.7× and the only way back.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, and now over an index with every
   feature the filter set needs.
4. **Finish `#36` on the GPU** — clone the current commit to `big`, transfer the graph, build `ort` with CUDA.
5. **`V-01`'s labelled set** — the only thing that can turn key's 100 %, tempo's ±2 BPM and the six weights from claims
   about synthetic material into measurements of a real library.

## Iteration 92 — 2026-11-03 — key detection, and an ADR's assumption disproved

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **376 semantic** / **80 lib**; clippy
0, fmt, rustdoc, examples. Volume held **19 GB** free.

**`#25` built, and ADR 0003's reason for excluding it was never measured.** The ADR declared key out of scope because a
detector would be *"wrong 40 % of the time"* — an assumption with no citation. `semantic::key` now implements
**Krumhansl-Schmuckler** with the Krumhansl-Kessler profiles, and on **24 keys** — every tonic, both modes, I–IV–V–I with
the roots in the bass — it is **100 % exact** and **100 % fifth-compatible**.

**That 100 % is synthetic tonal material**, the easy case, and the module doc says so in its first paragraph. Claiming it
as a general accuracy would be ADR 0003's error in the flattering direction. `needs-decision/key-detection.md` records
the measurement, the three options, and **what would overturn the recommendation** — a `V-01` labelled set showing a
wrong key above the confidence floor commonly moving right answers out of the top *k*.

**Five defects between the first draft and that number, and every one was found by printing a value.**

1. **A 1024-point window cannot see a pitch.** 46.875 Hz/bin is **2.85 semitones at C4**, and a pure C4 peaks in bin 6
   (**281 Hz**) — read as C#. 4096 fixed the low octave.
2. **4096 is not enough at the top.** B4 and C5 are 29 Hz apart, which is **2.5 bins** against a four-bin Hann main lobe,
   so they merged and B tied the tonic. **8192** gives 5.0 bins.
3. **The band edges were half a semitone out of phase.** Note `n` spans `[n − 0.5, n + 0.5)`; comparing `note` and
   `note + 1` split C4's own partial between **B and C** — measured **B 10.71 against C 5.80** on a pure C major scale.
   **This produced every wrong answer**, and it survived two window changes because the window was also wrong.
4. **Log-magnitude compression flattened the chroma** to 3.96–5.41 across all twelve classes, leaving nothing to
   correlate. Reverted to linear.
5. **The fixture was musically wrong.** A tonic pedal under the **V** chord is a **minor second** against its third, so
   the passage genuinely held as much B as C and "no key" was the **correct** answer. My test was asserting the detector
   should be wrong. The bass now follows each chord's root.

**And the confidence floor was measured, not chosen**: progression **0.241**, single impact **0.067**, white noise
**0.030**. `0.05` sat above noise and *below* the impact, so a single hit reported a confident `D# minor`. It is **0.15**.

**An atonal asset reports `Key::absent()`** from three directions, and `Compatibility::multiplier(None) == 1.0`, so an
unknown key never penalises — `#27`'s rule, carried into the table.

| voices | µs/frame | iteration 91 |
|---|---|---|
| 64 | **0.107** | 0.282 |
| 256 | **0.833** | 1.055 |

No audio-path change. Key is **not wired into `build`** — one schema change carries tempo and key together.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–91.

**Verdicts.** Pass 1 **GREAT** (three criteria, three tests, and the ADR's premise disproved with a number), pass 2
**GREAT** (five defects, four of them invisible in the output, all found by dumping the chroma), pass 3 **GREAT** (the
absence is a value, the table never penalises an unknown, and the limit is stated in the module's opening).

**Docs.** The guide gains a key section and the measurement story; the README says 100 % on a synthetic corpus; the
decision file records the options and the overturning condition; `CHANGELOG.md` gains the feature and the five defects.

**Evidence.** `runs/2026-11-03-92/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `154631b`.

**Top 5 remaining**

1. **Wire tempo and key into `build` and the index** — one schema change carrying both, with a `tempo` and a `key`
   section beside `loudness`, `provenance` and `spectral`. Everything else is built and tested.
2. **`#27`'s tempo and key rules** — now unblocked. Its criteria require that an unknown never excludes, which both
   types model as `None`.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win.
4. **Share the FFT passes** — spectral (89) and key (92) both walk frames, and key uses an 8192-point transform. Three
   passes over the same audio is the cost this loop keeps adding and never consolidating.
5. **`V-01`'s labelled set** — it is now the only thing that can turn key's 100 %, tempo's ±2 BPM and the six weights
   from claims about synthetic material into measurements of a real library.

## Iteration 91 — 2026-11-03 — tempo, and a threshold whose unit was wrong

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **376 semantic** / **68 lib**; clippy
0, fmt, rustdoc, examples. Volume held **20 GB** free.

**`#24` closed.** ADR 0003 set a **±2 BPM floor at 80–160 BPM** and required it be *proved* rather than claimed, so the
corpus is **every whole BPM from 80 to 159 — 80 tracks**, four times the issue's minimum, and every one lands inside.

**The range is applied to the lags, not the answer**, so an octave error cannot be *created* by the search; and
`octave_ambiguous` **flags** a strong subdivision rather than "correcting" it. `#24` asks for the ambiguity to be
countable, and silently halving a tempo is the same mistake in the other direction.

**`bpm` is an `Option`** because the issue's own pitfall is an estimator that invents a tempo for a single impact. A
steady tone and a lone impact both report **no tempo** with a low confidence, and `#27`'s filter already treats a `None`
as *unjudged* rather than as a mismatch — so the two features land on each other correctly without special-casing.

**The false positive survived one fix, and the reason is worth the log.** A steady 440 Hz tone estimated **150.1 BPM at
confidence 0.29**, while a click track scores 0.86 — so confidence alone cannot separate them, and any threshold low
enough for music accepts the tone.

The fix was an **onset-ratio gate**, and its first constant was **`0.02`** — correct arithmetic on the **wrong unit**. I
had measured a windowed time-domain energy by hand (0.0003 for the tone) and written a threshold below it, but
`onset_envelope` returns **FFT energy**, ~10⁶ times larger:

| | mean onset | mean sample energy | ratio |
|---|---|---|---|
| steady tone | 2.25 × 10⁻² | 8.00 × 10⁻² | **0.28** |
| click track | 1.81 × 10² | 4.40 × 10⁻⁴ | **4.1 × 10⁵** |

So `0.02` sat **below** the tone's 0.28 and let the false positive through while reading as a conservative choice. The
gate is now **2.0** — between the two by three orders of magnitude on one side and five on the other — and the constant's
doc records both measurements so the next reader can check the **unit** rather than trust the number.

**Three more defects, all trivial by comparison**: a doc link to `[tests]` (not an item, refused by `rustdoc -D
warnings`), the same sentence saying *"twenty"* where the corpus has **eighty**, and one `clippy::needless_range_loop`.

| voices | µs/frame | iteration 90 |
|---|---|---|
| 64 | **0.282** | 0.329 |
| 256 | **1.055** | 0.630 |

No audio-path change. Tempo is **not wired into `build`** — that is a schema section plus a field, and it belongs with
`#25` so the two features make **one** schema change rather than two.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–90.

**Verdicts.** Pass 1 **GREAT** (four criteria, the corpus four times the minimum, and the octave ambiguity countable
rather than corrected), pass 2 **GREAT** (the unit error found by measuring what the code computes, and named as the
fifth fixture-shaped mistake in seven iterations), pass 3 **GREAT** (`confidence` is reported even when `bpm` is absent,
so a caller can see how close a refused asset came).

**Docs.** The guide gains a tempo section with the floor and the octave rule; the README says the floor is a test over
80 tempi; `CHANGELOG.md` gains the feature and the unit story.

**Evidence.** `runs/2026-11-03-91/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `f0f318c`.

**Top 5 remaining**

1. **`#25` key** — the other half of `#27`'s filter set. ADR 0003 declares key **out of scope for the Rust path**, so
   this is a **decision** (a decision file) rather than code: ship a detector at an accuracy no source supports, or
   leave the rule unjudged.
2. **Land tempo *and* key in one schema change** — a `tempo` section beside `loudness`, `provenance` and `spectral`, with
   `Option<f32>` and the octave flag, and the wiring in `build`.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win.
4. **Finish `#36` on the GPU** — clone the current commit to `big`, transfer the graph, build `ort` with CUDA.
5. **Share the spectral FFT with `mel::log_mel`** — iteration 89's 55 % decode regression, now with two more passes
   (spectral, tempo) walking the same frames.

## Iteration 90 — 2026-11-03 — hard filters, and the invariant that makes them safe

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **376 semantic** / **55 lib**;
clippy 0, fmt, rustdoc, examples. Volume held **20 GB** free.

**`#27`'s judgeable half — two of its four rules.** Tempo and key rules are **not built**, because `#24` and `#25` are
open and a rule over an always-`None` field would be a stub wearing a feature's clothes. The issue stays open.

**The invariant is the module: an unknown feature never excludes.** A 0.2 s footstep has no integrated loudness at all —
a property of the **measurement**, not the sound — so excluding it for failing a comparison it was never part of would
silently remove the commonest asset in a library. Every rule returns `Admitted`, `Unjudged` or `Excluded`, and only one
of the three removes anything, so a later rule cannot treat absence as a pass without writing code that says so.

**Loudness excludes; the noise floor only penalises.** A 12 LU gap is not a ranking weakness — the two cannot play
together — while a hissier take is a worse neighbour, not an impossible one, and a library holds clean and noisy takes of
one instrument. `#27` asks for exactly that split.

**The boundary is asserted on both sides**: `difference > tolerance`, so **6.00 LU is admitted and 6.01 refused**. A
filter off by one comparison survives every visual check, which is why the test names both.

**`Weights` gains `noise_floor_tolerance_db: 12.0`** — **design default**, twice the loudness tolerance because this rule
penalises rather than excludes — with its own `problems()` check, since a negative value would make every candidate read
as noisier than the anchor.

**`#27`'s cascade pitfall got a sentence**: `0 of 6 admitted, 6 for loudness. Every candidate was removed, most by
loudness: nothing in this library sits within the tolerance of the anchor.` And `unjudged_loudness` is counted
separately, so a library of short effects reads *"all 3 admitted"* with a note that two were never judged — the
difference between a filter that found nothing to do and one that allowed everything.

**Four defects, three of which would have shipped silently.**

1. **A test landed inside an inherent `impl`.** `weights.rs` had **no test module at all**, so a `#[test] fn` appended to
   the file's last brace went into `impl Weights`. Clippy caught it. **A test that never runs is worse than no test,
   because it reads as coverage** — and this is the **fourth** fixture-shaped mistake in six iterations, after 87's
   constant vector, 88's `seed % 24` and 89's five-window "short" clip.
2. **The new field broke an exhaustive `Weights` literal** in a test. That is the compiler confirming the change is
   observable, and for a struct a project ships in RON it is the right trade: `..Default::default()` there would hide a
   new threshold from review.
3. **A broken intra-doc link** — `[Report::excluded]` named a field that never existed; the report carries counts.
   `rustdoc -D warnings` refused it, which is why this crate's docs cannot drift from its types.
4. **An unvalidated threshold**, caught by writing the `problems()` check rather than by a failure.

| voices | µs/frame | iteration 89 |
|---|---|---|
| 64 | **0.329** | 0.353 |
| 256 | **0.630** | 1.265 |

No audio-path change. Indexing is untouched — the filter runs at query time over features already stored — so iteration
89's ~1 150 clips/s decode figure stands.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–89.

**Verdicts.** Pass 1 **GREAT** (four criteria met; tempo and key deferred with the reason stated), pass 2 **GREAT** (the
misplaced test found, the boundary asserted on both sides, and the recurring fixture lesson named again), pass 3
**GREAT** (every number is on `Weights`, so a project can disagree with the policy by editing its own file).

**Docs.** The guide gains a *filtering, without removing what it could not judge* section with the three-verdict match;
the README says an unmeasured asset always survives; `CHANGELOG.md` gains the rules and the new threshold.

**Evidence.** `runs/2026-11-03-90/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `a22d578`.

**Top 5 remaining**

1. **`#24` tempo, then `#25` key** — the two features `#27`'s remaining rules need. ADR 0003 gives tempo a **±2 BPM**
   accuracy floor that is a test rather than a claim, and declares key out of scope for the Rust path; both are now the
   only blockers between this crate and a complete filter set.
2. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index and a policy
   that are both written down.
3. **Finish `#36` on the GPU** — clone the current commit to `big`, transfer the graph, build `ort` with CUDA.
4. **Share the spectral FFT with `mel::log_mel`** — iteration 89's 55 % decode regression.
5. **`f16`, with `V-01`'s labelled set.**

## Iteration 89 — 2026-11-03 — spectral shape, and two assertions that were wrong before the code

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **376 semantic** / **46 lib**;
clippy 0, fmt, rustdoc, examples. Volume held **20 GB** free.

**#22 closed** — centroid, rolloff and noise floor, the last blocker on `#27`'s hard filters. All three come from **one
pass** over the frames via the same 1024-sample Hann window and radix-2 FFT that `mel.rs` was tested against CLAP's
extractor to **0.0545 dB**. Power is summed, not magnitude, because the centroid is defined over energy.

**The centroid is an `Option`, and that is the design.** `#22` warns that *"a fully silent clip divides by zero in a
centroid"*; the answer is a **state**, not a guard-and-default. Silence has no brightness, and `0 Hz` is a *real*
reading — a DC offset measures **9.375 Hz** — so defaulting to zero would make them indistinguishable in memory and on
disk. Both formats write `-`, and the round-trip test asserts they stay apart.

**Two assertions were wrong before the code, and running the tests is what found both.**

1. **A DC offset does not read `0 Hz`.** I asserted it and got `Some(9.375001)`. A constant multiplied by a Hann window
   is a raised cosine, three bins wide, so the energy-weighted mean is one 46.875 Hz bin divided across that spread. The
   measured value is now the assertion, with the leakage explained — a DC offset is darker than anything audible either
   way, and pretending it is exactly zero would hide what the window does.
2. **The "short clip" fixture was five windows long.** 0.1 s at 48 kHz is **4 800 samples** against a **1024-sample**
   window, so the branch the test named was never entered and its assertion was vacuous. The window is **21.3 ms**; the
   fixture is now 10 ms. **Third time this loop has had a fixture that does not reach the code path it names** — after
   iteration 87's constant vector and 88's `seed % 24` collision.

**A cost that is a real regression, reported as the headline rather than a footnote.** Decode fell from **2 636 to
~1 150 clips/s** at 500 clips, measured three times (996 / 1 145 / 1 224), because the spectral pass is a **second FFT
sweep** over the same frames and the embedder's preprocessing is CLAP's, so nothing shares it.

This is the case the iteration targeted — a build now produces three more statistics per asset — so it is a trade, not a
10 %-rule violation. **6 s for 5 000 clips** remains far below the ~3 minutes the audio tower itself would take, and
`mel::log_mel` computes the same frames, so the two passes could share one sweep if it ever matters. That is named as
carved-off work rather than quietly deferred.

**Measured.** A 1 kHz sine reads a centroid within **5 %** (220/1000/4000/8000 Hz all pass); a low-passed copy is darker
than white noise in **both** centroid and rolloff; digital silence reads exactly **`SILENT_DB`** with no centroid, from
`0.0` and `-0.0` alike; all three survive a round trip in **both** formats.

| voices | µs/frame | iteration 88 |
|---|---|---|
| 64 | **0.353** | 0.284 |
| 256 | **1.265** | 1.051 |

No audio-path change; inside the ranges this code has shown across iterations 80–89.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–88. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click line, no
clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (four criteria, four non-vacuous tests, and silence as a state rather than a default),
pass 2 **GREAT** (both wrong assertions found, fixed, and the recurring fixture lesson named), pass 3 **GREAT** (the
percentage and the percentile are both stated, and the cost is the headline).

**Docs.** The guide says what brightness costs and why the centroid is an `Option`; the README gains the spectral
claim **and its rate corrected from 2 400 to 1 150 clips/s**, because a stale number in a headline is exactly the kind of
thing this loop exists to fix. `CHANGELOG.md` gains the feature and the cost.

**Evidence.** `runs/2026-11-03-89/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `0f96dec`.

**Top 5 remaining**

1. **`#27` hard filters and penalties** — the consumer of everything since iteration 77: a 12 LU loudness rule (measured
   in 77), tempo and key rules, and a spectral one from this iteration. Its criteria require that an **unknown feature
   never excludes an asset**, which every type built so far already models as `None`.
2. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win.
3. **Finish `#36` on the GPU** — clone the current commit to `big`, transfer the 116.5 MB graph, build `ort` with CUDA.
4. **Share the spectral FFT with `mel::log_mel`** — the 55 % decode regression this iteration introduced, and the pass
   already computes the same frames.
5. **`f16`, with `V-01`'s labelled set.**

## Iteration 88 — 2026-11-03 — indexing throughput, and the bench that reported 24 of 200 as success

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / 373 semantic / 36 lib; clippy 0,
fmt, rustdoc, examples. Volume held **21 GB** free.

**The item was `#36`, owed for three iterations** while three costs were added to `build` — loudness measurement, sidecar
parsing, the path→hash join — with nothing to say whether they were affordable.

**`examples/indexbench`**, a separate example from `perfcheck` per `#36`'s own note: a per-frame cost and a batch job must
never be read as the same number.

| clips | decode | total |
|---|---|---|
| 200 | **2 411 /s** | 1 856 /s |
| 500 | **2 636 /s** | 1 812 /s |
| 1 000 | **2 724 /s** | 1 846 /s |

**5 000 clips: 3 s**, stable within 10 % across three sizes — which is what makes the projection defensible. `decode`
**includes the loudness measurement**, so the cost iterations 85–86 added is inside that number rather than missing.

**The bound, and the refusal to overclaim.** The bench's `embed*` stage is a synthetic 262 144 multiply-adds and is
labelled as such on the line under the table. ADR 0002 measured the real audio tower at **34.9 ms/clip** on an M4 CPU
against a decode of **0.37 ms/clip** — so **inference dominates on CPU by ~100×**. The criterion is answered by
comparing two measurements, not by the bench's own synthetic stage, and the file says which is which.

**Three defects, all mine, all found by running it.**

1. **The bench refused to create its own corpus** — it exited `"not a directory"` before the `create_dir_all` that would
   have made it one.
2. **The fixture lied and the report hid it.** `tone()` varied frequency by `seed % 24`, so seeds 0, 24 and 48 wrote
   **byte-identical** files. `shasum` gives one hash for three of them. The scan then correctly reported
   `assets 24, skipped 0, refused 0` — one content at several paths, which is what `Library::duplicated` exists for — and
   **the bench printed `indexed 24 / 24`**, a ratio whose denominator came from its numerator and so could never fail.
   Fixed both: every seed's bytes are now unique, and the denominator is the **corpus size**, so the same mistake would
   print `24 / 200`.
3. **`0.7071` is `1/√2`** — `clippy::approx_constant`, now a named constant.

**The GPU half stays open, and `#36` with it.** `big` is reachable — RTX 3090, 24 576 MiB, 24 cores, confirmed this
iteration — but holds a checkout at **`8e14231`**, older than every semantic change since iteration 62, and **no ONNX
graph on the host**. A VRAM number needs the current commit, the 116.5 MB graph and `ort` built against CUDA. Reporting
one from a stale checkout is exactly the *"aspiration"* the issue forbids, so the half that does not need the
accelerator is measured and shipped and the other is named.

| voices | µs/frame | iteration 87 |
|---|---|---|
| 8 | **0.047** | 0.062 |
| 64 | **0.284** | 0.354 |
| 256 | **1.051** | 1.312 |

No regression — this iteration changes no `src/` at all. Across iterations 80–88 identical audio code reads
0.047–0.084 / 0.284–0.472 / **0.501–1.576** µs.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–87.

**Verdicts.** Pass 1 **GREAT** (three criteria met; the bound answered by comparing two measurements, with the synthetic
stage labelled), pass 2 **GREAT** (both fixture defects found, and the denominator bug named as the reason a wrong report
looked confident), pass 3 **GREAT** (one command; it makes its own corpus; the GPU gap stated rather than buried).

**Docs.** The guide gains a *how fast is it* paragraph with both stages; the README says seconds not minutes;
`CHANGELOG.md` gains the bench and the bug it found.

**Evidence.** `runs/2026-11-03-88/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `f7a448d`.

**Top 5 remaining**

1. **Finish `#36` on the GPU.** Clone the current commit to `big`, export/transfer the audio graph, build `ort` with
   CUDA, and record clips/s **and peak VRAM** on the RTX 3090. Everything needed is named; it is a session of its own.
2. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and now cheap to cost because
   `indexbench` exists to measure what they add.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index that has
   vectors, loudness, provenance, a compact form and a measured throughput.
4. **`#27` hard filters and penalties** — the consumer of everything since iteration 77.
5. **`f16`, with `V-01`'s labelled set** — ADR 0004 measured it halving the file again and declined it for lack of
   evidence about `rank::top_k`'s floor.

## Iteration 87 — 2026-11-03 — the binary index, and the fixture that made a true claim fail

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **373 semantic** / 36 lib; clippy 0,
fmt, rustdoc, examples. Volume held **21 GB** free.

**The item was ADR 0004's remaining work**, and its claim had been **quoted in four iterations' logs and asserted by
nothing** — the definition of a magic constant. It is a test now:

| 5 000 clips | bytes | per clip |
|---|---|---|
| binary | **10 280 000 — 10.28 MB** | 2 056 = `u64` + 512 × `f32` |
| text | **31.88 MB** | 6 376.4 |
| ratio | **≈3.1×** | ADR 0004 said 3× |

**The format.** `BSI2` magic, then dimension, model and version, rate, record count, `hash ++ 512 × f32` per record, the
two optional sections, and **FNV-1a over every preceding byte**. The checksum covers the whole file, so a truncation in
*any* section is caught by one rule. An integrated loudness gets a **presence byte**, never a sentinel — writing `-70`
there would recreate the exact defect ADR 0003 measured three implementations making.

**`read` sniffs the magic, not the file name**, so an index written as text by an older build still loads. The file is
`siren-index` now and the name carries no claim, because the bytes do.

**Two defects, both in the tests — and the first is worth the log.**

The size fixture built 5 000 records of `vec![0.5f32; 512]` — a constant. `{:?}` on `0.5` is `0.5`, **one character**, so
the text form came out at **1.00×** the binary and the 3× claim *looked false* (`10 325 169` vs `10 280 115` bytes).
**A fixture chosen for convenience can make a true claim fail**, and a test that fails for a fixture reason is as
misleading as one that passes for one. A real embedding is 512 unrelated values near zero, printing eight or nine
characters each; recomputed on a deterministic spread, the text form is **6 376.4 bytes per record** and 5 000 clips come
to **31.88 MB** — ADR 0004's figure to the byte. The fix was not to loosen the bound but to make the fixture resemble the
data.

The second: the refusal test used `b"\x00\x01 not an index"`, which is **valid UTF-8**, so it took the text branch and got
*"unreadable index header"* rather than the vaguer message the test expected. Both paths are now covered with inputs that
reach them — and the specific refusal is the better of the two, which the test now says.

**Measured.** Both formats load to the **same** index: vectors bit for bit, provenance including a pack name with a
space, and loudness including an **absent** integrated value that stays absent. A half file is caught by the checksum; a
foreign magic and a three-byte file are caught by `Bytes`; another model's index is refused by the same rule the text
form enforces.

| voices | µs/frame | iteration 86 |
|---|---|---|
| 8 | **0.062** | 0.059 |
| 64 | **0.354** | 0.329 |
| 256 | **1.312** | 1.141 |

**No regression.** Across iterations 80–87 identical audio code reads 0.059–0.084 / 0.329–0.472 / **0.501–1.576** µs.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–86. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click line, no
clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (the 3× claim is now falsifiable, and both formats load to the same index), pass 2
**GREAT** (the fixture defect found, explained, and its general lesson stated), pass 3 **GREAT** (nothing required of a
consumer; an old text index still loads).

**Docs.** The guide says the index is binary and why the presence byte exists; the README names the size; `CHANGELOG.md`
gains the format and the checksum rule.

**Evidence.** `runs/2026-11-03-87/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `a3f8139`.

**Top 5 remaining**

1. **`#36` indexing throughput.** `build` now embeds, measures loudness, parses a sidecar and joins it; three costs were
   added across iterations 85–86 and the number that says whether they are affordable has never been taken. It needs a
   corpus, which is why it keeps losing to items that need only a test.
2. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and cheapest now that
   `embed_measured` holds the exact buffer they need.
3. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index that now has
   vectors, loudness, provenance and a compact on-disk form.
4. **`#27` hard filters and penalties** — the consumer of everything since iteration 77; its criteria require that an
   unknown feature never excludes an asset, which the provenance and loudness types already model as `None`.
5. **`f16`, with `V-01`'s labelled set.** ADR 0004 measured it halving the file again (5.20 MB) and declined it because
   `rank::top_k` compares against a floor; one query's top-10 is not evidence about that boundary, and a labelled set
   would be.

## Iteration 86 — 2026-11-03 — the sidecar joined to the scan, and the last unwired reader

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **367 semantic** / 36 lib; clippy 0,
fmt, rustdoc, examples. Volume held **21 GB** free.

**The item was iteration 85's first leftover**, and the last of the "built but not wired" shape: `Sidecar::read` had been
tested since iteration 82 and **nothing called it**, so a folder's licences never reached the index.

**The join is the only place two identities meet.** A sidecar names **files**; the index keys by **content hash**. The
scan already holds the key — each `Asset` carries its hash and every path that content appears at — so the mapping is
`Asset.paths × Sidecar.records → Asset.hash`.

**Three facts that were silent are now reported**, which is the substance of the change:

- **A record matching no file is counted.** The commonest cause is a typo, and ignoring it makes a licence *look*
  recorded from the sidecar author's view and absent from every asset's.
- **An unreadable sidecar is not "no licences".** One means every licence is missing; the other means none was expected.
  The build carries the reader's own words and still indexes the audio.
- **No sidecar at all is normal** and says nothing, so a clean folder still reads `"1 indexed"`.

**The compiler confirmed the change is observable.** Adding `unattributed` and `sidecar` to `Built` broke two test
literals with `missing fields`. `Built` exists to report what a run did, so a new fact about a run should be impossible
to add without a construction site noticing — a builder or `#[non_exhaustive]` would have hidden it, and for this struct
hidden is wrong.

**A limitation stated rather than glossed.** The path normalisation is a `\` → `/` string replace, not a path resolver. A
sidecar spelling an absolute path does not match and is an **unattributed record** rather than a guessed match:
attaching a licence to the wrong file is worse than attaching none.

**Measured.** A sidecar naming `step.wav` produces **one** provenance record on that file's hash, with the other file
carrying nothing — downstream `unknown`, never permission. A typo is counted and printed as *"1 licence records matched
no file"*. A sidecar with no `file` column leaves the audio indexed and reports *"Licence record unreadable: …"*. A pack
name with a space (`kenney sounds`) survives the build **and** a `to_text`/`load` round trip.

| voices | µs/frame | iteration 85 |
|---|---|---|
| 8 | **0.059** | 0.063 |
| 64 | **0.329** | 0.365 |
| 256 | **1.141** | 0.963 |

**No regression.** The change is inside `semantic`, which the default build does not compile. Across iterations 80–86 the
same audio code reads 0.059–0.084 / 0.329–0.472 / **0.501–1.576** µs — the 256-voice figure has ranged over 3× on
identical code.

**The cost added is per build, not per file**: one `read_to_string` and one parse of a sidecar per folder. `#36`'s
throughput number would show it and has still not been taken, now with three added costs to account for.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–85. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click line, no
clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (five claims, each failing a store-only or name-blind implementation), pass 2 **GREAT**
(the compiler-forced construction sites explained as the right trade, and the path limitation named), pass 3 **GREAT**
(drop a CSV, build, and each failure mode is a sentence).

**Nothing in `semantic::` is now unwired.** Every reader is called from a production-shaped path.

**Docs.** The guide says a build reads the sidecar and names the three reports; the README line says read during the
build; `CHANGELOG.md` gains the join and the three facts.

**Evidence.** `runs/2026-11-03-86/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `d3ed633`.

**Top 5 remaining**

1. **The binary index writer** ADR 0004 specifies. The text form is **31.88 MB** at 5 000 clips — plus provenance and
   loudness sections now — against **10.30 MB** binary. The 3× overhead has been quoted for four iterations and never
   asserted by a test.
2. **`#36` indexing throughput.** `build` now embeds, measures loudness, parses a sidecar and joins it; three costs were
   added and the number that says whether they are affordable has never been taken.
3. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and cheapest now that
   `embed_measured` holds the exact buffer they need.
4. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index that now
   produces its own vectors, loudness and provenance.
5. **`#27` hard filters and penalties** — the consumer of everything built since iteration 77: loudness for the 12 LU
   rule, tempo and key for theirs. Its own criteria require that an unknown feature never excludes an asset, which the
   provenance and loudness types already model as `None`.

## Iteration 85 — 2026-11-03 — measuring in the build, and the block count that was wrong in three places

**Baseline.** Green, complete, uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **362 semantic** / 36 lib; clippy 0,
fmt, rustdoc, examples. Volume held **21 GB** free.

**The item was iteration 84's first leftover**, and it was the "built but not wired" shape this loop keeps finding:
`loudness::measure` had been tested since iteration 77 and **nothing called it**, so the index stored only what a caller
supplied.

**`embed_measured` shares one decode between the measurement and the embedding.** `embed_one` already decoded,
downmixed and resampled to the model's rate and handed that buffer to the embedder — which is exactly what `measure`
needs. A second pass would double the cost of the operation the whole feature exists to make cheap on a thousand-file
library. One decode, two consumers, neither keeps the buffer.

**A failed measurement is not a failed file**, because the vector is the index's reason to exist — but it is **counted**:
`Built` gained `unmeasured`, and `describe()` prints it only when non-zero, so a run that measured nothing cannot look
complete. That is the same rule as a silent refusal: the failure exists to be read.

**The defect is in shipped documentation, and a lower bound was hiding it.** `loudness.rs` claimed a 3 s clip has **7**
gating blocks; its own test message said the same. Both are wrong — 7 is the *non-overlapping* count. A 75 %-overlapping
block advances 100 ms, so

    1 + (144000 − 19200) / 4800 = 27

The existing assertion was `blocks >= 7`, **which passes on 27** and therefore never noticed. **A lower bound cannot
detect a wrong sentence, only a wrong magnitude.** The new test asserted the exact 27 and failed with `left: 27, right:
7`. Corrected in three places, and the doc table now says **1 s**, where 7 is the right number and the contrast it draws
(1 s of silence has 7 blocks and 0 above the gate; 0.2 s has 0 possible) still holds.

**Measured.** A 3 s, 0.5-amplitude 1 kHz tone stores ≈ **−9.0 LUFS** with **27** blocks and `unmeasured == 0` — anchored
on the **−23.0 LUFS** the 0.1-amplitude tone gives (measured in iteration 77; `20·log10(5) = 13.98 dB`). A 0.2 s click
stores `integrated: None` with a finite peak, through a real build rather than only through `to_text`.

| voices | µs/frame | iteration 84 |
|---|---|---|
| 8 | **0.063** | 0.062 |
| 64 | **0.365** | 0.352 |
| 256 | **0.963** | 0.501 |

**No regression, and no claim to measure one** — the change is inside `semantic`, which the default build does not
compile. Across iterations 80–85 the same audio code reads 0.062–0.084 / 0.342–0.472 / **0.501–1.576** µs; the 256-voice
figure has now ranged over 3× on unchanged code, which is the machine rather than the crate.

**The one real cost is scan-side and unmeasured here:** one IIR pass per file over a buffer already in hand. `#36`
(indexing throughput) owns that number, and it now has a real build to measure.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80–84. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click line, no
clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (the build produces a reference-anchored number, and `unmeasured` makes a silent failure
visible), pass 2 **GREAT** (the documentation defect found, explained, and the mechanism that hid it replaced), pass 3
**GREAT** (nothing required of a caller; `describe()` reads the same when healthy).

**Docs.** The guide says a build produces these numbers and shows `describe()`; the README line says measured during
the build; `CHANGELOG.md` gains both the addition and the documentation fix.

**Evidence.** `runs/2026-11-03-85/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `a6a8731`.

**Top 5 remaining**

1. **Join provenance's file names to content hashes.** `Sidecar::read` is the last unwired reader, for the same reason
   `measure` was: it needs the scan's path→hash map joined to the sidecar's name→record map. This is the step that makes
   `build` produce provenance as it now produces loudness.
2. **The binary index writer** ADR 0004 specifies. The text form is **31.88 MB** at 5 000 clips against **10.30 MB**
   binary, and with three sections it is larger still; the 3× overhead is still unasserted.
3. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and now straightforward because
   `embed_measured` is the natural place to compute them from the same buffer loudness already uses.
4. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index that now
   produces its own loudness, provenance and vectors.
5. **`#36` indexing throughput** — `build` now measures as well as embeds, so the scan got heavier; the number that says
   whether that was affordable has never been taken.

## Iteration 84 — 2026-11-03 — loudness stored, and the last p0 closed

**Baseline.** Green, complete and uninterrupted: 198 / 207 serde / 208 mixer / 6 doc / **359 semantic** / 36 lib;
clippy 0, fmt, rustdoc, examples. The volume held **24 GB** free throughout — iteration 83's cleans ended the six-iteration
disk siege.

**#21 closed — the last open p0 in the repository.** Its fourth criterion, and the previous three were already measured
in iteration 77.

| Criterion | State |
|---|---|
| integrated within **0.1 LU** of the reference | green since 77 — −23.0 ±0.1 against `ffmpeg`'s −23.0 |
| a short clip handled **explicitly** | green — `None` in memory, `-` on disk |
| true peak within **0.1 dB** | green since 77 — −20.00 dBFS against −20.0 |
| both survive a round trip | **green — this iteration** |

**Why a dash and not a number.** ADR 0003 measured that for a 0.2 s clip `ffmpeg` reports **−70.0 LUFS** — the standard's
floor presented as a level, *and also a real level* — while `ebur128` reports **`-inf`**. A storage format that writes a
number there destroys the distinction the in-memory type was carefully built to keep. So the lufs column writes `-`, and
everything else in the record is a real measurement even when the clip is too short to gate: a 0.2 s click has a true
peak, so it is always written, along with the two block counts that separate *too short* from *silent*. Both still
disagree after a write and a read.

**Two defects the new tests caught, neither by reading the code.**

1. **The section marker was parsed as a record.** With provenance written first, `provenance_and_loudness_sections_coexist`
   failed with `Record { line: 11, what: "a provenance record names no hash" }` — the `loudness 1` line arrived while
   the provenance section was open and was consumed as one of its records. Markers are now recognised **before** any
   record parsing, so sections may appear in any order.
2. **My own test asserted behaviour the format does not have.** It expected `INFINITY` to round-trip through `-`. The
   parser requires a number in that column and **refuses the whole file by name** — substituting `0.0` would look like
   a measured silence, which is the class of lie the section exists to prevent. The test now asserts the refusal.

| voices | µs/frame | iteration 83 |
|---|---|---|
| 8 | **0.062** | 0.065 |
| 64 | **0.352** | 0.358 |
| 256 | **0.501** | 1.038 |

**No regression.** Across iterations 80–84 identical audio code reads 0.062–0.084 / 0.342–0.472 / **0.501–1.576** µs;
the 256-voice low end moved by 2× this time, which is the clearest evidence yet that this machine's variance exceeds
anything these iterations could cause — so the table shows a range rather than a trend.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80, 81, 82 and 83. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no
click line, no clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (all four criteria met, three against a reference), pass 2 **GREAT** (both defects named,
and the second is the test's fault rather than the code's), pass 3 **GREAT** (nothing required of a consumer; no public
type changed this iteration).

**Docs.** The guide's sixth step gains the loudness story and why the dash exists; the README line says loudness is
carried with absence preserved; `CHANGELOG.md` gains the entry.

**Evidence.** `runs/2026-11-03-84/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `31b731b`.

**No open p0 remains.** What is left is p1 and below, and the two p0-shaped remainders are both scan-side wiring rather
than decisions.

**Top 5 remaining**

1. **Measure loudness and join provenance during indexing.** `loudness::measure` and `Sidecar::read` both exist and are
   tested; neither is called by `indexer::build` yet. This is the step that makes both features produce their data
   instead of only storing it — and it is where a sidecar's *file name* becomes a *content hash*.
2. **The binary index writer** ADR 0004 specifies. The text form is **31.88 MB** at 5 000 clips against **10.30 MB**
   binary, and with two new sections it is larger still; the 3× overhead is still unasserted.
3. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and the cheapest of the three
   hand-rolled features because they share one FFT pass over a spectrum `mel.rs` already computes.
4. **`#28` the MCP tool host**, then `#29`/`#44`/`#30` — the largest user-facing win, consuming an index that is now
   correct, incrementally updatable, provenance-carrying and loudness-carrying.
5. **`#16`'s follow-up: `#36` indexing throughput**, which now has a real index to measure — and a real reason to, since
   the scan-side work above will make `build` do more per file.

## Iteration 83 — 2026-11-03 — provenance stored and reported, and the fixture that tested the wrong thing

**Baseline.** Green, complete: 198 / 207 serde / 208 mixer / 6 doc / 355 semantic / 36 lib; clippy 0, fmt, rustdoc,
examples. Two disk interruptions handled with the documented fix — `cargo clean -p bevy_siren` freed **5.1 GB** then
**6.1 GB** — after which the volume read **27 GB free**, far more than either clean returned, so something else on the
machine released space too.

**#14 closed, all four criteria.** The two that were blocked last iteration needed only their own decision file's
recommendation, which was additive — and this repo's log has said three times that such a recommendation *"need not
wait."* Acting on it rather than waiting is what closed the issue.

**The format, and why each part is forced.**
- The section is a trailing block **inside the checksummed body**, so a hand-edited licence is a corrupt file rather
  than a silent change to which assets may ship.
- An index with **no provenance writes no section at all**, so the new format is a strict superset: `MAGIC` does not
  move and every old file is valid.
- Fields are **escaped**, because a pack called `kenney sounds` would split into two fields, and a licence of `-` would
  read as *nothing recorded* — the conflation the type refuses in memory, reintroduced by the file format. `?unknown`
  likewise keeps a recorded-unknown source distinct from an absent one, all the way to disk.

**`Match` gained `provenance` and lost `Copy`**, so a result cannot be built without one: forgetting to ask is a compile
error rather than an unlicensed asset presented as unencumbered. The decision file predicted a blast radius of one
literal and no by-value iteration; **the prediction held exactly.**

**Two defects found inside the iteration.** `a_search_result_carries_its_provenance` inserted hashes `1` and `2` on top
of `index_with(2)`'s existing `0x1000` pair, so four vectors ranked and the assertion read **`left: 4097, right: 2`** —
**the test was wrong, not the code**, and it now builds a fresh index. A fixture that silently adds to a fixture is how a
test starts passing for the wrong reason. And `clippy::bool_comparison` caught `is_recorded() == false`.

**Measured.** Provenance round-trips field for field including a pack name with a **space**; a recorded-empty licence
comes back `Some("")` and an absent one `None`, after a write **and** a read; a truncated section is refused by the same
count rule as a truncated vector body (re-checksummed in the test so the count is the only thing wrong).

| voices | µs/frame | iteration 82 |
|---|---|---|
| 8 | **0.065** | 0.062 |
| 64 | **0.358** | 0.342 |
| 256 | **1.038** | 1.227 |

**No regression.** Across iterations 80–83 identical audio code reads 0.062–0.084 / 0.342–0.472 / 1.038–1.576 µs — a
spread on a shared machine, reported as a spread. The one new cost is a type, not a loop: `Match` lost `Copy`, and
`top_k` was already cloning into a `Vec`.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80, 81 and 82. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click
line, no clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (three criteria met; the fourth now has both halves except the filename→hash join), pass 2
**GREAT** (the fixture defect found and named as the test's fault), pass 3 **GREAT** (one `&` for a consumer to add, and
it is a compiler error rather than a surprise).

**Docs.** The guide's sixth step gains the storage story, the `Copy` trade and a one-line example of iterating the new
`Match`; the README line says stored and reported; `CHANGELOG.md` gains the format description.

**Evidence.** `runs/2026-11-03-83/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `8a323a1`.

**Top 5 remaining**

1. **Join a sidecar's file names to content hashes** — the last half of #14's criterion 2, and it belongs where the scan
   and the sidecar meet: `indexer::build`. The type, the reader and the store are all done.
2. **`#21`'s index round trip** — `loudness-in-the-index.md` recommends the *same* optional-section shape this iteration
   just implemented and proved, so the decision is now nearly free and the implementation is a sibling of this one.
3. **The binary index writer** ADR 0004 specifies — `to_text` is **31.88 MB** at 5 000 clips against **10.30 MB** binary,
   and with a provenance section added the text form is larger still. The 3× overhead is unasserted.
4. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, sharing one FFT pass over a spectrum
   `mel.rs` already computes.
5. **`#28` the MCP tool host**, then `#29`/`#44` — the largest user-facing win, consuming an index that is now correct,
   incrementally updatable and provenance-carrying.

## Iteration 82 — 2026-11-03 — provenance, and the licence check that would have granted permission

**Baseline.** Green: 198 / 207 serde / 208 mixer / 6 doc / 36 semantic; clippy 0, fmt, rustdoc, examples. One disk
interruption handled with the documented fix — `cargo clean -p bevy_siren` freed **4.9 GB** after a serde link failure at
1.4 GB free, which then ran **207 passed**.

**The item: #14, the last unblocked p0.** p0 *because* it is cheap now and impossible later: an index built without
provenance can only be recovered by rescanning files that may be gone.

**Two of its four criteria are met; two are genuinely blocked**, and the split is recorded rather than glossed.

| Criterion | State |
|---|---|
| explicit unknown, distinguishable from a recorded empty value | **met** |
| a sidecar's fields reach the index | **half** — the reader is built; *reaching the index* is a schema change |
| a search result reports licence and source | **blocked** — `rank::Match` is `Copy` |
| unknown is never presented as permissive | **met** |

**The design worth defending.** `permits_commercial_use() -> Option<bool>` has **three** answers, and `None` means *a
person decides*. A `bool` defaulting to `true` turns *"nobody recorded this"* into *"this is fine"* — the direction that
gets a studio sued — and a `bool` defaulting to `false` would call `CC0` unusable. An empty string **is** a recording
and stays distinguishable from an absent one, and both answer `None`.

**The over-claim this iteration nearly shipped.** The permissive check was `lower.contains("mit")` — a **substring**
match — so `"resubmitted-terms"` would have been accepted as the MIT licence. **Every test passed as written**, because
none used a string that happened to contain the letters. Fixed to a **leading stem**, with `resubmitted-terms → None`
pinned. The asymmetry is deliberate and now stated: **non-commercial is a substring** (a missed `-nc` is dangerous),
**permissive is a stem** (a spurious match is dangerous). `CC-BY-SA-4.0` and `CC-BY-ND-4.0` also read correctly as
commercial — share-alike and no-derivatives are separate obligations — where my own first test had mis-asserted
`CC-BY-4.0` as `None`.

**The pitfall #14 names, refused in code rather than in prose.** `Sidecar` reads only a per-file record. A folder-level
`LICENSE` or `README` is **never consulted**, so a pack whose README says "all CC0" gets unknown provenance for every
file — an inherited licence that looks authoritative is worse than an honest unknown. Columns are matched **by name**, so
a pack that orders them differently still reads; two rows for one file is a **conflict**, not last-write-wins.

**Carved off with a decision file, not silently.** Storing provenance in the index and exposing it on `Match` are both
schema changes — `Index::records` is `hash → vector` under a pinned `MAGIC`, and `Match` is `Copy { hash, score }` — so
they are `needs-decision/provenance-in-the-index.md`, recommending an optional trailing section (the shape ADR 0004 chose
for loudness) and accepting that `Match` loses `Copy`, because a caller looking provenance up separately is the option
that lets a consumer forget.

| voices | µs/frame | iteration 81 |
|---|---|---|
| 8 | **0.062** | 0.064 |
| 64 | **0.342** | 0.355 |
| 256 | **1.227** | 1.341 |

**No regression, and no claim of an improvement** — a new module behind a non-default feature; across iterations 80–82
identical audio code reads 0.062–0.084 / 0.342–0.472 / 1.227–1.576 µs, which is the shared machine's spread.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75, 80 and 81. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no click line,
no clipping shelf, no gap.

**Verdicts.** Pass 1 **GREAT** (two criteria met, two blocked and tracked), pass 2 **GREAT** (the over-claim found by
re-reading the matching, and pinned as a test), pass 3 **GREAT** (a sidecar author writes CSV or JSON and it reads;
the pitfall is refused in code).

**Docs.** The guide gains a sixth step — *where each sound came from* — with the three-answer match and the CSV shape;
its heading moved from "five steps" to "six". The README line gains the licence claim. `CHANGELOG.md` gains the entry.

**Evidence.** `runs/2026-11-03-82/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `1e0cfd9`.

**Top 5 remaining**

1. **Answer `provenance-in-the-index.md` and finish #14.** Two criteria are waiting on it, and the recommendation mirrors
   ADR 0004 so the two features share one rule. `Match` losing `Copy` is the only consumer-visible cost.
2. **`#21`'s index round trip** — `loudness-in-the-index.md` recommends the same optional-section shape, so one decision
   can settle both.
3. **The binary index writer** ADR 0004 specifies — `to_text` is **31.88 MB** at 5 000 clips against **10.30 MB** binary,
   a 3× overhead still unasserted.
4. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, sharing one FFT pass over a spectrum
   `mel.rs` already computes.
5. **`#28` the MCP tool host**, then `#29`/`#44` — the largest user-facing win, consuming an index now correct,
   incrementally updatable and provenance-carrying.

## Iteration 81 — 2026-11-03 — incremental indexing, and the file the scan refused

**Baseline.** Green, and complete: 198 / 207 serde / 208 mixer / 6 doc / 25 semantic / 6 incremental; clippy 0, fmt,
rustdoc, examples. Two disk interruptions, both handled with the fix the skill now carries — `cargo clean -p bevy_siren`
freed **4.3 GB** twice — and the `--features mixer` doctest **link** failures at 229 MB free re-ran to **208 passed**
after cleaning, confirming they were the disk and not the code.

**The first feature in seven iterations.** #16 was p0, unblocked, and every part already existed: a scan keyed by
content hash, an index keyed by content hash, and `stale`/`vanished` answering each side of the set difference. Nothing
composed them. `semantic::incremental::update` is the composition.

**The identity rule falls out of a property, not a case.** `Index::records` is `hash → vector` and stores **no paths**,
so a rename is *indistinguishable from a file that did not move* and costs nothing **by construction**. That is stronger
than a rule correctly implemented: there is nowhere for it to be wrong. A **re-encode is a new asset** — the bytes
changed, so the hash did — and that is asserted, because the crate cannot tell "re-encoded" from "a different take" and
the opposite guess would leave the index describing audio that is gone.

**Each acceptance criterion is one test, and each would fail a wrong implementation.** The add asserts the **embedder's
call count is 1**, so re-embedding everything and discarding extras cannot pass. The delete asserts the **hash is
absent**, not that the length fell — which would pass for dropping the wrong vector. The rename asserts `calls == 0` and
that every vector is **identical**, not merely close.

**The iteration's own test found a real defect in its first draft.** `an_undecodable_file_is_failed_not_counted` failed:
a corrupt `broken.wav` appeared in **no count at all**. `update` iterated `library.assets` and ignored `library.skipped`,
so a file that *is* audio by extension and would not read vanished silently — **the exact silence this crate refuses at
every other layer**, reproduced in the newest code. `indexer::build` already split those skips by meaning; the
incremental path now does too. The test caught it because it asserts a **count**, not that something sensible happened.

**Measured.** Adding one file to a 100-file library: **1 embedder call**, `1 added, 0 removed, 100 unchanged`. A rename:
**0 calls**, every vector identical, `is_noop()` true so nothing is written. A deletion: **0 calls**, the vector gone by
hash. A re-encode: **1 add and 1 removal**, never "unchanged".

| voices | µs/frame | vs iteration 80 |
|---|---|---|
| 8 | **0.064** | 0.084 |
| 64 | **0.355** | 0.472 |
| 256 | **1.341** | 1.576 |

Faster, and **not because of this change** — nothing in the audio path moved. Two measurements of identical code now read
0.064–0.084 / 0.355–0.472 / 1.341–1.576 µs, so the honest reading is **no regression**, not an improvement.

**Visual gate unchanged:** 32 render files, **14 of 14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to
iterations 67, 71, 74, 75 and 80. `bang-spectrum.png` read again: broadband onset, dense harmonic comb across 0–10 kHz,
no click line at t=0, no clipping shelf, no gap; the tail window shows the same harmonics decaying smoothly.

**Verdicts.** Pass 1 **GREAT** (four criteria, four tests, each non-vacuous), pass 2 **GREAT** (the draft's silent-drop
found and fixed inside the iteration), pass 3 **GREAT** (`update` + `write` is the whole workflow, and the doc explains
why there is no `moved` field instead of leaving a reader to wonder).

**Docs.** The guide gains a fifth step — *re-indexing embeds only what changed* — with the composition in four lines, and
its heading corrected from "four steps" to "five". The README line gains the measurement. `CHANGELOG.md` gains the entry.

**Evidence.** `runs/2026-11-03-81/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,spectrograms}/`.

**Commits.** `dfb8b9c`.

**Top 5 remaining**

1. **`#14` per-asset licence and source metadata** — the last unblocked p0, unblocked by ADR 0005 and ADR 0004's key
   policy. `Asset` already carries `paths`, `hash` and `bytes`; provenance is the next field.
2. **`#21`'s index round trip** — `needs-decision/loudness-in-the-index.md` recommends an optional `loudness` section,
   and ADR 0004 has since settled the store it lives in, so the decision can be made against a known format.
3. **The binary index writer** ADR 0004 specifies — `to_text` is **31.88 MB** at 5 000 clips against **10.30 MB** binary,
   a 3× overhead no test asserts today.
4. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003, and the cheapest of the three
   hand-rolled features because they share one FFT pass over a spectrum `mel.rs` already computes.
5. **`#28` the MCP tool host**, then `#29`/`#44` — the largest user-facing win, and now consuming an index that can be
   kept up to date incrementally.

## Iteration 80 — 2026-11-03 — the second ratchet, and the disk actually solved

**Baseline, and the first thing worth saying: it now runs.** `cargo test` had not run for **five consecutive
iterations** — `errno=28` every time — and it was **red**:

```
library::the_semantic_feature_declares_no_dependency_of_its_own ... FAILED
  left:  ["dep:ort", "dep:rodio", "dep:rubato", "dep:ebur128"]
  right: ["dep:ort", "dep:rodio", "dep:rubato"]
```

**That red was mine, from iteration 77.** This crate guards its dependency list twice: `tests/leaf.rs` reads the
`[dependencies]` table and asks *"may this crate exist here?"*, and `tests/library.rs` reads the `semantic` feature's
value and asks *"does the feature pull anything beyond what we decided?"*. Iteration 77 added `ebur128` to the manifest,
to the feature **and** to `ALLOWED_DEPS`, and missed the third. **Two guards, one updated.** Both messages name each
other and say *"in the same commit"*, which is why the fix was one string and a comment.

**The disk, actually solved rather than worked around.** The two iterations before this one spent their time on
`cargo clean -p` of **dependency** artifacts — the wrong lever, which made every later build need a fresh copy and dug
the hole deeper. The right lever is this crate's own output:

    $ cargo clean -p bevy_siren --target-dir target
         Removed 645 files, 6.3GiB total

`target/debug/examples` alone held **4.2 GB** of stale binaries. One command took the volume from **122 MB to 6.2 GB
free**, touching no dependency, and every step below then ran. **The fix is now in the skill's machine facts**, with the
measurement and the instruction not to touch other apps' caches.

| Step | Result |
|---|---|
| `cargo test` | **198 passed** |
| `cargo test --features serde` | **207 passed** |
| `cargo test --features mixer` | **208 passed** |
| `cargo test --doc` | **6 passed** |
| `cargo test --lib --features semantic` | **20 passed** |
| clippy `-D warnings` / fmt / rustdoc / examples | **0 warnings**, all pass |

**Case 19, measured for the first time since iteration 75.** `perfcheck` on an idle M4:

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | **0.084** | 0.0105 |
| 64 | **0.472** | 0.0074 |
| 256 | **1.576** | 0.0062 |

The per-voice cost **falls** with count, which is what a fixed per-frame cost amortised must do. `perfcheck`'s own
footer calls it an upper bound — every voice is costed, including the ones `update_mix` skips.

**The visual gate, looked at.** 32 render files, **14 of 14 distinct** cue hashes, master **−21.5 LUFS** and
**−7.5 dBFS** — identical to iterations 67, 71, 74 and 75. `bang-spectrum.png` shows a broadband transient filling the
first ~35 ms and discrete harmonics decaying out of 2–7 kHz by ~0.21 s, with **no vertical line at t=0** (a click),
**no flat shelf at 0 dBFS** (clipping) and **no hole** (a dropped voice); the attack and tail windows at 50 and 100 ms
show a smooth onset-to-decay transition. The footstep's partials are visibly narrower and decay faster, which is the
difference the two cues exist to have.

**Before/after is byte-identical.** `cue-bang-0.wav` and `mix.wav` hash the same as iteration 75's render — a proof, not
a measurement, that the audio path could not have moved.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT** (the `--voices 64` re-render reports its own
**+2.9 dBFS / 290 samples past full scale** honestly, which is the renderer doing its job rather than a defect).

**Evidence.** `runs/2026-11-03-80/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/{after,before,fresh,spectrograms}/`.

**Commits.** `00e8bbd`.

**Top 5 remaining**

1. **`#16` incremental indexing by content hash** — p0, blockers closed, and ADR 0004 has fixed the format it writes
   into. The first item in six iterations that is a build rather than a repair or a document.
2. **`#14` per-asset licence and source metadata** — p0, unblocked by ADR 0005 and ADR 0004's key policy.
3. **`#21`'s index round trip** — `needs-decision/loudness-in-the-index.md` can now be answered against ADR 0004's
   settled store.
4. **The binary index writer** ADR 0004 specifies, with a test asserting the **10.30 MB** figure rather than accepting
   it — and the text form's 3× overhead is a fact no test asserts today.
5. **`#22` spectral centroid, rolloff and noise floor** — unblocked by ADR 0003 and the cheapest of the three
   hand-rolled features, since they share one FFT pass.

## Iteration 79 — 2026-11-03 — the vector store, sized rather than guessed

**Baseline.** All cargo steps **BLOCKED by disk**: 148 MB free at the start, `cargo clean -p bevy_pbr` buying 326 MB,
and one 7 KB `write` failing `ENOSPC` outright before that. **Fifth consecutive iteration the disk has cost.**

**#5 closed: ADR 0004, `docs/adr/0004-vector-store.md`.** One versioned file, read whole, brute force. No SQLite, no
shard directory, no ANN.

**The measurement corrected the issue's own premise.** #5 asks whether a game ships *"a 5 MB index as an asset or a
40 MB SQLite file"*. Computed from the crate's **own** `to_text` (mean **6 376.8 bytes** per record, measured over 2 000
records of dimension 512):

| Clips | Text (exists today) | Binary f32 | Binary f16 |
|---|---|---|---|
| 500 | 3.19 MB | 1.03 MB | 0.52 MB |
| **5 000** | **31.88 MB** | **10.30 MB** | **5.20 MB** |
| 50 000 | 318.84 MB | 102.99 MB | 52.01 MB |

**The 40 MB figure is the format the crate already has**, and 5 MB needs `f16`. An ADR written from the issue's numbers
would have chosen a store to avoid a size it is already at.

**Query cost, measured:** a BLAS `f32` dot over 5 000 × 512 takes **0.05–1.87 ms**. Brute force is not the slow option
at this scale — that is the whole argument against an ANN, and it is a measurement rather than a preference.

**Identity and its four rules.** Content hash, with rename keeping the vector, re-encode being a new asset, duplicate
bytes being one vector with two paths, and a 64-bit collision **detected** by the recorded file size rather than
assumed away.

**`f16` measured and declined**, which is the decision worth keeping: it halves the file to 5.2 MB and preserved one
query's top 10 **exactly**, and yet `rank::top_k` compares against a `floor` — one query says nothing about behaviour at
that boundary. Deferred to `V-01`'s labelled set.

**A query needs no writable filesystem**, stated as the issue requires and demonstrated by mechanism: `File::open` plus
`read_to_end`, no `MAP_SHARED`, no lock.

**Verdicts.** Pass 1 **GREAT** (all four criteria met), pass 2 **GREAT** (every size checked to the byte; the wide
cold/warm latency range stated as a range), pass 3 **GREAT** (the correction of the issue's premise sits before the
decision, where a reader needs it).

**Gate.** Nothing compiled — docs-only, `git diff HEAD -- src/ tests/ Cargo.toml` is **0 files**. The baseline gap is in
`eval.md`, not claimed green.

**Evidence.** `runs/2026-11-03-79/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md`,
`evidence/store-sizes.txt`.

**Commits.** `042796d`.

**The disk, with the consumer named at last.** The volume reports 461 GB used / 148 MB free, and the culprit is outside
this repo: `~/.cache/huggingface` is **50 GB**, of which **48 GB is two FLUX image-model checkpoints**
(`FLUX.2-klein-9B` 33 GB, `FLUX.2-klein-4B` 15 GB). `~/bevy_siren/target` is 16 GB and `~/.cargo` 8.7 GB. Deleting
either checkpoint recovers more than a full release build needs; neither is a build artifact, so it is the owner's
decision, not this loop's.

**All three unblocked p0 ADRs are now closed** — #4 (acoustic features), #5 (vector store) and #6 (models and licences).
What remains is implementation, which needs a compiler.

**Top 5 remaining**

1. **Free the disk.** Everything below is blocked on it. The named candidates are in the paragraph above; this is a
   one-command fix and it is the only thing standing between five iterations of documents and five of code.
2. **`#16` incremental indexing by content hash** — p0, blockers closed, and ADR 0004 has just fixed the format it
   writes into.
3. **`#14` per-asset licence and source metadata** — p0, unblocked by iteration 78 and by ADR 0004's key policy.
4. **`#21`'s index round trip** — `needs-decision/loudness-in-the-index.md` is now a decision *inside* ADR 0004's
   format, so it can be answered against a settled store.
5. **The binary index writer** ADR 0004 specifies, with its test asserting the 10.30 MB figure rather than accepting it.

## Iteration 78 — 2026-11-03 — every music encoder is non-commercial, and the corpus has no licence

**Baseline.** `cargo fmt --check` green. `cargo test --lib --features semantic` and every other step **BLOCKED by
disk**: 122–152 MB free all iteration, `cargo clean -p` buying 1.2 GB that `bevy_pbr`'s archive consumed in one link.
`du` over the home directory now times out at 120 s at this occupancy, so even finding the consumer needs a decision
about what may be deleted. **Fourth consecutive iteration the disk has cost**, and this is the first where the item was
chosen *because* it needs no build — a workaround, not a fix.

**The item: #6, closed.** p0, blocked by nothing, blocking #10, #39 and #14. A document, so no build was needed to
finish it, which is exactly what this machine could do.

**What the audit found, read from each model card on 2026-11-03.**

| Model | Licence | Commercial |
|---|---|---|
| `laion/clap-htsat-fused`, `-unfused` | **Apache-2.0** | **yes** |
| `m-a-p/MERT-v1-95M`, `-v1-330M` | **CC BY-NC 4.0** | **no** |
| `OpenMuQ/MuQ-large-msd-iter`, `MuQ-MuLan-large` | **CC BY-NC 4.0** | **no** |
| `musicfm-v-demo`, `audio-flamingo-hf`, `EF-CLAP` | **no licence tag** | **no grant** |

**Every music-specific encoder on the shortlist is non-commercial.** That retires #39's premise — it asks for exactly
that encoder — before #39 was built, which is the entire reason a p0 ADR has to arrive first.

**And #6's own insistence on auditing the corpus was right.** CLAP's weights are Apache-2.0, but `LAION-audio-630k`
has **no licence file at all** — the repo's `LICENSE` path returns `404` and the GitHub API reports `license: null` —
while its README documents eight sources including **Epidemic Sound, Audiostock, Sonniss and Paramount**, several
purchased by LAION. Recorded as an **open risk with a named escape hatch** (consume an index as data via `ModelStamp`)
rather than resolved by assertion.

**Decision.** CLAP only; no weights vendored; MERT and MuQ are **blockers with the fallback named** (CLAP's text tower
plus anchors). Weights arrive at **first use**, cached by content hash, pinned to a **commit sha**, verified against a
**SHA-256 recorded in the crate**, and every failure — offline, hash mismatch, download forbidden — is a refusal naming
the model rather than a silent fallback to different weights.

**A gap stated rather than implied.** `ModelStamp` does **not** carry a pinned sha today; its own doc says the version
*"names the measurement rather than a tag, because the spike did not pin a revision"*. The ADR records the pinning
story as a **requirement on the fetch**, not a description of what exists — so nobody trusts two machines' indexes to
be comparable before that lands.

**Verdicts.** Pass 1 **GREAT** (all four acceptance criteria met, each with a URL and a date), pass 2 **GREAT** (the
corpus risk is named, not buried; the `ModelStamp` gap is admitted), pass 3 **GREAT** (a reader can see why #39 is
blocked in one table). The baseline gap is stated in `eval.md`, not claimed green.

**Gate.** fmt green; everything else **unrun, disk**. The change is docs-only — `git diff HEAD -- src/ tests/
Cargo.toml` is **0 files** — so no compiled behaviour moved.

**Evidence.** `runs/2026-11-03-78/`: `research.md`, `plan.md`, `eval.md`, `perf.md`,
`evidence/model-licences.txt` (the raw API output, so a later reader can re-run it).

**Commits.** `e938e4f`.

**Top 5 remaining**

1. **Free the disk — this is now the only item that blocks everything else.** Four iterations have lost steps to
   `errno=28`, and this one lost the entire baseline. It needs a human decision about what may be deleted: the volume
   reports 461 GB used and 122 MB free, `~/bevy_siren/target` is 16 GB, and `du` cannot even finish enumerating the
   rest at this occupancy. Until that is answered, every iteration ships documents rather than code.
2. **`#16` incremental indexing by content hash** — p0, its blockers (#11, #15) are closed, and `library.rs`'s `fnv1a`
   is already its seed. The next implementable p0.
3. **`#14` per-asset licence and source metadata** — unblocked by this iteration; needs the same "explicit unknown"
   discipline the ADR just applied to models.
4. **`#21`'s index round trip** — waiting on `needs-decision/loudness-in-the-index.md`.
5. **`#5` the vector-store ADR** — p0 and unblocked, and like this one it needs no build.

## Iteration 77 — 2026-11-03 — the acoustic-features ADR, and loudness with the short clip as a state

**Baseline.** `cargo build` and `cargo test --lib --features semantic` (20 tests) green; `cargo fmt` green. Everything
else **blocked by disk**: the volume sat at **100 %, 122–230 MB free** and `cargo test`, clippy, rustdoc and the
examples all died in `ld: write() failed, errno=28` or `rustc-LLVM ERROR: IO failure`. `cargo clean -p` on the heavy
deps bought ~1.3–1.7 GB per attempt, which the next link consumed. **Third iteration in a row the disk has cost** —
recorded as its own blocker below.

**Two items, in dependency order.** [#4](https://github.com/bevy-coven/bevy_siren/issues/4) is p0, blocks #21–#25, and
is a document the disk cannot block — so it went first. Then the issue it unblocked.

**#4 closed: ADR 0003, `docs/adr/0003-acoustic-features.md`.** All six features are Rust; no sidecar is built. The
load-bearing fact is that **`ebur128` 0.1.11 is pure Rust** despite its name — nine `src/*.rs`, C confined to
`capi-test/` and `tests/c/`, `build.rs` gated on `c-tests` which is off by default. **Verified, not assumed:** a clean
`default-features = false` build takes **1.66 s**, invokes no `cc`, produces no C object. Tempo ships with a **±2 BPM**
floor (**design default**) that is a test and removes the feature if unmet; **key is out of scope** rather than shipped
at an accuracy no source reports.

**The measurement changed the decision, which is the point of measuring.** On two long clips `ebur128` agrees with
`ffmpeg`'s reference to **0.01 LU** (−23.01 vs −23.0; −8.92 vs −8.9). On a **0.2 s** clip they disagree entirely and
**neither number is a measurement**: `ffmpeg` reports **−70.0 LUFS**, the standard's floor presented as a level, and
`ebur128` returns **`-inf`**. `relative_threshold()` returns exactly −70.0 with no blocks — the same value a genuine
−70 LUFS passage gives. That is now a **requirement on #21** rather than a discovery inside it.

**`semantic::loudness`.** Integrated loudness is `Option<f32>` because a clip too short for a 400 ms block has no
loudness, and the module's doc opens with the three-way table above. Measured against the reference: **−23.0 LUFS**
±0.1 on a 3 s tone, **−20.00 dBFS** true peak, **13.98 dB** between 0.5 and 0.1 amplitude (`20·log10(5)`), and 0.2 s →
`None` with its peak still measured at −6.02 dBFS.

**The defect it found in its own first draft, worth keeping.** One `blocks` field cannot distinguish *"too short"* from
*"silent"*: `ebur128`'s count is blocks **above the gate**, so 3 s of digital silence and a 0.2 s footstep both returned
0. The module was reproducing the exact conflation it existed to prevent, one level down. Its own silence test caught
it. Now `blocks` and `blocks_possible` are separate, with `too_short()` and `silent()` as separate predicates asserted in
opposite directions on the two clips.

**Carved off with a decision file, not silently.** #21's last criterion — the index round trip — changes the versioned
`bevy_siren index 1` text format, so it is `needs-decision/loudness-in-the-index.md`: three options, a cost table, and a
recommendation for an optional `loudness` section because **absence must be encoded as absence, not as a sentinel
number in a numeric column**.

**Verdicts.** Pass 1 **GREAT** (the ADR meets all four criteria), pass 2 **GREAT** (the measurement is read closely and
its own defect is named), pass 3 **GREAT** (a developer can see why the type is an `Option` in the first twenty lines),
with the baseline gap stated in `eval.md` rather than claimed green.

**Gate.** 20 semantic tests, fmt, `cargo build` — green. Default suite, serde, mixer, doc, clippy, rustdoc, examples,
`perfcheck`, `render` — **unrun, disk**.

**Evidence.** `runs/2026-11-03-77/`: `research.md`, `plan.md`, `eval.md`, `perf.md`, `review.md` (three passes),
`evidence/loudness-vs-reference.txt`, `evidence/ebur128-probe.rs`.

**Commits.** `f584097` (ADR, closes #4), `63ba157` (loudness).

**Top 5 remaining**

1. **Clear the disk, then run the full baseline.** Three iterations have now lost steps to `errno=28`, this one losing
   clippy, rustdoc, the examples and both measuring gates. Nothing else on this list is worth as much as a machine that
   can build.
2. **`#21`'s index round trip**, once `needs-decision/loudness-in-the-index.md` has an answer. The measurement it stores
   is built and tested.
3. **`#14` per-asset licence and source metadata** and **`#16` incremental indexing by content hash** — the two
   remaining p0s that are neither blocked nor need a decision. `library.rs`'s `fnv1a` is already #16's seed.
4. **`#22` spectral centroid, rolloff and noise floor**, unblocked by ADR 0003 and cheapest of the three hand-rolled
   features because they share one FFT pass.
5. **`#28` the MCP tool host**, then `#29`/`#44`. Still the largest user-facing win, and still consuming an index whose
   feature layer is only now resolved.

## Iteration 76 — 2026-11-03 — the audio tower, verified to cosine 0.999999995

**Baseline.** Green on every step once disk was reclaimed: 198 / 207 serde / 208 mixer / 6 doc / 12 semantic, clippy 0,
fmt, rustdoc, examples, readme. Working tree was **dirty** at `b58ec1a` with iteration 75's first item half-done — code
edits in `mel.rs` and `model.rs` plus one evidence file — so finishing it *was* this iteration. No cue case red.

**The pass iteration 75 asked for found a defect, not a confirmation.** `runs/2026-11-03-76/evidence/audio-tower.txt`
ended, in the half-done state, with *"the embedding agrees with the reference at cosine 0.850 … a 0.850 cosine is not a
working feature."* Two constants were wrong, both read from `transformers/audio_utils.py`:

- **The right-hand reflect pad is `WINDOW/2`.** `np.pad` pads `(512, 512)` and the **left** pad shifts the last frame,
  so the right side needs 512 samples — not the distance to where the last frame's window ends. The tree reflected 1024;
  an earlier attempt read past the buffer at all, which in release builds is not a panic but whatever the allocator held,
  measured as the last frames of a 1001-frame spectrogram collapsing to −100 dB.
- **The FFT accumulates in `f64`**, because `spectrogram` promotes with `waveform.astype(np.float64)` before `rfft`.
  `f32` left the worst spectral value **11 dB** out at the noise floor.

**A wrong oracle nearly hid it.** The first measurements compared against a **default-constructed** `ClapFeatureExtractor`,
whose `mel_filters` differ from the pretrained checkpoint's — 582 non-zeros against 580 — and which made even the
*reference's* row sums look non-uniform. Measuring the pretrained model instead moved the crate from "cosine 0.850,
cause unknown" to a **0.0013 dB mean** and the two constants fell out. **The fixture was right and the oracle was wrong**,
which is the failure mode to remember: check the reference before trusting a disagreement.

| Number | Before | After | Reference |
|---|---|---|---|
| embedding cosine | 0.849973469 | **0.999999995** | 1.0 |
| mel, max abs diff over 1001 × 64 | 11.004 dB | **0.0545 dB** | — |
| mel, frame 0 band 0 | — | **2.4 × 10⁻⁵ dB** | 16.570840836 |
| mel, frame 1000 band 0 | −100 dB (floor) | **2.6 × 10⁻⁵ dB** | 16.498333931 |
| frames at the −100 dB floor | whole tail collapsed (0.850 cosine) | **0** | **0** |
| graph alone, on the reference's mel | — | **1.000000007** | — |

The last row separates the spectrogram from the ONNX path: the graph was **already exact**, so every remaining digit was
mine.

**Tests that would have caught it.** The old suite checked `[4][1001][64]` and frame 0 of a **101-frame** buffer — a
shape and the one frame a short pad cannot break. Two tests now name **frames 0, 500 and 1000** of the model's
480 000-sample window against rows measured from the pretrained extractor (tolerance 0.06 dB, measured max 0.0545), and
assert the count of all-floor frames is **0**. Frame 1000 is the assertion that matters: a short pad turns it from 16 dB
into −100 dB, a 116 dB error at one index.

Also fixed: **`is_longer` is a `bool` tensor** (the graph rejected `int64`), and **all twelve semantic error types
implement `std::error::Error`** — a `?` in a `Box<dyn Error>` caller could not use any of them.

**A defect clippy found that the compiler did not:** the reference literals carried ten significant digits for `f32`
values (`clippy::excessive_precision`). Truncated; the measurements are unchanged.

**The disk is part of this entry.** The volume hit **100 %, 122 MB free** and six baseline steps failed with
`ld: write() failed, errno=28` — the trap the skill names, not a code error. Deleted, with permission: the two prior
iterations' `/tmp` probe directories (2.9 GB) and `cargo clean -p` on the heaviest dependency artifacts. Every step then
passed. **A full `target/` was never deleted.**

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**. Pass 3 found the guide page telling readers the
feature was broken at cosine 0.850, which the fix had just made false in the other direction; its status line now leads
with the measured cosine and says plainly that the *downstream* recall is still design defaults.

**Gate.** 198 default / 208 mixer / 207 serde / 6 doc / 12 semantic; clippy 0, fmt, rustdoc, examples, readme — green.

**Evidence.** `runs/2026-11-03-76/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes),
`evidence/audio-tower.txt` with a measured resolution appended.

**Commits.** `b33fa89`.

**Top 5 remaining**

1. **`#28` the MCP tool host**, then `#29` `search_music` and `#44` `explain_match`. The index, the spectrogram, the
   embedding path and the documented API are all now verified; nothing downstream is blocked on the audio tower.
2. **`V-01` the labelled set** — needs a person. It turns the floor, the band constants and the six weights from design
   defaults into measurements, and it is now the largest unmeasured thing in the layer.
3. **Where the weights live** — `needs-decision/weights-live-where.md` recommends a fourth ungated section of the project
   file plus a companion commit in `bevy_grimoire`; actionable and non-breaking, so it need not wait.
4. **`cargo run --example semantic`** — the artifact a reader believes without building it themselves. Iteration 76
   measured the same path from a probe and left the example, which was carved off deliberately.
5. **Free disk before the next release build.** 122 MB free at the start of this iteration cost six baseline steps and
   an unplanned round of `cargo clean -p`. A release build wants ~2 GB.

## Iteration 75 — 2026-11-03 — the last closure, and the page that makes it findable

**Baseline.** Green on every step: 198 / 367 + 6, clippy, fmt, rustdoc. Working tree clean at `bf52554`. No case red.

**Two items that are one.** Iteration 74 left two things at the top of its list: wire the mel path into the indexer, and
document the semantic layer. They are the same item seen twice — **the code makes the feature runnable, the page makes it
findable**, and iteration 74 measured the layer at **zero** README mentions. This repository's own rule is that a
feature is not done until its README line exists.

**`build_from_model`** fills `indexer::build`'s seam: decode → resample → **mel** → **embed** → insert, with the model
doing the embedding and no caller-supplied closure. The one decision is **`is_longer`**, the audio tower's second input:
the extractor sets it per clip from whether the audio exceeded the window, and here it is read from the **buffer's
length** and **reported** on `Built::longer` rather than assumed — it selects a different branch of the graph, and a
caller comparing two indexes needs to know whether one of them took the fusion branch.

**`docs/guide/semantic.md`** leads with its status, which is the part worth quoting:

> *"behind the `semantic` feature, and not yet end-to-end … **the audio tower has never been run against the real model
> in this repository.**"*

Then it says which numbers make the rest trustworthy — the spectrogram exact to **4 × 10⁻⁶** against CLAP's own
extractor, the tokeniser exact against the model's, and the floor's recall table (**100 %** at 0.0, **75 %** at 0.150,
**0 %** at 0.5). A developer learns what works, what does not, and how they would know, **in that order, before the
API**. The README gains one bullet and the guide index one line: **0 → 2 mentions, 5 → 6 pages**.

**A defect worth naming.** A blanket text replace removed `mut` from `embed_text`'s closure while adding the new
function, where it *is* required. The compiler refused — `cannot borrow run as mutable` — and the fix was to restore it
there and remove it only from `embed_audio`, whose closure clones rather than moves. **A global edit that compiles in
one place can be wrong in another.**

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** **198 default / 368 all-features**, clippy 0, fmt, rustdoc — green.

**The visual gate is unchanged**, which is the check that neither the mel path nor the writing touched the audio: 32
render files, **14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS** — identical to iterations 67, 71 and 74. The bang
spectrogram still shows dense harmonics across 2–7 kHz decaying by 0.21 s, visibly not the footstep's three partials,
with no click line, no clipping shelf and no gap.

**Evidence.** `runs/2026-11-03-75/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes),
`evidence/after/` (32 files), `evidence/spectrograms/bang.png`.

**Commits.** `5d14866`.

**Top 5 remaining**

1. **Run the audio tower against the real graph** on `bmb`. It is now the *only* unverified link in the chain, and the
   guide page says so in its first paragraph. This is a verification pass, not a build.
2. **Where the weights live** — `needs-decision/weights-live-where.md` recommends a fourth ungated section of the
   project file plus a companion commit in `bevy_grimoire`. Its recommendation is actionable and non-breaking, so by
   iteration 72's own lesson it need not wait for an answer.
3. **`#28`** the MCP tool host, then `#29` `search_music` and `#44` `explain_match`, which now have an index, a
   spectrogram, an embedding path and a documented API to call.
4. **`V-01`** — the labelled set, which needs a person, and would turn the floor, the band constants and the six
   weights from design defaults into measurements.
5. **An example** — the crate has `mixer`, `music`, `budget`, `perfcheck` and `render`, and nothing that shows the
   semantic path. `cargo run --example semantic` is the artifact a reader believes without building it themselves.

## Iteration 74 — 2026-11-03 — CLAP's spectrogram, in Rust, exactly

**Baseline.** Green on every step: 198 / 367 + 6, clippy, fmt, rustdoc. Working tree clean at `7ad8c54`. No case in
`cues.yaml` red.

**The item.** Iteration 73's first item, and *"the largest blocker that is not a decision"*: the log-mel spectrogram
`ClapFeatureExtractor` produces, implemented in the crate. It is the mel path `indexer::build`'s seam has been waiting
for — after this, a caller can be given a real embedder.

**Every constant read from the model, not remembered.** 48 000 Hz; window **1024**, Hann; hop **480**; **64** mel bands;
**50 – 14 000 Hz**; filterbank **`(64, 513)`**; `1e-10` floor (−100 dB). The pipeline was read from
`transformers.audio_utils.spectrogram` line by line: **reflect-pad by `frame/2`**, Hann, `rfft`, `|z|²`, mel matmul
floored at `1e-10`, `10·log10`.

**It is the same spectrogram, not one that looks right.**

| band | this crate | `ClapFeatureExtractor` | Δ |
|---|---|---|---|
| 0 | 16.570843 | 16.570843 | **0** |
| 1 | 16.544992 | 16.544992 | **0** |
| 2 | 16.996017 | 16.996021 | 4 × 10⁻⁶ |
| 3 | 17.427809 | 17.427816 | 7 × 10⁻⁶ |
| 5 | 18.320272 | 18.320274 | 2 × 10⁻⁶ |

Five of six agree to **six decimal places**; the differences are `f32` rounding. The fixture is the model's own 32,832
weights, checked in, so the comparison cannot drift and needs no Python to re-run.

**The defect worth remembering, and it is the ninth of its kind here.** `mel_filters` is **`(64, 513)`, band-major**. My
first version assumed `(513, 64)` and applied the transpose — which produces a spectrogram of the **right shape and the
wrong content**: 101 frames of 64 bands either way, no error, no crash, and every downstream comparison quietly wrong.
**Only the numbers caught it; the shape could not.**

**One deliberate difference, named.** `ClapFeatureExtractor`'s fusion path picks its three crop windows **at random**,
which makes an embedding depend on a dice roll — the one thing this crate's determinism rule forbids. `fusion_channels`
uses fixed front/middle/back instead. For a clip that fits one window all four channels describe the same audio, so the
difference shows only on a long track, where a deterministic choice is also the reproducible one.

**`Model::embed_audio` is written but not exercised against a real graph** — the 116 MB audio tower lives on `bmb` and a
runtime check needs `ort` there. The spectrogram, which is the hard part and the part with numbers, is exact.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** **198 default / 367 all-features** plus **10 mel unit tests**, clippy 0, fmt, rustdoc — green.

**The visual gate is unchanged**, which is the check that the audio path was not disturbed: 32 render files, **14
distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS**, the duck recovering by **1.97 s** — identical to iteration 67. The
footstep spectrogram still shows three partials at ~90/220/340 Hz decaying by 0.21 s, with no click line, no clipping
shelf and no gap.

**Evidence.** `runs/2026-11-03-74/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes),
`evidence/after/` (32 files), `evidence/spectrograms/`.

**Commits.** `7bea69b`.

**Top 5 remaining**

1. **Wire the mel path into `indexer::build`** — it is now one closure: `mel::input_features` then
   `Model::embed_audio`. The indexer has the seam and the mel module has the data; joining them is small and makes the
   feature runnable end to end.
2. **Documentation** — the semantic layer still has **zero** README mentions and no guide page. This is now the cheapest
   large gap, and it is what makes the work discoverable.
3. **Exercise the audio tower against the real graph** on `bmb`, which is the one part of this chain not yet run.
4. **Where the weights live** — `needs-decision/weights-live-where.md` recommends a fourth ungated section plus a
   companion commit in `bevy_grimoire`; actionable and non-breaking.
5. **`#28`** the MCP tool host, then `#29` `search_music` and `#44` `explain_match`, which now have an index, a
   spectrogram and an embedding path to call.

## Iteration 73 — 2026-11-03 — the indexer: every part existed and nothing composed them

**Baseline.** Green on every step: 198 / 356 + 6, clippy, fmt, rustdoc. Working tree clean at `6f86607`. **No case in
`cues.yaml` is red** — case 31 went green in iteration 72 and stayed green.

**An audit before picking, and it found three gaps worse than the item I expected.**

| Gap | Evidence |
|---|---|
| **nothing builds an index from a folder** | `grep "scan\|build_index\|embed_folder"` over `src/semantic/*.rs` → nothing |
| **no audio embedding** — `Model` exposes `embed_text` only | `grep "pub fn embed"` → one hit |
| **zero documentation** — no README mention, no guide page | `grep -ci semantic README.md` → **0** |

The first made every other item on the list premature: `Index::new` plus `Index::insert` is not an indexer, and a caller
who wanted an index of their library had to write the loop — and get the content hash, the skip list, the failure policy
and the stamp right on their own.

**What shipped.** `build(root, stamp, embed)`: per file, decode (rate from the stream, not the header) → resample (tail
flushed, so 1.000 s stays 1.000 s) → embed → insert under the content hash, so a rename does not change identity. Plus
`write`/`read` beside the library, and `stale`/`vanished` — the two halves of an incremental rebuild, where **a rename is
neither**.

**The embedding is a `FnMut`, not a trait.** Audio embedding needs a mel spectrogram, which needs a filterbank — its own
piece of work with its own numbers. A seam keeps the orchestration complete and testable today and lets the mel path drop
into one place later; the module doc says plainly that it is not there yet rather than implying it is.

**A defect the test caught, and it is the same fault this crate forbids one layer down.** `library::scan` reports three
kinds of skip — `NotAudio`, `UndecodableHeader`, `Unreadable` — and the first indexer filed only the first into
`not_audio` and **ignored the other two**. A WAV with a corrupt header vanished from both lists.
`a_corrupt_file_is_counted_not_swallowed` asserted `failed.len() == 1` and got **0**. The fix splits the skips by what
they mean to a caller: *not audio* is a fact about the folder, *would not read* is a failure, and both now appear.

**A second defect was the test's.** `a_rebuild_is_what_an_incremental_index_asks_about` asserted that a rename changes
nothing, using a file that had never been indexed. It failed, correctly. **A test asserting an absence can be satisfied
by never doing the thing**, which is worth remembering.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT** on the orchestration — with the honest note that
**the feature as a whole is not yet usable**, because there is no audio embedding and no documentation.

**Gate.** **198 default / 367 all-features**, clippy 0, fmt, rustdoc — green.

**A disk failure, the sixth.** The all-features link failed at **323 MB** with `cc: error: linker command failed` — the
first time in this project it has presented as a *linker* error rather than a `rustc` I/O error. Same cause, same fix.

**Evidence.** `runs/2026-11-03-73/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes).

**Commits.** `118bbee`.

**Top 5 remaining**

1. **Audio embedding** — a mel spectrogram and a filterbank, which is the one thing between the indexer and a usable
   feature. It needs no model choice and no licence audit; it is the largest blocker that is **not** a decision.
2. **Documentation** — the semantic layer has zero README mentions and no guide page. A feature nobody can find is not
   shipped, and this is the cheapest item on the list.
3. **Where the weights live** — `needs-decision/weights-live-where.md` still recommends a fourth ungated section plus a
   companion commit in `bevy_grimoire`; actionable and non-breaking, so by iteration 72's own lesson it need not wait.
4. **`#28`** — the MCP tool host, then `#29` `search_music` and `#44` `explain_match`, all of which now have an index to
   answer against.
5. **`V-01`** — the labelled set, which needs a person, and would turn the floor, the band constants and the six weights
   from design defaults into measurements.

## Iteration 72 — 2026-11-03 — case 31: the decision nobody implemented

**Baseline.** Every crate test green (194 / 352 + 6, clippy, fmt, rustdoc) — but **`cues.yaml` case 31 was red**, and had
been for about thirty iterations.

**The case.** Every number a cue author writes that the crate cannot honour should be named. `pitch` and `attenuation`
are, through `problems()`. **`gain_db`, on the cue and on the slot, was silent**: a `NaN` reads as no gain, so the cue
sounded at the level it authored and nobody ever learned why the number they wrote had no effect.

**Why it stayed red, and it is the loop's failure rather than a blocker.** The case's own `blocked_on` named
`needs-decision/bad-gain-problem.md`, and **that file has recommended option C — a sentence on `SirenLog`, no API
change — since iteration 18.** C needs no owner's answer. Thirty iterations read the file and moved on, treating a
recommendation as a stop sign.

> **A decision file is not always a stop sign. When its recommendation is actionable and non-breaking, the
> recommendation is the instruction.**

**The change, and it adds no variant.** `Entry` is a public enum without `#[non_exhaustive]`, so a new variant breaks
every consumer's exhaustive `match`. But iteration 16 had already established the shape: `Entry::Stopped` carries a
free-text `why: String`. So `Started` now carries `neutralised: Vec<&'static str>` — in the shape of its existing
`stolen` — and the log says:

> `played, but cue gain_db and sound gain_db were not a number and each was read as the neutral value`

The field names are the ones a project's file uses, so the string can be searched for. **The behaviour is unchanged**: the
cue still plays, because a lost sound is worse than a mis-typed number — the conclusion the decision file already
reached.

**Four tests**, and the one that matters most is the one that must *not* fire: `a_finite_gain_says_nothing_at_all`. A
sentence on every play would be noise a person learns to skip.

**A defect worth remembering.** The first version of the test asserted on the log without ever sending a `Play` or
loading a sound. It failed — because the assertion was `assert_eq!(said.len(), 1)`. A weaker assertion,
`assert!(said.iter().all(…))`, **would have been green on an empty log and proved nothing**. A test that reads a log
must trigger it.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** **198 default / 356 all-features**, clippy 0, fmt, rustdoc — green.

**`cues.yaml` case 31 is now green**, and `docs/guide/cues.md` describes what happens rather than what is missing. The
`needs-decision/` file is **not** marked decided: option A — the `CueProblem` variant — is still the tidier home and
still waits for a minor release.

**Evidence.** `runs/2026-11-03-72/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes), and
`tests/neutralised.rs` as the case's evidence.

**Commits.** below.

**Top 5 remaining**

1. **Where the weights live** — `needs-decision/weights-live-where.md` recommends a fourth ungated section plus a
   companion commit in `bevy_grimoire`. Its recommendation is **actionable and non-breaking**, which by this iteration's
   own lesson means it does not need to wait.
2. **`#39`'s encoder** — genuinely blocked on `S-05`'s licence audit, which needs an owner's answer because it decides
   what may ship.
3. **`V-01`** — the labelled set, which needs a person.
4. **`#28`** — the MCP tool host, then `#29` `search_music` and `#44` `explain_match`, both now possible.
5. **Option A for `CueProblem`** — the variant, at the next minor, which would move the sentence from the log back into
   `problems()` where the other five live.

## Iteration 71 — 2026-11-03 — the ensemble's breakdown: a score that explains itself

**Baseline.** Green on every step: 181 / 339 + 6, clippy, fmt, rustdoc. Working tree clean at `c842b6b`.

**The item.** Three of iteration 70's five remaining items are **decisions this loop must not make**: where the weights
live (a schema change), `V-01`'s labelled set (a person and a licence-cleared corpus), and `#40`'s full ensemble
(`#39`'s encoder, behind `S-05`'s licence audit). What *is* decidable is **`#40`'s structure** — the types a score is
carried in and the arithmetic that makes a breakdown add up — which needs no signal at all.

Building it first means `#40` has somewhere to put its numbers instead of inventing them at a call site.

**Where the arithmetic went, and the rule that enforces it.** `#40`'s note says to put the scoring where `src/model.rs`
keeps its arithmetic. That is a rule and not a preference: `tests/all.rs::model_names_no_engine_type` scans **exactly one
file**. A score defined in `semantic/` would sit outside the scan, could quietly grow a `Res<…>` parameter, and then
nothing would test it without an `App`. So the types went into `model.rs` — and
`the_ensemble_lives_in_the_engine_free_model` now asserts they are **there**, so moving them out fails a test instead of
silently escaping the scan.

**Four facts per signal, not one.** Each `Contribution` carries its **raw** value, its **weight**, the **product**, and a
**`missing`** flag. The flag is the subtle one: a tempo that was never estimated and a tempo that was estimated and
disagreed both contribute `0.0`, and conflating them misleads precisely the person the breakdown exists for. A raw cosine
in `[−1, 1]` and a loudness penalty in decibels look identical once multiplied.

**The score is a sum, not a mean.** The weights are the normalisation — a project that halves a weight is saying that
signal matters half as much, and dividing by the total would undo the statement. `shares()` is the normalised view, for
reporting.

**A defect in my own first draft, caught by the compiler.** `strongest()` recovered which signal a contribution belonged
to with `std::ptr::eq` — a pointer comparison standing in for a field, working only by accident of array layout. Giving
`Contribution` a `signal` field fixed it and let `Breakdown::new` place parts by their own name, so the array order
cannot be got wrong.

**The issue's own caveat, carved off and named.** `#40` warns that raw values from different signals are on different
scales and must be mapped before weighting. This iteration does **not** solve that — the mapping is a property of the
signals, and there are no signals yet. What it does is make the problem **visible**: a `Contribution` reports `raw`
beside `weight`, so a caller can see the scales differ. A breakdown showing only products would hide exactly that.

**Verdicts.** Pass 1 **GREAT** on what it claims (one `#40` criterion needs signals and is named as carved off), pass 2
**GREAT**, pass 3 **GREAT**.

**Gate.** **194 default / 352 all-features**, clippy 0, fmt, rustdoc — green.

**A breakdown reads as a sentence**, which is the whole of `#44`'s `explain_match` in a string:

> *"semantic carried it (+0.400 from 0.4 raw at weight 1); acoustic counted against it (-0.100 from -0.2 raw); no value
> for tempo, key"*

**Evidence.** `runs/2026-11-03-71/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes).

**Commits.** the ensemble commit below.

**Top 5 remaining**

1. **Where the weights live** — `needs-decision/weights-live-where.md` still waits; until it is answered `Weights` is
   unreachable from a project, and `#40`'s ensemble has nowhere to be configured from.
2. **`#39`'s encoder** — behind `S-05`'s licence audit, and the gate on the ensemble having real signals.
3. **`V-01`** — the labelled set, still the largest single lever: it would turn `Floor`, the band constants and the six
   weights from design defaults into measurements.
4. **`#28`** — the MCP tool host, then `#29` `search_music` and `#44` `explain_match`, both of which the breakdown now
   makes possible.
5. `cues.yaml` case 31 — still the only red case in the crate. **One public-enum decision unblocks it and four other
   `needs-decision/` files at once**, which remains the cheapest large win.

## Iteration 70 — 2026-11-03 — ensemble weights as data, with a report of their own

**Baseline.** Green on every step: 181 / 324 + 6, clippy, fmt, rustdoc. Working tree clean at `2af5529`.

**The item.** `#41` is the first item on iteration 69's remaining list that is **decidable today**: `V-01` needs a person,
`#45`'s runner needs `V-01`'s data, and `#28` would expose a pipeline whose thresholds are still unmeasured. Building the
weights first means `#40` has somewhere to put its numbers rather than inventing them at a call site.

**A dependency the issue states that the code does not.** `#41` says *blocked by #40*, but `#40` needs `#39`'s
music-specific encoder, which is blocked on `S-05`'s licence audit — a decision. The weights type is what `#40` would
**consume**, so building it first inverts nothing and unblocks the half that can be judged.

**Six fields, each a design default with its argument in the source.** Two rest on this crate's own measurements rather
than taste:

| Field | Default | Why |
|---|---|---|
| `acoustic` | **0.5** | the semantic bands **overlap** (0.126–0.429 vs −0.104–0.150, measured), so production differences are what break a near-tie |
| `drop_fraction` | **0.0** | iteration 69 measured recall@1 at **100 %** for 0.126, **75 %** for 0.150 and **0 %** for 0.5 |

The other four — `semantic` 1.0, `tempo` 0.35, `key` 0.35, `loudness_tolerance_db` 6.0 — are argued from the crate's own
behaviour: a tempo is absent for every effect so it earns less weight; nothing measures tempo against key so ranking
them would be a magic constant wearing a name; and 6 dB sits below the crate's own 8 dB duck.

**The design decision is the line between *impossible* and *unusual*.**

**Refused:** all four weights zero (nothing ranks), a negative weight (a signal meant to attract now repels), a NaN
(compares `false` against everything), a zero tolerance (every pair penalised), a drop fraction outside `0..1`.

**Allowed:** a semantic-only ranking, a **40 dB** tolerance for a stylised palette whose loudness jumps are intended —
the issue's own example — and a tight tolerance with no dropping.
`an_unusual_but_possible_configuration_is_allowed_through` is the more important of the two tests: **a report that
refuses a legitimate choice is a report people learn to ignore.**

**A criterion not met, and it is named rather than dropped.** The issue asks that a default configuration produce a
non-empty result on the fixture library. That needs something to produce a result — `#40`'s ensemble — which needs an
encoder behind a licence decision. Carved off and stated in `plan.md`.

**Where a project writes them is a schema change**, so it went to
[`needs-decision/weights-live-where.md`](needs-decision/weights-live-where.md) rather than being decided here. Its
recommendation is a **fourth section of the project file, ungated**, because the other two options reintroduce the defect
iteration 61 shipped a consumer fix for: **a reader with fewer fields than the writer silently deletes data.**

**Verdicts.** Pass 1 **GREAT** on what it claims (the unmet criterion is named), pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 181 default / 190 serde / 191 mixer / **339 all-features**, clippy 0, fmt, rustdoc, examples — green.

**A disk failure, recorded for the fourth time.** Three suites failed with errors that read as code defects —
`rustc-LLVM ERROR: IO failure on output stream` and `failed to write lib.rmeta` — at **130 MB free**. A clean freed
18.6 GiB and every count above was then taken from a complete build. **The machine fact, worth writing down: a debug
build of this crate plus the full `bevy` dev-dependency is ~19 GB, and running more than two feature combinations in one
iteration needs a clean between them.**

**Evidence.** `runs/2026-11-03-70/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes).

**Commits.** `c1a0f53`.

**Top 5 remaining**

1. **Where the weights live** — `needs-decision/weights-live-where.md` recommends a fourth ungated section of the
   project file plus a companion commit in `bevy_grimoire`. Until then `Weights` is unreachable from a project.
2. **`#40`** — the ensemble with a per-result breakdown, which now has weights to consume. Still needs `#39`'s encoder,
   which needs the licence audit.
3. **`V-01`** — the labelled set, still the largest single lever: it turns `Floor`, the band constants **and** these six
   weights from design defaults into measurements.
4. **`#28`** — the MCP tool host, then `#29` `search_music`.
5. `cues.yaml` case 31 — still the only red case in the crate. **One public-enum decision unblocks it and four other
   `needs-decision/` files at once**, which remains the cheapest large win.

## Iteration 69 — 2026-11-03 — the evaluation metric: what a floor does to recall

**Baseline.** Green on every step: 181 / 309 + 6, clippy, fmt, rustdoc, examples. Working tree clean at `593e6b8`.

**The item.** Every threshold on the semantic path is a **design default** because there is no labelled set: `Floor`
at **0.0**, `MEASURED_MATCH_FLOOR` at 0.126, `MEASURED_MISS_CEILING` at 0.150. `V-01` produces the data and `#45` runs
against it, but **neither can exist usefully without a metric** — a labelled set with no metric is a data file. The
metric is decidable now while the data is not, so it was built first.

**What shipped.** `src/semantic/eval.rs`: `Labelled` (a description and its right answer), `Judged` (an anchor and the
neighbours a person **affirmed** fit it), `Ranking`, `recall_at_k`, `agreement`, `score` → a `Report` that **names its
failures**, and `at_floor`.

**Two decisions in it worth stating.**

- **Recall, not precision**, because a wrong result in a list of three costs a glance and a missing one costs a
  developer their afternoon — the same asymmetry that makes this crate refuse with a reason rather than play something
  wrong.
- **Agreement is positive-only.** A pair a person was shown and rejected is **not** evidence that it is wrong: given
  twenty candidates a person rejects most of them for reasons the ranking cannot see. Asserting only what was affirmed
  makes the metric weaker and honest.

**`at_floor` is `V-01`'s whole purpose in one function**, and its first real use is the measurement below.

**Measured on iteration 68's real cosines** (real CLAP, four clips, four descriptions):

| floor | recall@1 | note |
|---|---|---|
| **0.000** | **100 %** | the default |
| 0.126 | **100 %** | `MEASURED_MATCH_FLOOR` |
| 0.150 | **75 %** | `MEASURED_MISS_CEILING` |
| **0.500** | **0 %** | the number intuition reaches for |

That table is a test, not a comment: `a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model`. It
is the argument for `Floor::default() == 0.0` in four rows.

**A report reads as a sentence, not a scoreboard.** *"recall@k 50% over 2 descriptions, agreement 100% over 0 anchors.
Missed: `white noise hiss` (rank 2)."* A count alone tells a person nothing they can act on.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 181 default / **324 all-features**, clippy 0, fmt, rustdoc, examples — green.

**A destructive command, recorded.** Freeing disk I ran `git checkout -- .`, which **reverted the two tracked edits this
iteration had made** (`src/semantic/mod.rs`, `tests/all.rs`); the new files survived because they were untracked. Both
were restored and verified with `grep`. The narrower `git checkout -- examples/evalprobe.rs` is what was wanted — the
repository's rule covers `git restore` over my own changes as much as over anyone else's.

**Also recorded:** a `--release --example` build failed at **495 MB free** with
`rustc-LLVM ERROR: IO failure on output stream: No space left on device`. The probe was moved into the test suite, where
the same measurement costs nothing. Third disk failure in this project.

**Evidence.** `runs/2026-11-03-69/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes).

**Commits.** the eval commit below.

**Top 5 remaining**

1. **`V-01`** — the labelled assets and human judgements. It is now the single largest lever left, and the metric that
   consumes it exists.
2. **`#45`'s runner** — the small harness that feeds real rankings into `score`. It needs `V-01`'s data to be worth
   writing; the metric half is done.
3. **`#41`** — tunable weights as a serde type, which `at_floor` can now evaluate rather than assume.
4. **`#28`** — the MCP tool host, then `#29` `search_music`, exposing `Embedder::ask` to an agent.
5. `cues.yaml` case 31 — still the only red case in the crate. **One public-enum decision unblocks it and four other
   `needs-decision/` files at once**, which remains the cheapest large win.

## Iteration 68 — 2026-11-03 — the description-to-answer seam, in one call

**Baseline.** Green on every step: 181 / 300 + 6, clippy, fmt, rustdoc, examples. Working tree clean at `c74c84d`.
Case 31 remained the only red case, blocked on the `CueProblem` enum decision.

**The item.** Iteration 67's item 4: the four semantic pieces were tested and **nothing joined them**. `Query`
validated a description, `Vocabulary` tokened it, `Model` embedded it, `search` ranked an index against the vector. A
caller had to wire them in the right order and flatten three error types — and **four of the five orders are wrong**:

1. embed before validating, so `"   "` costs a GPU call to fail;
2. rank before the stamp check, so vectors from two models compare silently;
3. tokenise after embedding, which the compiler prevents;
4. check the stamp after the embed, which is a correct answer with a wasted call and a message blaming the ranking.

`Embedder::ask` fixes the order — **validate, stamp, encode, embed, rank** — and reports one `AskProblem` whose variants
are distinct things to do.

**Proved on the real model.** Four descriptions, through the crate's own tokeniser, against the 501 MB CLAP text tower
and four embedded clips: **4 / 4**. Two of those are honest near-ties (0.429 vs 0.423 between two clips both generated
from white noise), which is the model saying two things are nearly the same rather than a failure of the seam.

**The finding that changes a default's justification.** At the **0.126** floor the correct band bottoms out at, the
number dropped is **2, 2, 1 and 3** out of 3 — on the last query the floor deletes **every** result. So `Floor::default()`
staying at **0.0** is not timidity; it is the only defensible value while there is no labelled set. Iteration 63
measured that the correct and incorrect bands **overlap**; this iteration measured what that costs in practice.

**A fixture that had been modelling the wrong contract since it was written.**
`tests/fixtures/tiny-text.onnx` declared a `MatMul` of `[3, 512]`, so it accepted **exactly three tokens** — while the
real graph declares `[-1, -1]`. Nothing caught it because until this iteration nothing passed it a phrase longer than
three tokens. Rebuilt to average over the token axis so it takes any length. The cost is recorded rather than hidden: it
can no longer distinguish phrases, so **the contract is proved on the fixture and the ranking on `bmb`**, and
`the_answer_reports_what_the_floor_dropped` asserts the floor's accounting rather than a ranking the fixture cannot
produce.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 181 default / **309 all-features**, clippy 0, fmt, rustdoc, examples — green.

**Evidence.** `runs/2026-11-03-68/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes),
`evidence/ask-e2e.txt`, `evidence/ask_e2e.json`.

**Commits.** `9597faf`.

**Top 5 remaining**

1. **`#41`** — tunable weights as a serde type. Now it has a consumer: `Embedder::ask` takes a `Floor`, and weights are
   the next thing a caller will want to vary.
2. **`#40`** — the weighted ensemble with a per-result breakdown, which `#41`'s weights feed.
3. **`#28`** — the MCP tool host, then `#29` `search_music`, which is `Embedder::ask` exposed to an agent.
4. **`V-01`** — the labelled set. It is now the single largest lever on quality: it is what turns `Floor` and the
   ensemble weights from design defaults into measured thresholds.
5. `cues.yaml` case 31 — still the only red case in the crate. **One public-enum decision would unblock it and four
   other `needs-decision/` files at once**, which makes it the cheapest large win left.

## Iteration 67 — 2026-11-03 — **the evidence harness was measuring one waveform**

**Baseline.** The crate was green on every step (174 / 183 / 184 / 293 + 6, clippy, fmt, rustdoc, examples). The red was
in the *harness*, found by looking at the evidence rather than at the code:

    md5 cue-footstep-grass-0.wav    eefe987d5aef348c1d6f4abe482db585
    md5 cue-dialogue-greeting-0.wav eefe987d5aef348c1d6f4abe482db585
    md5 cue-bang-0.wav              eefe987d5aef348c1d6f4abe482db585

**Three different cues, one file.** `examples/render.rs` synthesised every cue and variant as
`wave(440.0 * ratio, ms, 12.0)` — **one 440 Hz sine** — and `write_gain` began `let _ = speed;`, discarding the playback
rate its own doc claimed to apply.

**The consequence is not cosmetic.** Every spectrogram this project has judged, across three passes of review for many
iterations, was a picture of the same sine wave. The visual gate reported 30 cues and measured one.

**The fix.** A timbre comes from four numbers derived by FNV-1a over the sound's own name — a fundamental (90–620 Hz),
two partial ratios, and a decay — with a loop slow (0.6–1.6 /s) and an effect fast (6–24 /s). `at_speed` applies the
playback rate by interpolation, so a cue detuned a semitone is 1.0594x longer **and** every frequency drops by 0.9439.

| | before | after |
|---|---|---|
| distinct cue files | **3 cues shared one hash** | **14 files, 14 distinct hashes** |
| `voices-8` peak | 3.419 | **2.959** |
| `voices-8` samples past full scale | 307 | **162** |
| `mix.wav` integrated | −18.6 LUFS | **−21.5 LUFS** |
| `mix.wav` true peak | −6.1 dBFS | **−7.5 dBFS** |

**The spectrograms now differ, and the difference is the point.** A footstep shows three partials at ~90/220/340 Hz
with a broadband attack decaying by 0.21 s; a bang shows more harmonics and a fuller 2–7 kHz band; a music loop shows
harmonics continuous across 1.55 s with no transient. No click line, no clipping shelf, no gap in any of them.

**The shapes moved to `examples/render/shapes.rs`** so `tests/render_shapes.rs` includes them by `#[path]` and asserts
against the *same functions the harness calls* — seven tests in **0.01 s** rather than nine minutes per release build.
They stay beside the harness rather than in `src/`: a stand-in for a recording is evidence tooling, and a game should not
be able to call it.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 181 default / 300 all-features, clippy 0, fmt, rustdoc, examples — green.

**A mistake, recorded rather than quietly fixed.** Freeing disk I ran `rm -rf` over
`docs/loop/runs/2026-11-03-6{2,3,4,5}/evidence/after` — **committed evidence from four earlier iterations**. Restored
immediately with `git checkout -- docs/loop/runs/`; `git status` then showed only `examples/render.rs` modified. The
repository's rule is explicit: deleting files this session did not create needs permission.

**Evidence.** `runs/2026-11-03-67/`: `research.md`, `plan.md`, `eval.md`, `review.md` (three passes), `evidence/after/`
(32 files) and `evidence/spectrograms/` (four cues).

**Commits.** `a9f0ffd`.

**Top 5 remaining**

1. **`#41`** — tunable weights as a serde type, which is where the rank floor stops being a design default.
2. **`#40`** — the weighted ensemble with a per-result breakdown, which `#41`'s weights feed.
3. **`#28`** — the MCP tool host, then `#29` `search_music`, which makes any of this reachable by an agent.
4. **The description → ids → model → search integration test** — `Query::tokenize` works, but nothing runs all four in
   one call yet. That is a test, not a feature.
5. `cues.yaml` case 31 — still the only red case in the crate, still blocked on the same public-enum decision as four
   other `needs-decision/` files.

## Iteration 66 — 2026-11-03 — the tokeniser: the last seam on the semantic pipeline

**Baseline.** Green on every step; working tree clean at `5eedd98`. Case 31 remained the only red case, blocked on the
`CueProblem` enum decision.

**The item.** With the original critical path closed in iteration 65, this opened the next one: the seam between the
crate and its consumers. The first item is the tokeniser, because every text query needs it and nothing downstream —
the tool surface, the panel, the labelled set — can be built until it exists.

**CLAP's tokeniser, measured from the model.** A `RobertaTokenizer`: **byte-level BPE**, vocabulary **50,265**, merges
**50,000**, pre-tokeniser `ByteLevel` with GPT-2's pattern, post-processor `RobertaProcessing` (`<s>`=0 … `</s>`=2).
`transformers`' `tokenizer.json` is **3,558,802 bytes** and carries the whole rule set, so the implementation is the
algorithm plus a data file — **not a dependency**. The crate's list stays at five.

**Proven by equality, not plausibility.** Ten strings pinned against `transformers`' own `ClapProcessor`, then four of
them fed to the real model: **`crate ids == reference ids: True`**, and retrieval **4 / 4**.

**The finding worth keeping.** `falling` alone is `[7349, 154]` — `fall` + `ing` — but ` falling`, with the space, is the
single token `[3064]`. **The same word is one token or two depending on context.** A tokeniser that split on whitespace
and looked each word up would produce the wrong ids for every phrase in a description, and nothing downstream would
fail: the ids are in range and the model still returns a unit vector.

**Three bugs, all found by tests.** `splitn(2, ':')` breaks on `":"` and `","`, which are real tokens written
`"\":\": 6`. `find("vocab")` finds the one inside `added_tokens` rather than `model`'s. And splitting the merge table on
commas breaks on its pretty-printed nested arrays.

And one that was **my test, not the code**: an early harness hard-coded `rain falling` as `[9946, 7349, 154]`, which is
what `falling` tokenises to alone. The crate was right and the harness was wrong, and it reported `False` until it was
fed the crate's own output.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 174 / 183 / 184 / **293 all-features**, clippy 0, fmt, rustdoc, examples — green.

**A disk failure that reads as a code error.** The gate failed with `No space left on device (os error 28)` reported
against `proc-macro2` — the disk was at **121 MB**. `cargo clean` (20.8 GiB) fixed it. Third time in this project.

**Evidence.** `runs/2026-11-03-66/`: `research.md`, `plan.md`, `perf.md`, `eval.md`, `review.md`,
`evidence/tokenizer-e2e.txt`, `evidence/ids.json`, `evidence/spike/`.

**Commits.** `33ef8f4`.

**Top 5 remaining** — the tokeniser done, the path now runs into the retrieval refinements: `#41` weights, `#40` the
ensemble, `#28` the tool host, the integration test, and case 31.

## Iteration 65 — 2026-11-03 — **the critical path closes**: a 44.1 kHz library converts

**Baseline.** Started at **1.6 GB free** (100 %), `target` 23 GB — the same obstacle as iteration 64 and for the same
reason. `cargo clean` freed 21.9 GiB in 2.5 s under the standing approval for this repo's own build cache. Then every
step was green: 174 / 183 / 184 + 6. Case 31 is still the only red case and still blocked on the `CueProblem` enum
decision, which is not on this path.

**The item — and it is the last one.** `#12`'s conversion closes the semantic critical path. **All nine issues
#2 → #3 → #7 → #9 → #10 → #11 → #12 → #15 → #17 → #26 are now closed.**

**`rubato` lands, and the first implementation was wrong twice.** Both defects are worth recording because both
produce a result that looks right:

1. **The resampler's latency was not skipped.** `FftFixedInOut` cannot emit its first sample until its window is full;
   `rubato` reports the cost as **560 samples** at 44.1 → 48 kHz. Without the skip the tone is displaced by most of a
   cycle — and **energy at 1 kHz is still 0.494**, so a test asking "is there a 1 kHz tone" passes on a broken
   conversion. Max error against a pure reference: **1.765444 raw, 0.071178** after the skip.
2. **The loop stopped when the input was spent.** With the delay to pay and a padded final chunk, a 1.000 s file gave
   **47600** samples of real audio instead of 48000, and the tone read as **991 Hz** instead of 1000. The loop now runs
   until `expected + delay` samples exist.

**Both numbers are asserted now**: exactly **48000** samples from 1.000 s, and a tone that keeps its frequency
(998–1002 crossings), its energy (within the measured 0.9763 band) and its place (15 kHz survives, 9.1 kHz does not
appear). Awkward lengths — 1, 7, 1000, 1023, 1025, 4411 samples — all keep their duration.

**The refusal stopped firing on the common case.** `decode_for_model` downmixes and converts, so a 44.1 kHz library
indexes. `buffer::model_buffer` still refuses a rate it cannot convert, and a test proves that low-level contract
survives for a caller that skips the conversion.

**Two ratchets fired, which is what they are for.** `leaf::the_dependency_list_stays_closed` and
`library::the_semantic_feature_declares_no_dependency_of_its_own` both rejected `rubato` the moment it appeared; both
were widened with the sentence that argues for it, in the same commit. The feature now names three dependencies and
each has its argument: `ort` (inference), `rodio` (decoding, **no new crate** — `bevy_audio` resolves it), `rubato`
(resampling, pure Rust).

**The default build is still untouched**: `cargo tree --edges normal` is **413 lines** with no features, before and
after.

**Judged by measurement and by looking.** `examples/render` wrote 32 files; `render.txt` reports the same variant
sequences and gains as before. The bang's spectrogram shows broadband attack decaying to a low tail with **no click
line, no clipping shelf, no gap**; the mix waveform shows a clean attack, the regular cycles a duck produces, and a
smooth decay with **no step**. Loudness: **−18.6 LUFS** integrated, **−6.1 dBFS** true peak — a game mix comfortably
under −14 LUFS with 6.1 dB of headroom.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 174 default / 183 serde / 184 mixer / **282 all-features**, clippy 0, fmt, rustdoc, examples — green.

**Perf.** Not re-measured and `perf.md` says why: no audio-path code changed, and the 413-line graph proves it. Iteration
64's baseline stands: **0.024 / 0.284 / 0.667 µs** per frame at 8 / 64 / 256 voices.

**Evidence.** `runs/2026-11-03-65/`: `research.md` (the two defects and the numbers), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes), `evidence/after/` (32 render files).

**Commits.** `dec90b6`.

**The path is closed. Top 5 remaining, none of them on it**

1. **A tokeniser** — the one thing between `Query::text()` and `Model::embed_text`. Its own dependency argument, and
   the last piece of the semantic pipeline that is not built.
2. **`#41`** — tunable weights as a serde type, which is where the rank floor stops being a design default.
3. **CUDA on `big`** — every measurement so far ran `CPUExecutionProvider`; `P-01`'s throughput target needs the 3090.
4. **`V-01`** — the labelled set, which turns the measured 0.126–0.429 / −0.104–+0.150 overlap into a chosen threshold.
5. `cues.yaml` case 31 — the only red case in the crate, blocked on the same public-enum decision as four other
   `needs-decision/` files.

## Iteration 64 — 2026-11-03 — decoding, and the first measured perf baseline

**Baseline.** Started at **642 MB free** — the first obstacle, and not a code problem. `cargo clean` in `bevy_siren`
(approved; 23.3 GiB of the crate's own build artifacts) freed 24 GB, and the rebuild took 43.5 s. Then every step was
green: 174 / 183 / 184 / 265 + 6, clippy, fmt, rustdoc, examples. Case 31 is still the only red case, still blocked
on the `CueProblem` enum decision.

**The item.** `#12`'s decode half — the last critical-path issue, and the one whose other half waits on a dependency.

**`rodio` costs no new crate, and that is measured.** `cargo tree -i rodio` reports
`bevy_audio → bevy_internal → bevy → bevy_siren`, so **`rodio 0.22.2 is in the default graph already**. The
`semantic` feature's `dep:rodio` entry turns on this crate's own `wav`/`vorbis` features and adds **nothing**:
`cargo tree --edges normal` is **413 lines** with no features, before and after. That is a stronger argument than
"we already trust it", and it is why reusing the mixer's decoder was the right call over adding a second one.

**What decoding proves.** `the_decoded_audio_is_the_tone_that_was_written` writes a 440 Hz tone and counts **exactly
220** rising zero crossings in 0.5 s — a decode that returned noise, silence or a shifted rate would fail where a
length check would pass. `rodio` also reports rate and channels as `NonZero`, which is a stronger promise than a
runtime check, so two `BufferProblem` variants are unreachable through this path.

**The refusal is the deliverable.** A 44.1 kHz file now **decodes successfully** and is then refused with both rates
named — *"the file is 44100 Hz and the model needs 48000 Hz"*. That is a different message from "your file is broken",
and it is the truthful one. The conversion is carved off to `needs-decision/resampler-dependency.md`, which now carries
four options, three measurements and a recommendation (`rubato`), plus the caveat that would bite: the final partial
chunk must be **flushed** or every file loses up to `input_frames_next()` samples — measured as a 480-sample gap.

**Two defects, one of them a lesson.** My zero-crossing bound was `(436..=444)` — a tolerance around a number that has
no tolerance, which is how a test stops proving anything; it is an equality now. And clippy caught a useless
`u16::try_from` around a value already `u16`. A third was caught by the pin rather than by me:
`the_semantic_feature_declares_no_dependency_of_its_own` failed the moment `semantic` grew `dep:rodio`, which is
exactly what it exists for, and was updated with the argument in the same commit.

**The first measured perf baseline.** `perfcheck`, release build 9 m 10 s:

| Voices | µs / frame | µs / voice |
|---|---|---|
| **8** | **0.024** | 0.0030 |
| **64** | **0.284** | 0.0044 |
| **256** | **0.667** | 0.0026 |

At the crate's largest budget the per-frame mix costs **under one microsecond**. Iteration 63 could not measure this
at all — the disk was at 1.2 GB and a release build would have failed as `ld: write() failed, errno=28`, the failure
this skill's own notes warn looks like a code error. That gap is now closed and these are the numbers the next
audio-path change must not exceed by more than 10 %.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 174 default / 183 serde / 184 mixer / **271 all-features**, clippy 0 errors, fmt, rustdoc, examples — green.

**Evidence.** `runs/2026-11-03-64/`: `research.md` (the graph measurement and what decoding proves), `plan.md`,
`perf.md` (the measured baseline), `eval.md`, `review.md` (three passes).

**Top 5 remaining**

1. **`rubato`** — `needs-decision/resampler-dependency.md` recommends it with the measurements behind the
   recommendation, and `#12` closes when it lands with a flushed final chunk and a test that 1.000 s at 44.1 kHz
   becomes exactly **48000** samples at 48 kHz.
2. **A tokeniser** — the last piece of the walking skeleton, and its own dependency argument.
3. **CUDA on `big`** — the probe ran `CPUExecutionProvider`; `P-01`'s throughput target needs the 3090.
4. **`#41`** — tunable weights, the runner-up this iteration.
5. `cues.yaml` case 31 — still the only red case, still blocked on the same public-enum decision.

## Iteration 63 — 2026-11-03 — the last decision points, and a pipeline that works end to end

**Baseline.** Fully green: 174 / 183 / 184 + 6, clippy, fmt, rustdoc, examples. Iteration 62 left no uncommitted work.
Case 31 is still the only red case and still blocked on the `CueProblem` enum decision.

**The item.** The three remaining decision points on the semantic critical path, done in the order their dependencies
require: `#10` gates `ort`, which gates `#17`, which gates `#26`. **Three closed, one narrowed by measurement.**

**What the text tower measured.** 124,645,632 parameters, **501.4 MB**, 1,016 nodes, 31 standard operators, cosine
**0.999999969**. Its **token axis is dynamic** — 4 and 15 tokens ran in one graph — which is the opposite of the audio
tower, whose batch axis is baked into a `Reshape`. Query latency **5.28 ms** on an M4 CPU, six times cheaper than
indexing a clip.

**The finding that changed code.** An exact export says nothing about whether a description lands near the sound it
describes. Measured on six clips and five descriptions: four of five winners correct, and the bands are
**correct 0.126–0.429, incorrect −0.104–+0.150** — they **overlap**. A floor of 0.5, the number intuition reaches
for, would reject **every correct match**. So `Floor`'s default stays `0.0`, and a `const` assertion now stops the
**build** if a later measurement ever finds the bands separated. `an_intuitive_floor_would_have_rejected_every_correct_match`
pins the trap.

**`ort` landed, measured first.** It builds in 3.2 s and then **fails to link** without `download-binaries`; that alone
**fails to compile** without a TLS feature. **Two features, not one** — exactly the thing a consumer adds one of and
abandons the crate over. With both, it loads the 501 MB graph, reports `[-1, -1]`, and returns a unit-norm vector. The
default build is proven unchanged: **413 dependency lines with no features, 424 with `semantic`**, and `ort` in exactly
one of them.

**The pipeline, end to end.** `search.rs` takes a validated `Query`, an index, an embedding and a floor, and returns an
`Answer` that **counts what it dropped** — because a search returning 1 of 3 is indistinguishable from a library of one
asset otherwise. On real audio in `bmb`: **3 of 4**, and the miss is a **0.006** tie between two clips both generated
from white noise. That tie is the argument for a score on every result: a bare "best match" would have hidden it.

**Five defects the tests found.** `batch_sizes` built its capacity from the model's own cost, so a dynamic graph
reported **one** size — an optimistic number that fails at run time and passes a test that only checks it is non-zero.
The tiny ONNX fixture's `MatMul` assumed a fixed 8 tokens. A constant `assert!` clippy rejected and that should have
been in a `const` block. `ort`'s API had moved in four places, all recorded in ADR 0002 because a consumer hits them
too. And **`Model::load` created an ONNX Runtime environment per call**, which `ort` refuses — *"only one environment is
allowed per process"* — so it worked in a test that loaded one model and would have failed in every real application,
which loads at least a text tower and an audio tower. Found only because the suite ran together rather than one test at
a time; fixed with a `OnceLock`, in `1be5ee7`.

**The resampler, measured rather than argued.** `rubato 0.16.2` builds in 2.12 s and keeps a 1 kHz tone at **0.9763**
of its energy and a 15 kHz one intact, where `rodio`'s converter discards samples. So `#12`'s decision is now one
dependency wide, with a number to implement against and one implementation note: the final partial chunk must be
**flushed** or a file loses up to `input_frames_next()` samples.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 174 default / 183 serde / 184 mixer / **265 all-features**, clippy, fmt, rustdoc, examples — all green.

**Perf.** **5.28 ms** per query, **3.2 s** to build `ort`, **2.12 s** for `rubato`. `perfcheck` not run and `perf.md`
says why: no audio-path code changed, so it would measure iteration 62's crate and report it as new.

**A machine note that matters next time.** Local disk ended at **1.2 GB free**; `target` is 22 GB. A release build
needs ~2 GB, so the next iteration that wants `cargo run --release` must free space first, and a full disk will look
like a linker error.

**Evidence.** `runs/2026-11-03-63/`: `research.md` (four measured sources), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes), and `evidence/` — `ort-probe.txt`, `resampler-probe.txt`, `e2e.txt`, `spike/{text,space,e2e}.json`.

**Commits.** `81a6a1f` text tower + floor, `f52d79f` `ort` + session, `f317a27` search + model, `1be5ee7` the environment singleton.

**Top 5 remaining**

1. **`#12`** — add `rubato` with the argument, flush the final chunk, and prove a 44.1 kHz file arrives at 48 kHz with
   its tone intact. The probe's numbers are the target.
2. **A tokeniser** — the one thing between `Query::text()` and `Model::embed_text`. Its own dependency argument, and the
   last piece of the walking skeleton that is not built.
3. **CUDA on `big`** — the probe ran `CPUExecutionProvider`; `P-01`'s throughput target needs the 3090.
4. **`V-01`** — the labelled set, which is what would turn the floor from a design default into a measured threshold
   and stop `T-05`'s explanations being qualitative.
5. `cues.yaml` case 31 — still the only red case, still blocked on the same public-enum decision.

## Iteration 62 — 2026-11-03 — the semantic critical path, bottom-up

**Baseline.** Fully green in `bevy_siren` — 163 / 172 / 173 + 6, clippy, fmt, rustdoc, examples. Iteration 61 left no
uncommitted work. Case 31 (`gain_db` silent) remains the only red case and remains blocked on the `CueProblem` enum
decision.

**The item.** Not one feature but the **critical path of the 48-issue backlog filed this session**. Worked in
dependency order, nothing stubbed: **6 issues closed, 3 advanced**.

**#2 — the CLAP export spike, and its answer.** `laion/clap-htsat-fused` exports to ONNX at **cosine 1.000000000**,
**6.8 × 10⁻⁷** max absolute difference, opset 17, 39 standard operators, 116.5 MB, `onnx.checker` passing. Two findings
beyond the question: **every duration yields 1001 mel frames** (so chunking is a counting problem) and **dynamic batch
is broken** at batch ≥ 2 (so `N-02` needs a ladder of fixed-batch exports). Latency 34.9 ms/clip on an M4 **CPU**.

**#3, #7 — the two ADRs.** `ort` + CUDA EP, because the export turned out exact; `candle` is the fallback on coverage
and the sidecar on a measured trigger. One library with two front ends, index owned by the library, playback only
through the plugin's existing messages.

**#11 — the library manifest.** Identity is the **content hash**, not the path, because that is the distinction an
incremental index needs: a rename is **0 adds / 0 removals**, a re-encode in place is **1 / 1**. Rows headers only, no
decode; a symlink out of the root is refused; every file it does not index is reported with a reason.

**#9 — the feature skeleton, proven.** `cargo tree --edges normal` is **413 lines with and without**
`--features semantic`. The contract is measured, not asserted.

**#12, #17, #26 — the device-free half of each.** `buffer.rs` (downmix, rate agreement), `budget.rs` (what fits, what a
ceiling forbids, the batch a leftover takes), `rank.rs` (cosine, top-k, total order). No model call yet, because that
needs `ort`, which `#10` gates — so each does its whole job and is tested rather than stubbed.

**The decision that took a measurement to make.** `#12`'s decode half needs a resampler, and the crate's dependency
list is closed. `rodio 0.22.2`'s own doc comment settles it: *"simple linear interpolation for up-sampling and discards
samples for down-sampling"* — **aliasing**. So `model_buffer` **refuses** any rate but 48 kHz with
`ResamplingRequired { from, to }`, naming both rates and `needs-decision/semantic-dependencies.md`. An aliased
embedding is a plausible vector and nothing downstream can tell, which is the failure this repository keeps paying for.

**Five defects the tests found.** A RIFF with no `fmt ` chunk was indexed instead of reported corrupt; my own re-encode
fixture was wrong; the interpolator contradicted the decision file and was removed; `IndexProblem` was large enough to
pad every `Result` (clippy); and the GPU ceiling was reported as device space, sending a reader to fix the wrong thing.

**Verdicts.** Pass 1 **GREAT**, pass 2 **GREAT**, pass 3 **GREAT**.

**Gate.** 174 default / 218 semantic / 237 all-features, clippy, fmt, rustdoc, examples — all green.

**Perf.** One real number, the model's: **34.9 ms/clip on M4 CPU**. `perfcheck` was not run and `perf.md` says why: no
crate code on the audio path changed, so it would measure iteration 61's code and report it as new.

**Evidence.** `runs/2026-11-03-62/`: `research.md` (five measured sources), `plan.md` (the ranked list and what was
carved), `perf.md`, `eval.md`, `review.md` (three passes), `evidence/spike/clap-export.json`.

**Commits.** `36990e6` ADRs, `c40c00d` library, `c0228a0` feature skeleton, `0f1b9b9` buffer, `4c1a375` index,
`fa21716` budget, `43431c2` ranking.

**Top 5 remaining**

1. **`#10`** — extend `tests/leaf.rs` with the crates the ADR named, so `ort` can land and `#17`/`#26` can finish.
   Everything on the path behind it waits on this.
2. **`#3`'s batch ladder** — export CLAP at batch 1 and 8, because dynamic batch is broken. A measured constraint, not
   an optimisation.
3. **`#26`'s model call** — text encode plus `top_k` against a real index; the walking skeleton's exit condition.
4. **`#12`'s decision** — `semantic-dependencies.md` needs an answer (recommendation: `ort` plus an explicit resampler).
   Until then every non-48 kHz asset is refused, which is the commonest rate a game ships.
5. `cues.yaml` case 31 — still the only red case, still blocked on the same public-enum decision.

## Iteration 61 — 2026-11-03 — the Cauldron's `bevy_siren` feature never compiled

**Baseline.** `bevy_siren` green on every step. But the **consumer** was red, and had been for a long time: two commands
the skill names every iteration had never been run here.

**Change (all in `bevy_grimoire`, its own commit `d6bb032`, per the repo rules).**
1. **Deleted the duplicate `pub mod audio_panel;`** — declared at `lib.rs:53` unconditionally and again at `109` behind
   the feature, so `--features bevy_siren` failed with `E0428`.
2. **Defined `AudioPanelPlugin`** — `editor.rs:2428` has been adding it; `audio_panel.rs` never had it (`E0425`). It now
   registers `AudioPanel`, `read_audio` in `PreUpdate`, and `on_audition` as an observer.
3. **Carried the `music` section**: a `music: MusicScore` with `#[serde(default)]` in the reader and the writer, and
   `AudioPanel::score` to hold it.

**Both compile faults are pre-existing**, confirmed by stashing the only local edit and re-checking: the same two errors
on a clean tree.

**The score was dropped, measured** with a standalone program using the same `serde`+`ron` types:

| | before | after |
|---|---|---|
| read `(settings, cues, music)` | `File { settings, cues }` — **music gone** | tempo **120.0** carried |
| write back | `(settings:32,cues:0)` — 2 keys | 3 keys, lossless |
| read a file with **no** `music` | n/a | `MusicScore::default()` |

The panel's own module doc says the file is `SirenSettings`, `CueTable` **and `MusicScore`**, and `read_file`'s says *"a
score is a third section and is optional"* — "optional" had been implemented as "unread", because RON ignores an unknown
key without `deny_unknown_fields`.

**A shape that would have refused every real file.** `Option<MusicScore>` with `#[serde(default)]` is the obvious fix and
it fails with `ExpectedOption`: RON parses a bare struct into an `Option` only when the field is spelled `Some(..)`. The
correct shape is a **plain `MusicScore` with `#[serde(default)]`**, whose default is the same value the serde field
default uses — which is iteration 53's fix that made `MusicScore::default()` four beats.

**Tests.** `--test editor audio_panel`: **5 passed, 0 failed**, including the existing round trip and the new
`a_music_score_survives_the_panels_file`. The four `a_person_finds_and_runs_a_command_in_the_palette` tests fail
**identically with the feature off**, as `CLAUDE.md` records — not mine.

**A trap the skill names, which caught the loop anyway.** The first test attempt reported compiler errors on a dozen
crates; the real message was **`No space left on device`**. The Cauldron's test target pulls `wgpu` and needs several GB;
the machine had 2.5 GB. `cargo clean` in `bevy_siren` (37.3 GiB of compiled artifacts, nothing tracked) freed 32 GB, asked
and approved because it is a target this loop did not create.

**No perf figure this iteration**, and `perf.md` says why: the cleanup predates the release rebuild, and `src/` did not
change.

**What this says about the loop.** Twenty-odd iterations ran crate-side work while the *consumer* was never built. The
skill names `cargo test -p bevy_cauldron --features bevy_siren --test editor audio_panel_reads_the_crates_file` in step 5
every time. A feature that has never compiled, and a file type that silently drops a third of its input, are both
invisible from inside `bevy_siren` — whose own tests were green throughout.

**Evidence.** `runs/2026-11-03-61/`: `research.md` (S1 the module doc, S2 `read_file`'s doc, S3 the `editor.rs` call site,
S4 `deny_unknown_fields` on the crate's types but not the panel's, S5 iteration 53's default), `plan.md`, `perf.md`,
`eval.md`, `review.md` (three passes, GREAT), `evidence/after`.

**Commit.** `d6bb032` in `bevy_grimoire`; this repository records the iteration.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it. Same open enum decision.
3. **A stinger on the wrong bus writes only a log line, not a `Refused`** — `needs-decision/stinger-refusal.md`; same
   decision.
4. **The Cauldron's panel now compiles but draws nothing from the score** — `AudioPanel::score` is read and written, and
   no UI row shows the layers. The next Cauldron iteration's obvious work.
5. `needs-decision/exact-pitch-draw.md` — option B for the next minor.

## Iteration 60 — 2026-11-03 — a draw that cannot change its answer

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** Documentation and a test, no `src/` behaviour change: the guide's `## The seed` section gains the fact that a
play draws for its detune **even when the cue's `pitch` range is exact**; `needs-decision/exact-pitch-draw.md` carries the
options; and `tests/play.rs::an_exact_pitch_range_still_draws_from_the_stream` pins the coupling.

**Rationale.** `answer` computes `cue.pitch.at(choices.rng.next_f32())` before it can know the range cannot use the
number. `PitchRange::at` is `low + (high - low) * unit`, so with `low == high` — what `PitchRange::exact(n)` builds — the
draw **cannot** change the answer, and it is spent from the one stream every cue shares.

**Measured**: an app that plays an exact-pitch cue four times before a `Shuffle` cue produces a **different** six-play
sequence than one that never played it. Same seed, same table. So a project's soundscape is a function of its seed **and
its play order**, not its seed alone — which `docs/guide/cues.md` had promised was not the case.

**Why not patched.** The fix is three words, and it **re-rolls every project's soundscape** in the build that ships it,
with no authoring change to explain it. That is a release note, not a patch: option B in the decision file, recommended for
the next minor. The test is written to **flip** — `assert_ne!` with the decision file named in its message — so the day B
ships, the test says why.

**Six hypotheses probed, all correct, all deleted** rather than kept (a test asserting untouched behaviour is noise):
`SirenLog`'s chronology through out-of-order eviction; `semitones_to_ratio`'s clamp against the mixer's `set_speed`;
pitch composition with a `Play` override; `load_table`'s ask-then-not-missing semantics; a muted bus still playing; and
two music layers settling on a one-voice bus.

**One probe failed first and the code was right.** I asserted `blocked("combat")` on a one-voice Music bus; the log showed
`played calm → stopped calm: stolen by combat (bus) → played combat → refused calm: no voice free`. The **stolen** layer
is the one refused and recorded — the documented virtual-voice path working. Deleted rather than inverted, because
`tests/music.rs` already covers it as case 33.

**Nothing audible moved**: `bus-Music` still `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still
−18.6 LUFS at −6.1 dBFS.

**Evidence.** `runs/2026-11-03-60/`: `research.md` (S1 `PitchRange::at`'s arithmetic, S2 the seed's promise, S3 iteration
51's precedent, S4 who writes an exact range and why), `plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT),
`evidence/{spectrograms,after}`, and `docs/loop/needs-decision/exact-pitch-draw.md`.

**Commit.** `9542b45` — *docs: a detune draw that cannot change its answer*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **A stinger on the wrong bus writes only a log line, not a `Refused`** — confirmed by probe in iteration 59.
   `needs-decision/stinger-refusal.md`; same decision.
4. **`needs-decision/exact-pitch-draw.md`** — option B (skip a draw that cannot change its answer) is the next minor's
   change, with its release note.
5. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.

## Iteration 59 — 2026-11-03 — an empty bus is not a bus mixed to the floor

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** `SirenMix::peak_db(bus) -> Option<f32>` — **additive**. The example uses it and its `voices_on > 0` filter
is gone; the guide's meter snippet now matches.

**Rationale.** `worst_peak_db` answers `SILENT_DB` for a bus with **no voices**, which is the same number it answers for a
bus **mixed to the floor** — while its own doc says *"nothing playing is not `SILENT_DB` — it is no peak at all"*. The
API's own sentence, contradicted by its own body.

| | `worst_peak_db` | `peak_db` |
|---|---|---|
| no voices | `SILENT_DB` (−80.0) | **`None`** |
| 2 voices, bus at the floor | `SILENT_DB` (−80.0) | `Some(−80.0)` |
| 2 voices, bus at −70 dB | −64.0 | `Some(−64.0)` |

**Ranked honestly as rank 3, not rank 1.** Every caller here filters `voices_on > 0` first, so no output is wrong today
— it is a latent trap. But the workaround is in the crate's **own example**, which is how a consumer would meet it.

**`worst_peak_db` was not changed.** A signature change to `Option<f32>` would break every `> x` comparison on a
**published** crate (0.1.0, nine downloads) for a naming nit; the one-number form is what some callers want.

**Two probes that were correct and deleted**: `load_table` treats an asked key as not-missing and asks nothing the
second time; a muted bus still plays with `Played` carrying the cue's own gain.

**A probe that found the behaviour is protective, not buggy.** `answer` picks before it validates, so a `MissingSound`
refusal does spend a draw and advance the window. But with one file missing and `avoid: 1`, **4 of 8 requests still
sound** — without advancing the picker it would re-pick the missing file for ever. And the two cannot be separated:
`pick_from` draws and advances in one call. Recorded in **`needs-decision/pick-before-validate.md`** with both
measurements, so a later iteration does not "fix" it into a livelock.

**A third probe confirmed a real gap that stays blocked**: a wrong-bus stinger is invisible to a program (`refused` is
empty), and closing it needs a new `Refusal` variant — the public-enum break `needs-decision/stinger-refusal.md` awaits.

**Nothing audible moved**: `bus-Music` still `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still
−18.6 LUFS at −6.1 dBFS. The panel capture is **unchanged** from before, which is the claim: a caller's picture does not
move when a sentinel is replaced by a type that cannot collide.

**Evidence.** `runs/2026-11-03-59/`: `research.md` (S1 the doc's own sentence, S2 `SILENT_DB` as a level borrowed for
absence plus the crate's `Option` precedent, S3 Gregory on buses as levels, S4 iteration 49's bound), `plan.md`,
`perf.md`, `eval.md`, `review.md` (three passes, GREAT), `evidence/{panel,spectrograms,after}`.

**Commit.** `85f4975` — *mix: an empty bus is not a bus mixed to the floor*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **A stinger on the wrong bus writes only a log line, not a `Refused`** — confirmed by probe this iteration
   (`refused` is empty). `needs-decision/stinger-refusal.md`; same decision.
4. `needs-decision/pick-before-validate.md` — a refused play spends a draw and advances the window. **Protective** and
   measured; a strict version needs a rule `pick_from` cannot express.
5. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.

## Iteration 58 — 2026-11-03 — a layer change that names nothing says so

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** `move_layers`' `if let Some(layer) = …find(..)` became a `let … else` writing
``refused `{name}`: this score has no layer `{name}`, so the change names nothing``. Plus `SIREN_LAYER=typo` in
`examples/mixer`, its capture, the guide's paragraph, and a CHANGELOG entry.

**Rationale.** `MusicLayers` was the one message in the crate with **no failure path at all**: no `Refusal` arm, no log,
no counter. `find` returning `None` inside an `if let` reads as *"apply it if the layer exists"* — true, and hiding that
the caller's whole intent was discarded. A typo in a layer name was a change that did nothing and said nothing, which is
exactly what `src/lib.rs:50` promises the crate never does.

| | log |
|---|---|
| before | **only the layers' own `played` lines** — nothing about the typo |
| after | ``refused `cmobat`: this score has no layer `cmobat`, so the change names nothing`` |
| a real layer in the same change | still moves to its target |
| tests | 156 / 164 / 166 + 6 |
| after | **157 / 165 / 167 + 6** |

**Why the log and not a new `Refusal` variant.** The score is not the mix table, so `UnknownBus` would be a false
sentence; a new variant is the public-enum break three open decisions already wait on. Iteration 9 set this route for the
stinger's wrong bus. The entry is a `Refused`, and the log's own rule — *"a refusal outlives a play"* — means a busy scene
cannot scroll it away.

**Three probes that passed**, all deleted rather than kept: a stinger's duck recovering after its voice is gone, a deeper
stinger handing back to a shallower dialogue duck, and a negative/`NaN` `duck_s` still recovering.

**Three captures before one worked, each failure informative.** The panel shows the newest **six** log lines, and the
layer step fires on the bar: 96 frames landed it at the very end; 200 frames fired it but scrolled it out behind later
plays; `SIREN_KEYS=layer SIREN_KEY_EVERY=200` fires it **once**, and `docs/media/mixer-layer-typo.png` then shows
`combat -3.0 dB` and the refusal sentence on one screen.

**Nothing audible moved**: `bus-Music` still `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still
−18.6 LUFS at −6.1 dBFS; the music cue's spectrogram is a clean attack with silence after 0.85 s.

**Evidence.** `runs/2026-11-03-58/`: `research.md` (S1 `MusicLayers`' doc, S2 the crate's "nothing plays with no
explanation", S3 iteration 9's log-not-variant precedent, S4 the `if let` trap, S5 the log's eviction rule), `plan.md`,
`perf.md`, `eval.md`, `review.md` (three passes, GREAT), `evidence/{panel,spectrograms,after}`, and
`docs/media/mixer-layer-typo.png`.

**Commit.** `bbd73ac` — *music: a layer change that names nothing says so*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **A stinger on the wrong bus writes only a log line, not a `Refused`** — `needs-decision/stinger-refusal.md`; a host
   reading `Refused` sees an unknown-cue stinger but not a wrong-bus one. (This iteration shows the log route works, but
   a program still cannot react.)
4. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.
5. `reset_histories` has no caller — a consumer item for `bevy_grimoire`.

## Iteration 57 — 2026-11-03 — a stinger that cannot dip does not claim it did

**Baseline.** Green on every step, no open issues. Nothing red. `bevy_siren` **0.1.0 is published** (crates.io,
2026-11-02, 9 downloads), so the `CueProblem` break the two blocked items wait on is a real break against a real
consumer — the decision stays open.

**Change.** One condition in `arm_stinger_ducks`: arm and log the duck only when `duck_db.is_finite() && duck_db < 0.0`,
which is `active_duck_db`'s own guard read from the other side.

**Rationale.** `arm_stinger_ducks` pushed `Ducked { by_db }` **unconditionally**, while `active_duck_db` returns `0.0`
for any `by_db >= 0.0`. So a `Stinger { duck_db: 0.0 }` — a depth a project computes from gameplay — wrote a line saying
the music stepped back, and the music did not move.

| | log | `Music` |
|---|---|---|
| before | **`[Ducked { by_db: 0.0, by: "stinger" }]`** | unmoved |
| after | **`[]`** | unmoved |
| −9.0 (non-regression) | `[-9.0]` | below −0.5 dB |
| +6.0 (a lift) | `[]` | unmoved |
| tests before | 154 / 162 / 164 + 6 | |
| tests after | **155 / 163 / 165 + 6** | |

**Why the log and not a report.** Iteration 52 fixed the same inert path for a table *rule*, where `problems()` could
name it. A `Stinger` is a **message** sent at runtime — `MusicScore::problems(&table)` cannot see one that has not been
sent — so the log line was the only place a person could learn what a stinger did, and it said the opposite of what
happened.

**Nine hypotheses probed, one bug found.** Cue-cap steal order was checked four ways (including archetype reuse after a
free), plus a stopped voice's duck recovery and a bus level changed mid-voice — all already correct. Those four cue-cap
probes were **deleted** rather than kept: they assert behaviour no change touched, and `tests/play.rs` is not a museum for
this loop's questions.

**Two self-inflicted breakages, both caught by the gate.** My removal script cut mid-comment and left four orphaned doc
fragments that would not parse; then the deletion left an unused import and a non-snake-case test name. Both fixed. The
lesson, written here because it is the second iteration running: **re-run `clippy --all-targets` after any test
removal**, not only after additions.

**No clip, and why.** The panel draws each music layer's `dB -> target`; a stinger's dip appears there for its hold, but
the *absence* of a dip is not drawable. It names its test: `a_stinger_at_zero_db_logs_no_duck`.

**Nothing else moved**: `bus-Music` still reads `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still
integrates −18.6 LUFS at −6.1 dBFS.

**Evidence.** `runs/2026-11-03-57/`: `research.md` (S1 `Stinger::new`'s own doc, S2–S3 the two guards, S4 iteration 52's
rule fix, S5 `arm_stinger_ducks`' own "not before the play is answered" reasoning), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{spectrograms,panel,after}`.

**Commit.** `cc86578` — *music: a stinger that cannot dip does not claim it did*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`, and 0.1.0 is published, so the break is real.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **A stinger on the wrong bus writes only a log line, not a `Refused`** — `needs-decision/stinger-refusal.md`; a host
   reading `Refused` sees an unknown-cue stinger but not a wrong-bus one.
4. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.
5. `reset_histories` has no caller — a consumer item for `bevy_grimoire`.

## Iteration 56 — 2026-11-03 — a sound key the table dropped stops holding its handle

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** `SirenSounds::retain_keys(&table)` — drops every key the table no longer names. Plus the guide's reload
section and a CHANGELOG entry.

**Rationale.** `SirenSounds` exposed `insert`/`get`/`len`/`keys`/`load_table`/`missing`/`failed` and **no remover at all**,
and `load_table` **merges** — it asks for keys it has no handle for and never drops one the table has lost. So a project
that reloads a table with renamed keys kept one handle per key it had ever loaded, for the life of the process.

**The handle is the cost, not the map.** `Handle<AudioSource>` is 24 bytes and the entry overhead ~62 KiB per thousand — but
`AudioSource` is `bytes: Arc<[u8]>`, the **whole encoded file**, and a strong handle keeps it loaded.

| | keys held | resident |
|---|---|---|
| two 10-key tables loaded in turn, before | **20** | 20 whole audio files |
| after `retain_keys` | **10** | the 10 the table names |
| tests | 143 / 151 / 161 + 6 | |
| after | **151 / 159 / 161 + 6** | |

**Honestly ranked, not dressed up.** Three rank-1 hypotheses were probed first and **passed**, and are kept as tests:
`a_stolen_voices_duck_recovers` (the duck reaches exactly `0.0` after the source bus empties),
`a_bus_level_changed_mid_voice_reaches_it` (`Music` 12 dB down within 0.5 s of the table being written), and the merge
itself. And a stale key is **never read** — `missing()`, `failed()` and every `sounds.get` take current-table keys — so
this is a **memory** bug, not a wrong-output one. It is logged as such.

**Nothing audible moved**: `render.txt`'s variant streams and the duck are byte-identical to iteration 55's; `mix.wav`
still integrates −18.6 LUFS at −6.1 dBFS.

**`load_table` was left alone.** Its contract is *"returns the keys it asked for that it had no handle for yet"* — ask for
what is missing. Folding a removal into it would silently change what an existing caller's map means.

**Evidence.** `runs/2026-11-03-56/`: `research.md` (S1 Böttcher's *"management of thousands of assets"* — this module's own
citation, S2 `AudioSource`'s `Arc<[u8]>`, S3 `load_table`'s contract, S4 the two retains before this one), `plan.md`,
`perf.md`, `eval.md`, `review.md` (three passes, GREAT), `evidence/{spectrograms,after}`.

**Commit.** `7955e82` — *siren: a sound key the table dropped stops holding its handle*, plus `f1d39d9` — *tests: use the
second table in the merge test, so clippy passes*. The second commit fixes a red gate in the first: my merge test built
`table_of("second")` and never used it, which is an unused-variable **error** under
`clippy --all-targets --all-features -D warnings`. Caught by re-running clippy against the pushed commit rather than
trusting the gate result from before the final edit.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`load_table` does not call `retain_keys` itself** — the retain is the caller's. A one-line change would make every
   project benefit, but it changes `load_table`'s documented contract, so it wants a decision file rather than an
   iteration's judgement.
4. **`reset_histories` has no caller** — a consumer item for `bevy_grimoire`.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; a lower default is a decision file.

## Iteration 55 — 2026-11-03 — a cue the table dropped stops holding a picker

**Baseline.** Green on every step, no open issues. Nothing red. `/tmp/pr-target` (1.7 GB) was this loop's own scratch from
iteration 54 and was removed, leaving 5.8 GB.

**Change.** `SirenChoices::retain_cues(&table)` + `picker_count()`, called from `play_requests` **only when the table
changed**. Plus a guard for an empty map, the guide's new section, and a CHANGELOG entry.

**Rationale.** Iteration 54 fixed `DuckClock::started`; this is the same shape one resource over. `pickers` is keyed by cue
name and created on a cue's first play, and nothing removes one — `reset_histories` clears each picker's **contents**, so
it is not a retain. A project that generates cue names (Franco: the request *"says what kind of sound it wants"*) grows
one picker per name ever played, each holding an **owned copy of its recent sound keys**.

| | pickers held |
|---|---|
| 200 plays | 200 |
| after `reset_histories` | **200** — clears contents, is not a retain |
| table cut to 10, before | **200** |
| table cut to 10, after | **10** |

Tests: 144 / 154 / 156 + 6 before, **147 / 155 / 157 + 6** after.

**A regression I introduced and measured away.** The first version was
`pickers.retain(|name, _| table.get(name).is_some())`. `CueTable::get` is a **linear scan**, so that is O(pickers × cues)
every frame: **989.8 µs at 1000 cues**, 6 % of a 60 Hz frame. One pass into a `HashSet` is **8.5 µs**; guarded on
`table.is_changed()` it is **0 ns** on a static table, which is the ordinary case.

**The test runs on both trees.** `the_picker_map_grows_per_cue_name` reads the map through `Debug`, so it compiles
against the old crate and fails there: `left: 200, right: 10`.

**Nothing moved.** `render.txt`'s variant streams are **byte-identical** to iteration 54's, and `bus-Music` still reads
`duck -8.0 dB, recovered by 1.97 s`. A retain that dropped a live cue's picker would reset its history and change the
next variant — the bug a cleanup like this introduces if it is wrong.

**Evidence.** `runs/2026-11-03-55/`: `research.md` (S1 Franco on runtime cue names, S2 Gregory's budget discipline,
S3 the picker's owned strings, S4 `CueTable::get`'s linear scan, S5 iteration 54's precedent), `plan.md`, `perf.md`
(the 989.8 µs → 8.5 µs → 0 ns ladder), `eval.md`, `review.md` (three passes, GREAT),
`evidence/{plots,after}`.

**Commit.** `f7030f7` — *siren: a cue the table dropped stops holding a picker*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`SirenSounds` is never pruned either** — the key → handle map is filled by `load_table` and nothing removes a key a
   reload dropped. The third of the same shape; check whether `load_table` replaces or merges.
4. **`reset_histories` has no caller** — a consumer item for `bevy_grimoire`: the panel's reload wants `retain_cues`
   (which the crate now calls itself) and a seed change wants `reset_histories`.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; the guide says what to turn, a lower default is a decision.

## Iteration 54 — 2026-11-03 — a duck rule the table dropped stops holding its start

**Baseline.** Green on every step, no open issues. Nothing red. `/tmp/pq-target` (2.2 GB) was this loop's own scratch from
iteration 53 and was removed, leaving 8.9 GB.

**Change.** `DuckClock::retain_rules(&settings)` + `rule_starts()`, called by `update_mix` beside `prune`.

**Rationale.** `started` is keyed by `(source, target)` and was only emptied by `clear()`, which nothing calls;
`prune(now_s)` retained the *manual* envelope list and not the rules'. The two are different questions — *is this envelope
over?* (the clock can answer) and *is this rule still a rule?* (only the table can) — and conflating them is why one
collection was cleaned and the other was not.

**Measured** (through `Debug`, no API needed):

| scenario | before | after |
|---|---|---|
| 2000 fires, then `prune` at t=1000 | **2000 entries** | 0 |
| 2 fires, table names one | 2 | **1** |
| 100 table edits, table names one | 100 | **1** |
| a live rule across a retain | −8.0 dB | **−8.0 dB, unchanged** |
| the new call's cost | — | **13 ns/frame** (flat at 1/10/1000 starts) |
| tests | 143 / 151 / 153 + 6 | **144 / 152 / 154 + 6** |

**Reachable**: `DuckClock::fire` is public, and the crate's own `fire_all` grows the map whenever a table's rule set
changes — which the Cauldron does on every edit of `family/bevy_siren.ron`.

**A wrong claim from last iteration, corrected here.** Iteration 53 logged this as *"unobservable — no accessor, so no
capture could prove it"*. It was observable all along: `DuckClock` derives `Debug`. The accessor is worth adding for a
panel and a test, but the claim that nothing could prove it was false and it cost an iteration.

**A drawing mistake, caught by looking.** `showwavespic` on `bus-Music.wav` rendered **blank** — the file is a gain
staircase, not audio, and the skill says to draw a bus gain as a **line**. The first line plot then drew nothing because
its path used `var(--accent)`, which the rasterizer does not resolve; the final plot
(`evidence/plots/bus-music-gain.svg`) uses literal colours and reads in both. It shows 0.00 dB flat to 1.00 s, a
staircase to −7.96 dB at 1.27 s, recovery complete by 1.97 s — **no restart, no second duck**.

**Evidence.** `runs/2026-11-03-54/`: `research.md` (S1 Gregory's "for a while", S2 the voice budget's discipline,
S3 `prune`'s own doc naming the fault for one collection, S4 the bus-pruning precedent), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{plots,spectrograms,after}`.

**Commit.** `ee7eb67` — *mix: a duck rule the table dropped stops holding its start*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`SirenChoices::pickers` is never pruned either** — one `VariantPicker` per cue name, grown by procedure and never
   removed. The same shape as this iteration's bug, one resource over; it needs a `retain_cues(&table)` beside
   `retain_rules`.
4. **`reset_histories` has no caller** — the Cauldron's table reload should reset pickers; a consumer item for
   `bevy_grimoire`, its own commit.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; the guide says what to turn, a lower default is a decision.

## Iteration 53 — 2026-11-03 — `MusicScore::default()` is the score the file loader gives

**Baseline.** Green on every step, no open issues. Nothing red. Disk arrived at **100 % (3.3 GB)**; `/tmp/pz-target`
(1.7 GB) was this loop's own scratch from iteration 52 and was removed, leaving 4.9 GB. `target/` (38 GB) was not touched.

**Change.** `MusicScore`'s `derive(Default)` replaced by a handwritten `Default` returning
`beats_per_bar: Self::default_beats_per_bar()`. Plus a guide paragraph, a CHANGELOG entry, and four tests.

**Rationale.** `beats_per_bar` carried `serde(default = "MusicScore::default_beats_per_bar")` — **4.0** — while
`derive(Default)` gave it **0.0**. Two spellings of one field, and the `Default` had the wrong one.

**Measured, the same score two ways:**

| path | `beats_per_bar` | `bar_s()` | `problems()` |
|---|---|---|---|
| RON, key omitted | 4.0 | `Some(2.0)` | `[]` |
| `MusicScore::default()` | **0.0** | **`None`** | **`["beats_per_bar is not a positive number"]`** |

So the crate's own default produced a score the crate's own report called broken, with **no beat grid at all** — every
`NextBar` change landing on the frame it was asked for instead of at the bar (R5's quantisation has nothing to quantise
to).

**`tempo_bpm: 0.0` was left alone on purpose**: it is documented as *"no beat grid: every transition is immediate, for a
score that is a pad rather than a song"* — a meaning, not a missing number. Zero beats is only ever broken.

| | before | after |
|---|---|---|
| `Default().beats_per_bar` | 0.0 | **4.0** |
| `Default()` at 120 bpm, `bar_s()` | `None` | **`Some(2.0)`** |
| `Default().problems()` | 1 sentence about its own field | **only "no layers"** |
| a `NextBar` change from a `Default` score | lands immediately | **lands at 2.0 s** |
| tests | 139 / 147 / 150 + 6 | **141 / 149 / 151 + 6** |

**A mistake that damaged the repository, caught in the same step.** A `sed -i ''` in a **scratch crate** matched
`bevy_siren`'s own `[dependencies]` table through the path dependency and inserted `ron`/`serde` into the crate's
`Cargo.toml`. `git diff` showed it immediately; reverted.

**No clip, and why.** The panel does not draw the bar: `examples/mixer`'s `Music` block shows each layer's `dB -> target`
and a `waiting` marker, never `next_change_s()` or `bar_s()`. A capture would show an unchanged panel. It names its test
instead: `tests/music.rs::a_default_score_quantises_to_the_bar`, which **fails on the old tree** at the bar assertion.

**Not built.** `DuckClock::started` is never pruned — one entry per fired `(source, target)` for the process's life —
which is real but **unobservable**: `DuckClock` exposes no accessor for it, so no capture or test could prove it. Logged
for an iteration that adds one.

**Evidence.** `runs/2026-11-03-53/`: `research.md` (S1 the two defaults, S2 `bar_s`, S3 R5's quantisation, S4 Khan et al.,
S5 the crate's own "a number that disables a feature" precedent), `plan.md`, `perf.md`, `eval.md`, `review.md` (three
passes, GREAT), `evidence/{spectrograms,after}`.

**Commit.** `0de8501` — *siren: MusicScore::default() is the score the file loader gives*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`DuckClock::started` is never pruned** — one map entry per fired `(source, target)` forever. Needs an accessor
   before it can be proved, then a `prune` that drops entries no rule names.
4. **`reset_histories` has no caller** — the Cauldron's table reload should reset pickers; a consumer item for
   `bevy_grimoire`, its own commit.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; the guide says what to turn, a lower default is a decision.

## Iteration 52 — 2026-11-03 — a duck that lowers nothing now says so

**Baseline.** Green on every step, no open issues. Nothing red. **Disk was at 100 % (3.8 GB)** on arrival;
`/tmp/probe-target` (1.2 GB) was this loop's own scratch from iteration 51 and was removed, leaving 4.9 GB. `target/`
(38 GB) is not mine and was not touched.

**Change.** One sentence in `SirenSettings::problems()` for a duck whose `by_db` is exactly `0.0`, placed **before** the
envelope check. Plus `SIREN_MIX=zero` in `examples/mixer`, its capture, and the guide's sentence list.

**Rationale.** The two items above this on the backlog both need a **seventh `CueProblem`**, gated by the open
`bad-gain-problem.md`. Probing found a fault of the same character that needs no enum change: `SirenSettings::problems()`
returns **`Vec<String>`**, so a mix-table sentence is additive — the shape of iteration 48's bus-cap sentence.

`active_duck_db` returns `0.0` for any `by_db >= 0.0` (`src/model.rs:187`) and `duck_db` keeps a reduction only
`if db < deepest` with `deepest` starting at `0.0` (`src/mix.rs:489`). So such a rule fires, reaches the log, holds for
its whole `hold_s`, and the bus never moves.

**Measured, one duck each:**

| duck | before | after |
|---|---|---|
| `by_db: 0.0`, hold 0.25, fade 0.6 | **`[]`** | 1 sentence, names `by_db` not `hold_s` |
| `by_db: -0.0`, hold 0.25, fade 0.6 | `[]` | 1 sentence |
| the crate's `default_ducks()` (Gregory's −8 dB) | `[]` | **`[]`** — a real duck stays silent |
| `by_db: +6.0` | 1 sentence | unchanged |
| `by_db: nan` written as RON | **refused by the parser** (`Expected float`) | — not authorable, needs no sentence |

| | before | after |
|---|---|---|
| tests | 136 / 144 / 147 + 6 | **137 / 144 / 147 + 6** |
| `bus-Music` duck (render) | −8.0 dB, recovered 1.97 s | unchanged — the audio path is untouched |
| `mix.wav` | −18.6 LUFS, −6.1 dBFS | unchanged |

**Two things not built, and why.** `by_db: NaN` needs no sentence: RON refuses it at parse, and a non-finite number is
already neutralised by `active_duck_db`'s guard. `DuckClock::manual` can be handed `0.0` at runtime, but that is a
*computed* depth, not an authored one — reporting it would need a log entry, which is a public-enum change, and the
decision on those is open.

**A test I wrote wrongly.** `zero_depth_is_one_sentence_however_it_is_written` first asserted a zero-depth rule with no
envelope produces **two** sentences. It produces one: the no-hold sentence is guarded on `by_db < 0.0`, so a zero rule
can never reach it. The code was right; my expectation was not.

**The capture that would not have proved it.** `SIREN_MIX=broken` already fills the three-sentence cap with
`master_db is not a number`, the no-hold sentence, the self-duck and `and 1 more` — so a fourth fault there would be the
invisible one. `SIREN_MIX=zero` gives it its own table, following `overbudget`'s precedent.

**Evidence.** `runs/2026-11-03-52/`: `research.md` (S1 Gregory §14.6.3's duck, S2 grouping, S3–S4 the two guards, S5 the
report's existing vocabulary), `plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT),
`evidence/{panel,spectrograms,after}`, and `docs/media/mixer-zero-depth.png`.

**Commit.** `c063360` — *mix: a duck that lowers the bus by nothing says so*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`reset_histories` has no caller** — the Cauldron's table reload should reset pickers; a consumer item for
   `bevy_grimoire`, its own commit.
4. **`SirenSettings::master_db` defaults to `0.0`** — no trim, so a project from `default()` has no headroom. The guide
   says what to turn; whether the default should be lower is a decision file.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 51 — 2026-11-03 — a picker's window is what the cue asked for, not what its first play sized

**Baseline.** Green on every step, no open issues. Nothing red — the only red case in `cues.yaml` (31) is still blocked
on `needs-decision/bad-gain-problem.md`.

**Change.**
1. **`VariantPicker` keeps an explicit `window`**, not the deque's allocator capacity. `pick_from`/`pick` grow it to the
   `avoid` they are handed, so a rule read fresh from the cue is the rule enforced.
2. **`VariantPicker::set_capacity` / `capacity`** — additive public API.
3. **`Default` is `new(1)`**, not an empty picker that refuses nothing.
4. **`examples/render` writes each cue's variant stream** as a `variants <cue>:` line.

**Rationale.** Found by probing, not from the backlog. `SirenChoices::pick` built each cue's picker once, on its first
play (`or_insert_with`), and `remember` bounded the window by `VecDeque::capacity()` — an allocator detail, never the
authored number. Measured with four keys and seed 11: `capacity 1, avoid 3` produced the **byte-identical** 24-pick
sequence to `capacity 1, avoid 1`, `adbcbcabcacabcabcbabcbcb`, where `avoid: 3` requires `adbcadbc…`; **16 of 20** picks
returned within three of themselves. Reachable: the Cauldron rewrites and re-reads `family/bevy_siren.ron`, and
`reset_histories` has **no caller** anywhere.

| | before | after |
|---|---|---|
| picks returning within 3 (avoid 3, 4 keys) | **16 of 20** | **0 of 20**, every gap exactly 4 |
| `capacity 1, avoid 3` sequence | `adbcbcabcacabcabcbabcbcb` | `adbcadbcadbcadbcadbcadbc` |
| `VariantPicker::default()` | remembers nothing | remembers one pick |
| tests | 133 / 141 / 144 + 6 | **134 / 141 / 144 + 6** |

**Two stale list entries corrected.** Iteration 50's item #2 (`Duck::new`'s zero hold/fade) was **already shipped** —
`problems()` reports it at `src/mix.rs:217` and the test exists at `tests/mix.rs:958`. I wrote a duplicate, found the
original, deleted mine. Item #1 (case 31) remains blocked.

**Perf, and what it does not prove.** 8 and 64 voices moved +2.4 % and +0.6 %; 256 read +56 %, but `perfcheck` calls
**no function this iteration changed** (no picker, no cue table — `approach`/`db_to_gain`/`chain_db` only), and one
binary gave `0.673 1.384 1.337 1.221 1.291 1.193` across six runs. A 2× spread on one binary is the host (load 5.1);
neither median is distinguishable. No speedup or regression is claimed.

**Not built.** A `CueProblem` for an `avoid` window that cannot be honoured. `pick_from`'s own comment promises
*"`problems()` is where the table should be named"*, and a two-sound cue at `avoid: 5` probes to `problems: []` — but a
seventh variant needs the open enum decision. `examples/render`'s `footstep.stone` (**avoid 4, 2 sounds**) is exactly
that case, and its stream alternates correctly, which is the best two keys allow.

**Evidence.** `runs/2026-11-03-51/`: `research.md` (S1 `AvoidRepeat`'s doc, S2 the picker's window, S3 the unkept
`problems()` promise, S4 Böttcher on repetition, S5 Gregory on per-play state), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{plots,spectrograms,after}`.

**Three mistakes in this iteration's own work.** (1) A duplicate of an existing test — found by grepping before trusting
the stale list. (2) My first regression test asserted through `pick_from` alone and still failed after the source fix,
because growth lived in `SirenChoices::pick`; the right answer was to grow the window inside `pick_from`, so the rule
travels with the call. (3) Two `cargo test <name> <name>` calls failed on argument parsing — the suite takes one filter.

**Commit.** `46c18c1` — *siren: a cue's avoid window is the number it asks for*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`AvoidRepeat`'s window that cannot be honoured is unreported** — `pick_from`'s comment promises `problems()` names
   it; probed `problems: []` for two sounds at `avoid: 5`. Same open enum decision.
3. **`reset_histories` has no caller** — the Cauldron's table reload should reset pickers; a consumer item for
   `bevy_grimoire`, its own commit.
4. **`SirenSettings::master_db` defaults to `0.0`** — no trim, so a project from `default()` has no headroom. The guide
   says what to turn; whether the default should be lower is a decision file.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 50 — 2026-11-03 — the master is the headroom trim, and two demonstration files stop clipping

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`tests/mix.rs::the_master_is_inside_the_worst_peak`** — one voice at unity reads **`0`, `−6`, `−20` dBFS** at masters
   of `0`, `−6`, `−20`. The claim `needs-decision/above-unity-gain.md` rests on, measured for the first time.
2. **`examples/mixer`'s buses trimmed**: `Effects` `0.0 → −7.0`, `Music` `−2.0 → −7.1` — `2 × 10^(−7/20) = −0.98 dBFS`,
   `−1.08` at −7.1. The panel's peak clause **disappears**, because nothing reaches 0 dBFS.
3. **`examples/render`'s `mix.wav`** written at `settings.master_db` instead of `sum_db(&gains)`.
4. **The guide** states *"`master_db` is the trim, and it is inside the number"*, and why `problems()` stays silent.

**Rationale.** Iteration 49 left the shipped table's clipping as *"an art decision"*. It is not: `above-unity-gain.md`'s
recommendation **C — do nothing** rests on *"`SirenSettings::master_db` is the tool for it"*, and nothing had tested that
sentence, written it in the guide, or applied it to the crate's own two demonstration files. `mix.wav` was **`4.2×` full
scale** — `sum_db` of ten cues (+18.58 dB) applied to one 0.5-amplitude tone — and read `+0.2 dBFS / −4.3 LUFS`.

**Not built: case 31** — still blocked on `needs-decision/bad-gain-problem.md`. The loudness question was **not** blocked:
its decision already recommends C, so a report was never the answer.

**Evidence.** `runs/2026-11-03-50/`: `research.md` (S1 the decision's *"the master is the tool"*, S2 `master_db`'s doc,
S3 the master already inside `chain_db`, S4 the render harness's `sum_db` write, S5 the −14 LUFS target),
`plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT), `evidence/{panel,after}`.

| | before | after |
|---|---|---|
| `examples/mixer`'s `Effects` | `+0.0 dB`, `peak +6.0 dBFS / 2` | **`−7.0 dB`**, no peak clause |
| `examples/mixer`'s `Music` | `−2.0 dB`, `peak +4.0 dBFS / 2` | **`−7.1 dB`**, no peak clause |
| `render`'s `mix.wav` true peak | **`+0.2 dBFS`** (4.2× full scale) | **`−6.1 dBFS`** |
| `render`'s `mix.wav` integrated | `−4.3 LUFS` | **`−18.6 LUFS`** |
| `worst_peak_db` vs `master_db` | untested | **1:1**, at `0`, `−6`, `−20` dB |
| tests | 128 / 136 / 139 + 6 | **129 / 137 / 140 + 6** |
| perf | — | **0.500 / 0.501 µs** at 256 voices, load 4.6 — the steadiest run this loop has recorded |

**Three mistakes in this iteration's own work, all caught by reading captures or numbers.** (1) `Music` was first set to
`−5.1`, which is `+0.9 dBFS` — the capture said so and my arithmetic had not. (2) `Effects`'s line carries an
`if overbudget` conditional, so my `replace` missed and the row still read `+0.0 dB`. (3) My first fix for `mix.wav` was a
`master_trim` helper clamping to `−3.0` — a magic constant doing an unclear job; replaced by writing the file at the field
the decision file names.

**Commit.** `0b5d895` — *examples: the master is the headroom trim, and two demonstration files stop clipping*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`Duck::new`'s zero hold/fade defaults** — reported since iteration 46, but the constructor still hands back an inert
   rule where `by_db` gets a real `-6.0`. Changing it is a behaviour change and needs a decision file.
3. **`SirenSettings::master_db` defaults to `0.0`** — no trim at all, so a project starting from `default()` has no
   headroom. The guide now says what to turn; whether the default should be lower is a decision file, not an iteration.
4. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
5. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.

## Iteration 49 — 2026-11-03 — a bus's worst-case peak is readable, and the harness stops calling clipping the budget's job

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`SirenMix::worst_peak_db(bus)`** — a bus's worst-case peak this frame in dBFS: `n` live voices at full scale, plus
   the bus's gain, via `model::sum_db`. An **upper bound** (the crate never reads a sample's amplitude), `SILENT_DB` for
   a bus with no voices. Additive — a new method, no field or variant.
2. **`examples/render`'s line** said *"307 samples would clip (**the budget's job, not the gain's**)"* — false, and now
   corrected to name the sample-cost budget and the new method.
3. **The panel draws it** in the `Music` block's tail: `peak Effects +6.0 dBFS / 2  Music +4.0 dBFS / 2`.

**Rationale.** Four iterations of `problems()` wording, so this one went back to **sound output**: the render harness,
its spectrograms and its loudness numbers — a gate the skill names that had not been read since iteration 42. `ebur128`
on `mix.wav` integrates at **−4.3 LUFS with a true peak of +0.2 dBFS**, and `voices-8.wav` has **307 samples at the
rail** measured from the WAV itself, against a comment that called it the budget's job. `max_voices` is *"fewer than a
mixer's worth of sample cost"* — a sample cost, never headroom — and R6 has always said *"several sounds sum in linear"*.
Nothing in the crate let a game ask, so even the harness's author assumed the budget covered it.

**Two things checked and found correct first**, recorded so the next iteration does not repeat them: `bus-Music:
recovered by 1.97 s` against a 0.85 s hold+fade is right — `update_mix` eases toward the duck's already-shaped envelope
with `MIX_TAU_S`, and `cues.yaml` case 6's own `green_when` says *"hold + fade + **50 ms**"*; and `ebur128` returning
nothing for `voices-64`/`voices-256` is all-rail input, not a measurement bug.

**Not built: case 31** — still blocked on `needs-decision/bad-gain-problem.md`.

**Evidence.** `runs/2026-11-03-49/`: `research.md` (S1 R6's linear sum, S2 `max_voices` as a sample cost, S3 `sum_db`
already existed, S4 `SirenMix` held both inputs, S5 a new method is additive), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{panel,after}`.

| | before | after |
|---|---|---|
| 307 samples past full scale in `voices-8.wav` | *"the budget's job, not the gain's"* — **false** | *"the budget is a sample cost, not headroom"* |
| a game asking "can my table clip" | no API | **`SirenMix::worst_peak_db(bus)`** |
| the shipped table, two voices on `Effects` | invisible | **`+6.0 dBFS / 2`** |
| the shipped table, two voices on `Music` | invisible | **`+4.0 dBFS / 2`** |
| the `Problems` block | `none` | **`none`** — the table is not wrong, and a peak was never its question |
| tests | 127 / 134 / 138 + 5 | **128 / 136 / 139 + 6 doctests** |
| perf | — | 1.157 / 1.241 µs at 256 voices, load **8.2** — the same host spread since iteration 41 |

**Two defects in this iteration's own work, both caught by looking.** (1) The peak first went into the **`Problems`**
block, which then said `none` *and* `bus \`Effects\` can reach +6.0 dBFS` at once — one block answering two questions,
one of them about the frame rather than the table; moved to the `Music` block. (2) That move left the old line in place
twice because `cargo fmt` had reflowed the block and both `replace`s silently missed — caught by **re-reading the
capture** and seeing the sentence still there.

**Commit.** `4882723` — *mix: a bus's worst-case peak is readable, and clipping is not "the budget's job"*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`Duck::new`'s zero hold/fade defaults** — reported since iteration 46, but the constructor still hands back an inert
   rule where `by_db` gets a real `-6.0`. Changing it is a behaviour change and needs a decision file.
3. **The shipped table clips**: `Effects` reaches **+6.0 dBFS** and `Music` **+4.0 dBFS** with two voices. The panel now
   says so; whether `examples/mixer`'s buses should be trimmed a few dB is an art decision, not a bug.
4. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
5. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.

## Iteration 48 — 2026-11-03 — a bus cap above the table's budget is reported

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** A **twelfth sentence** in `SirenSettings::problems`:
`bus \`Effects\` asks for 16 voices but the table's max_voices is 4, so it mixes 4`. Plus a twelfth bullet in the doc's
list, an eleventh line in the guide, the indexed assertion in `problems_names_every_mistake`, and `SIREN_MIX=overbudget`
— its own capture table, see below.

**Rationale.** `bus_voice_limit` is `min(cap, total)`, so a bus asking for more voices than the table allows silently
gets the table's. **Neither document said so**: `BusLevel::max_voices`'s doc names the `0` sentinel and not the clamp,
and the guide's *"a bus's own `max_voices` caps it further"* is true without warning that a larger number does nothing.
Backlog #5, verified in iteration 47 and built here.

**Why no decision file.** `problems()` returns `Vec<String>` — additive. Iterations 46 and 47 relied on the same fact.

**A capture decision made by looking.** `SIREN_MIX=broken` already carries three faults and the block caps at **three
sentences**, so a fourth would be the invisible `and 1 more` — a picture that cannot show a sentence does not prove it.
`SIREN_MIX=overbudget` gives this fault **its own table** (`max_voices: 4`, `Effects: 16`), and the block is that one
sentence. Every bus row under it reads `2 / 4`, agreeing with the sentence.

**Not built: case 31** — still blocked on `needs-decision/bad-gain-problem.md`.

**Evidence.** `runs/2026-11-03-48/`: `research.md` (S1 the `min`, S2 the doc's sentinel-only wording, S3 the guide's
"caps it further", S4 iterations 46/47's precedent, S5 the decision README's scope), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{panel,after}`, `docs/media/mixer-overbudget.png`.

| | before | after |
|---|---|---|
| `Effects: 16` under `max_voices: 4` | mixes **4**, report silent | mixes 4, **named** |
| the bus rows in the same capture | `Effects 2 / 12` | `Effects 2 / **4**` |
| a bus at `0` (the sentinel) | silent | **still silent**, asserted |
| a cap **equal** to the table's | silent | **still silent**, asserted |
| a bus cap above the table's, in `problems_names_every_mistake` | not indexed | a dedicated table (the big one's `max_voices: 0` hid it) |
| sentences `problems()` can emit | 11 | **12** |
| tests | 126 / 133 / 136 + 5 | **127 / 134 / 137 + 5** |
| perf | — | 0.643 / 1.265 µs at 256 voices, load **9.3** — the same host spread every run since 41 shows |

**One mistake in this iteration's own work**: I appended a second near-identical clamp test before noticing iteration
47's still in the file; the older one was deleted and the survivor rewritten to assert the sentence rather than the
silence it used to pin.

**Commit.** `1486c4b` — *mix: a bus cap above the table's budget is reported*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`Duck::new`'s zero hold/fade defaults** — reported since iteration 46, but the constructor still hands back an inert
   rule where `by_db` gets a real `-6.0`. Changing it is a behaviour change and needs a decision file.
3. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
4. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
5. **A cue's `max_instances` larger than its bus's cap** is the bus's — 8 requests on a bus of 2 give **two live voices**,
   silently. Pinned this iteration by `a_cue_polyphony_above_its_bus_is_the_buss_cap_and_unreported`, which also records
   why it is **not** a fault: the bus steals rather than refusing, so the table still plays. The sibling bus/table clamp
   *is* reported because nothing steals for it — worth keeping that distinction in mind before reporting this one.

## Iteration 47 — 2026-11-03 — a duck that starts on the bus it lowers is reported

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** An **eleventh sentence** in `SirenSettings::problems`:
`the duck on \`Effects\` starts on the bus it lowers, so every voice on it quietens itself`. Plus an eleventh bullet in
the doc's list, the assertion in `problems_names_every_mistake`, `SIREN_MIX=broken`'s ducks chosen so the block's
three-sentence cap shows both new sentences, and two guards kept from the probing.

**Rationale.** Iteration 45's backlog listed a **self-duck** as an open question and iteration 46's first probe of it
produced a different finding, so it had never been settled. Measured this iteration:
`Duck::new("Effects","Effects")` with a hold lowers the bus **that fired it** — `-6.0 dB` on the frame its own voice
starts, and `bus_db` that much lower. `fire_all` matches on `source`, `duck_db` matches on `target`; for a self-duck one
bus is both, so a footstep pulls down every other effect for the rule's whole envelope, and itself with them.

**Why no decision file — though I wrote one first.** `needs-decision/README.md`: *"only the public API, the message
types and the definitions-file schema come here."* `problems()` returns **`Vec<String>`**, additive, no `match` breaks.
I wrote `self-duck.md`, read the README, and deleted it; iteration 46 relied on the same fact. The sentence states the
**mechanism** rather than forbidding the rule: a self-sidechain is a real device, so a project that meant one reads a true
line and moves on.

**Not built: case 31** — still blocked on `needs-decision/bad-gain-problem.md`.

**Evidence.** `runs/2026-11-03-47/`: `research.md` (S1 `source`/`target` are different sentences, S2 `fire_all` +
`duck_db`, S3 the guide reasons about two rules never a self-rule, S4 the decision README's scope, S5 Gregory §14.6.3),
`plan.md`, `perf.md`, `eval.md` (six probes, five already correct), `review.md` (three passes, GREAT),
`evidence/{panel,after}`.

| | before | after |
|---|---|---|
| a self-duck's depth on its own voice's frame | −6.0 dB, **unreported** | unchanged, **named** |
| the bus it mixes | 6 dB lower | unchanged |
| sentences `problems()` can emit | 10 | **11** |
| the log on that frame | silent about it | `sword.hit ducked Effects by -6.0 dB` |
| the sentence's width | 118 chars, **wrapped**, pushed the log to `and 1 more` | **78 chars**, one line |
| tests | 124 / 131 / 134 + 5 | **126 / 133 / 136 + 5** |
| perf | — | 0.580 / 1.353 µs at 256 voices, load **10.9** — the same host spread every run since 41 shows |

**Six probes, five of them already correct** (recorded so the next iteration does not repeat them): `Cue::new` agrees
with the serde defaults field for field; `backend_spatial_gain` is **identical to rodio's own `spatial.rs`** arithmetic,
confirmed from the crate source; `stop_requests` against a layer's voice is handled by `reconcile`; a loop at
`max_instances: 1` asked 60 times keeps **one** voice (new guard, kept).

**Three mistakes in this iteration's own work.** (1) The first sentence was 118 characters and **wrapped onto a second
line** — caught by reading the capture, shortened to 78. (2) `SIREN_MIX=broken` still carried iteration 45's ducks because
`cargo fmt` had reflowed the block and my replacement silently did not match — the capture showed the *old* sentences and
I nearly read it as "the new one does not fire". (3) `let mut app = app(...)` shadowed the helper, and one assertion used
`None` where `ducked` holds every bus at `0.0`.

**Commit.** `1145170` — *mix: a duck that starts on the bus it lowers is reported*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`Duck::new`'s zero hold/fade defaults** — reported since iteration 46, but the constructor still hands back an inert
   rule where `by_db` gets a real `-6.0`. Changing it is a behaviour change and needs a decision file.
3. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
4. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
5. `SirenSettings::problems` has no sentence for a **bus with a `max_voices` above the table's** — the table's budget
   silently wins, and nothing says so.

## Iteration 46 — 2026-11-03 — a duck with neither a hold nor a fade is reported

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.** A **tenth sentence** in `SirenSettings::problems`:
`the duck on \`Music\` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s`. Plus a
tenth bullet in the doc's list, the assertion in `problems_names_every_mistake`, and the shape in `SIREN_MIX=broken` so
the panel draws it.

**Rationale.** `Duck::new(a, b)` leaves **`hold_s: 0.0` and `fade_s: 0.0`**, and `active_duck_db` returns `0.0` past the
hold when `fade <= 0.0` — so the shortest rule a project can write **fires, is recorded, and is inaudible on the frame
it fires**. Measured, not assumed: the new test reads `Some(0.0)` from the bare rule and `Some(-8.0)` from the same rule
with `.hold_s(0.25).fade_s(0.6)`. The crate's own `default_ducks()` chains both, which is why its pair works and why
nothing had said otherwise.

**Why no decision file.** `SirenSettings::problems` returns `Vec<String>`, so a sentence is **additive** — no consumer's
`match` breaks. (A `CueProblem` or `Refusal` variant would need one; this does not.) Iteration 36 relied on the same
fact. The **behaviour is unchanged**: a project relying on an inert rule still gets one, it is simply named.

**Not built: case 31** — still blocked on `needs-decision/bad-gain-problem.md`.

**Evidence.** `runs/2026-11-03-46/`: `research.md` (S1 `Duck::new`'s zero defaults, S2 `active_duck_db`'s `fade <= 0`
branch, S2b `default_ducks()` chaining both, S3 the guide's own sentence, S4 `hold_s` having no serde default function),
`plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT), `evidence/{panel,after}`, `docs/media/mixer-broken.png`.

| | before | after |
|---|---|---|
| a rule from `Duck::new(a,b).by_db(-8.0)` | fires, logs, ducks **0.0 dB**, report silent | unchanged behaviour, **named** |
| the same rule with `.hold_s(0.25).fade_s(0.6)` | `-8.0 dB` | `-8.0 dB` |
| sentences the report can emit | 9 | **10** |
| sentences the test asserts | 9 | **10** |
| the broken-table capture | 3-line block, 9 px | same, and the tenth among them |
| tests | 123 / 130 / 133 + 5 | **124 / 131 / 134 + 5** |
| perf | — | 0.406 / 1.313 µs at 256 voices, load **7.4** — the same host spread every run since 41 shows |

**Three mistakes in this iteration's own work, all caught before shipping.** (1) The first test asserted `None` where
`SirenMix::ducked` holds every bus at `0.0`. (2) `let mut app = app(...)` shadowed the helper. (3) The first big fault
table had bare `Duck::new` rules, so the **new** sentence fired from a rule whose real fault was a missing bus and the
assertion would have passed for the wrong reason — caught by removing the accidental triggers and seeing nine
sentences, not ten. Every rule in that table now carries `hold_s`, with a dedicated row for the tenth.

**Commit.** `a327457` — *mix: a duck with neither a hold nor a fade is reported*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. **`Duck::new`'s zero hold/fade defaults** — the report now names it, but the constructor still hands back an inert
   rule where `by_db` gets a real default (`-6.0`). Changing it is a behaviour change and needs a decision file.
3. A sentence for a **duck whose `source` and `target` are the same bus** — measured this iteration and legal; whether
   it is a fault is a question, not a bug.
4. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
5. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.

## Iteration 45 — 2026-11-03 — the mix table's report is held to the standard its sibling meets

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`SirenSettings::problems`'s doc lists all nine checks** — it named **six** of the nine, and **four of the nine
   sentences had no test at all** (measured by fixed-string search over `tests/`): `master_db is not a number`, `a bus
   has no name`, a bus's gain that is not a number, and a duck's hold or fade.
2. **`tests/mix.rs::problems_names_every_mistake`** — one table holding every fault, each sentence asserted, each shown
   to be its own fault, the `NaN` half of the hold/fade covered, and a clean table asserting nothing.
3. **The panel's `Problems` block draws both reports.** It read `table.problems_in(&settings)` alone, so a mix table with
   a NaN master drew `Problems: none` **while every bus read −80 dB**.
4. **`SIREN_MIX=broken`** produces the mix table's faults so the block can be seen drawing them.
5. **The cue list is bounded at eight rows** with `and N more`, and the log's count subtracts that row.

**Rationale.** `MusicScore::problems` has had a nine-sentence test since iteration 34, written after *its* doc named four
of nine. The mix table's report — the one an author trusts instead of the sound — had never been held to it, and the
panel drew only half of it. Beats the runner-up (`Music` header live-layer count) because that is decoration while this
is a report that says `none` about a table it cannot run.

**Not built: case 31** — still **blocked** on `needs-decision/bad-gain-problem.md` (a public enum; its own
recommendation defers the variant to the next minor, and the crate is 0.1.0).

**Evidence.** `runs/2026-11-03-45/`: `research.md` (S1 the score's test, S2 the doc's six-of-nine, S3 R6, S4 the existing
four-sentence test), `plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT),
`evidence/{panel,after}`.

| | before | after |
|---|---|---|
| a mix table with a NaN master | `Problems: **none**`, every bus −80 dB | `master_db is not a number` |
| sentences reached by a test | **5 of 9** | **9 of 9** |
| the doc's list | six of nine | nine of nine |
| 12 cue rows (`SIREN_FAULTS=many`) | log **off the bottom**, heading not on screen | `and 6 more`, log visible |
| margins: clean / broken / 14 cues | 9 / **0** / **0** px | **9 / 10 / 10 px** |
| tests | 122 / 129 / 132 + 5 | **123 / 130 / 133 + 5** |
| perf | — | unchanged; 256 voices read 0.537 at HEAD vs 0.455 here, alternating on one machine |

**Two defects in this iteration's own work, found by looking**: the broken-mix capture clipped at **0 px** (the block
cost two lines the log did not give up), and `SIREN_FAULTS=many` still clipped at 0 px because the **cue list** — twelve
rows — pushed the log off the screen entirely. Both fixed; three tables now measure 9–10 px.

**Commit.** `aa46bc7` — *mix: the report is held to the standard its sibling meets, and the panel draws all of it*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md`.
2. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
3. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
4. `SirenSettings::problems` has no sentence for a **duck whose `source` and `target` are the same bus** — accepted today;
   worth deciding whether it is a fault.
5. The panel's `Problems` block prints the mix table's sentences **before** the cue table's; a reader may expect the
   cue list's faults first, since the cue list is what sits above it.

## Iteration 44 — 2026-11-03 — a music layer waiting for a voice says so

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`MusicState::blocked(layer)`** — the **virtual voice** state. A score may name more layers than the budget has room
   for; when a layer's `Play` is refused the plugin records the live count and retries when it falls below. That field
   was private, so the `Music` block drew `-80.0 dB` for a layer **waiting for a voice** and `-80.0 dB` for one authored
   silent — one string, two answers to the question a panel exists for. The accessor is additive: no variant, field or
   exhaustive `match`.
2. **The panel appends `waiting`** to a layer's gain when `blocked` is `Some`.
3. **`SIREN_TIGHT=1`** gives the `Music` bus one voice so the state occurs in a capture; the cap is on the bus, not the
   table.
4. **`cues.yaml` gains cases 32 and 33** — iteration 43's parented despawn, and this iteration's virtual voice. 31 → 33.

**Rationale.** Gregory §14.5.8: *"a **virtual voice** is a sound the game believes is playing but that has no hardware
voice"*. The state existed and was correct; only the reading was missing, and a panel that cannot tell a refusal from a
silence is the silent-failure class this loop exists for. Beats the runner-up (writing case 32 as the item): the case is
bookkeeping until the state it describes is observable.

**Not built: case 31** (a non-finite cue `gain_db` in `problems()`) — still **blocked** on
`needs-decision/bad-gain-problem.md`. Its own recommendation is *"C now, A at the next minor"*; the crate is 0.1.0, so A
waits for the release that moves it. Nothing was built from a blocked decision.

**Evidence.** `runs/2026-11-03-44/`: `research.md` (S1 Gregory's virtual voice, S2 `blocked_at`'s own doc, S3
`MusicState::live`, S4 the suppression rule), `plan.md`, `perf.md`, `eval.md`, `review.md` (three passes, GREAT),
`evidence/{panel,after}`, and `docs/media/mixer-waiting.png`.

| | before | after |
|---|---|---|
| a layer the budget refused | `combat -80.0 dB` — same as a silent layer | `combat -80.0 dB **waiting**` |
| a layer the score lacks | — | `blocked` answers `None` |
| after one slot frees | — | `(calm: None, combat: Some(1))` |
| `SIREN_TIGHT` scope | table-wide, **starved the whole script** | the `Music` bus only |
| `cues.yaml` | 31 cases | **33** |
| tests | 122 / 129 / 132 + 5 | unchanged — the assertions went into an existing test |
| perf | 0.06 / 0.20–0.36 / 0.41–1.28 µs at load 5.7–6.3 | noise; `perfcheck` builds no `MusicScore` |

**A defect in this iteration's own capture, found by looking**: the first `SIREN_TIGHT` capped the **table** at one voice,
so the capture proved the waiting marker by refusing every other play in the script — six `refused` lines, every bus at
`0 / 1`. Scoping the cap to the `Music` bus isolated the point: `Effects 2 / 2`, the bus steal intact, and only the two
music lines that matter.

**Commit.** `a559edc` — *music: a layer the budget refused says it is waiting*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md` (a public enum; its own recommendation defers to the next minor).
2. The `Music` header could show `live() of names().count()`; the block shows each layer but not the total.
3. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
4. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).
5. `SirenSettings::problems` has no test that every sentence it can emit is reachable — a report nobody can trigger is a
   sentence nobody trusts.

## Iteration 43 — 2026-11-03 — a parented sound says it stopped when its owner is despawned

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`mix::log_despawned_voices`** — a system after `SirenSystems::Mix` that records each live voice's cue and whether it
   had a `ChildOf`, reads `RemovedComponents<Voice>`, and writes one `stopped \`<cue>\`: owner despawned` for a parented
   voice that vanished. **Both** backends, once.
2. **`examples/mixer`'s `parent` step** — spawns an owner, plays `engine.idle` on it with `Play::on`, despawns the owner
   two frames later.
3. **`start_pending` stops writing that line** — the first draft wrote it in both places and the mixer logged twice.

**Rationale.** `Play::on` parents the voice with `ChildOf`, so despawning the owner is the **ordinary** end of a sound on
a vehicle or a torch. Neither backend said so: the mixer freed the slot silently since iteration 7, and Bevy's path had
no entity left for `retire_voices` to see — its log's last word about such a sound stayed `played` while the mix counted
one fewer voice. `"ended"`, `"stop"`, `"cue stopped"` and `"stolen by …"` covered every other end. Found by probing the
parenting path, which `cues.yaml` does not cover.

**Not built: case 31** (a non-finite cue `gain_db` in `problems()`) — still **blocked** on
`needs-decision/bad-gain-problem.md`. Also re-checked `remove-completes.md`: it blocks **nothing** — `retire_voices` already
reads `AudioOutput::finished`, so only the dead type waits, and this iteration did not need it.

**Evidence.** `runs/2026-11-03-43/`: `research.md` (S1 `Play::on`'s doc, S2 every other end logged, S3 a reason is a
sentence, S4 Bevy 0.20.1 `RemovedComponents`, S5 R6), `plan.md`, `perf.md`, `eval.md` (with a suggested case 32),
`review.md` (three passes, GREAT), `evidence/{panel,after}`.

| | before | after |
|---|---|---|
| a parented sound's end | **no line**, both backends | `0.69s stopped \`engine.idle\`: owner despawned` |
| the mixer's own line | slot freed, silent | one line from the shared system |
| a game's unparented despawn | silent | **still silent** (`ChildOf` gates it) |
| a double line on the mixer | — | caught by test, removed |
| the panel | no `parent` step | `parent` in the default script; the log and `Effects 1 / 2 voices` agree |
| tests | 121 / 128 / 130 + 5 | **122 / 129 / 132 + 5** (two new) |
| perf | 0.06 / 0.34 / 0.75–1.21 µs | noise — load spiked to **8.66** during the run; `perfcheck` never despawns a parented voice |

**A defect in this iteration's own capture, found by looking**: the `owners.retain` that despawns the owner sat *after*
`stage`'s `since % every != 0` early return, so it only ran on step frames and the owner was never dropped before the
still — `parent-80.png` showed `played` with no stop line. Moved above the gate. No test would have caught it.

**A second thing, checked rather than assumed**: the still shows `0.69s stopped` above `0.67s played` of the same cue,
which reads as a stop before its play. Read bottom-up the timestamps descend monotonically and every stop follows its own
play; `review.md` records the sequence. The review's first explanation — "two interleaved cycles" — was wrong and is
corrected there.

**Commit.** `de35eea` — *mix: a parented sound says it stopped when its owner is despawned*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md` (a public enum).
2. **Case 32, proposed this iteration**: a parented sound's despawn, now covered by two tests but not by `cues.yaml`.
3. The `Music` header shows `calm`/`combat` targets but not how many layers are **live**.
4. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 42 — 2026-11-03 — the cue row says how many may sound, and a steal says which cap

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **The cue row gains a polyphony column.** It drew name, bus, mode, gain and `priority` — and `priority` decides who
   *survives* a full budget while `max_instances` decides whether the **next play may start at all**. `x2`, `x1`, and
   `all` for the documented `max_instances: 0` (which `problems()` already reports as `UnlimitedInstances`).
2. **A new `poly` step** asks `footstep.grass` three times in one frame against `x2`: two voices live, one steal, and
   `Effects 2 / 2 voices` — the row's number, proved. It runs **last** in the default script, so the still's log keeps
   the stinger, the dialogue duck and the spatial play above it.
3. **A steal now names the cap.** `Entry::Stopped`'s reason was `stolen by footstep.grass` for **three different budgets**
   — the cue's `max_instances`, its bus's `max_voices`, the table's `max_voices` — so a step that took three voices
   logged two identical lines and neither said which number to raise. `room_for` now returns `(Entity, Cap)` and the
   sentence ends `(cue)`, `(bus)` or `(table)`.

**Rationale.** `cues.yaml` cases 7 and 8 were green in tests and **unreadable in the picture**: the panel never showed
the budget that produced the steal, and the steal named the cue rather than the cap. The second part is the same class
this loop has fixed at iterations 19, 31, 36, 40 and 41 — one string for two facts — found by making the first work.

**Not built: case 31** (a non-finite `gain_db` in `problems()`), which stays **red and blocked** on
`needs-decision/bad-gain-problem.md`. Checked again this iteration: every route — a `CueProblem`, `Refusal` or `Entry`
variant — breaks a public enum with no `#[non_exhaustive]` (`grep` finds none), which a patch release should not carry.

**Evidence.** `runs/2026-11-03-42/`: `research.md` (S1 the budget-on-plays reading, S2 the three caps, S3 Gregory
§14.5.8.4, S4 `UnlimitedInstances`, S5 the refusal that already names its cap), `plan.md`, `perf.md`, `eval.md`,
`review.md` (three passes, GREAT), `evidence/{panel,after}`.

| | before | after |
|---|---|---|
| the cue row | name, bus, mode, gain, priority | **+ `x2` / `x1` / `all`** |
| a cue-cap steal | `stolen by footstep.grass` | `stolen by footstep.grass **(cue)**` |
| a bus-cap steal | `stolen by sword.hit` | `stolen by sword.hit **(bus)**` |
| three requests at `x2` | — | 2 voices, **1** steal, `2 / 2` on Effects |
| `max_instances: 0` | not shown | `all`, with `UnlimitedInstances` in `Problems` |
| the default script | 6 steps | 7 — `poly` last, so the still keeps the log |
| the still's margin | 9 px | 9 px |
| tests | 120 / 127 / 129 + 5 | **121 / 128 / 130 + 5** (one new) |
| perf | 0.019–0.062 / 0.102–0.358 / 0.402–1.360 µs | unchanged — the spread is the machine (three runs, one binary) |

**A defect in this iteration's own review, found by counting**: the default run logged **two** `stolen by footstep.grass`
lines for a three-request step. The arithmetic says one *cue* steal; the second was the **bus** cap taking a different
footstep voice — the exact ambiguity this iteration removed. After the change the isolated run shows one `(cue)` line and
the default run's two lines say `(bus)`, which is true. A second defect, found by reading the still: `poly` placed second
hid the stinger, duck and spatial log, so it now runs last and the still is captured at frame 140.

**Commit.** `dc0a396` — *play: a steal names the cap that took the voice, and the row shows how many may sound*.

**Top 5 remaining**

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on
   `needs-decision/bad-gain-problem.md` (a public enum).
2. A `Stopped` line for a parent's despawn — **blocked** on `needs-decision/remove-completes.md`.
3. The `Music` header shows `calm`/`combat` targets but not how many layers are **live**.
4. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 41 — 2026-11-03 — the panel shows a placed sound, and a gain it can be checked against

**Baseline.** Green on every step, no open issues. Nothing red.

**Change.**
1. **`examples/mixer` had no spatial cue and no `.at(..)`** — grep found zero of either. So `cues.yaml` cases 13 and 14
   (a spatial play with no listener is refused; distance decides the gain, 0 dB at 4 m and −6.02 dB at 8 m) were tested
   and **never drawn**. The table gains `bang` (`spatial: true`, `Effects`, priority 30), a `spatial` step plays it at
   **8 m** — twice the default `near_m`, where the law is exactly a halving — and its cue row carries
   `inverse at 8 m -6.02 dB`, computed with `model::distance_gain`, the function a play uses.
2. **`Entry::line` wrote a played gain to one decimal.** One decimal cannot show a distance law: `-6.0206` is case 14's
   number and `-6.0` is also what a cue authored `gain_db: -6.0` produces, so the log could not tell a distance from a
   fader — and it disagreed with the panel's row, which prints two. Now `{:.2}`, with a test whose mutation reddens it.
3. Log lines 7 → **6**: the eighth cue row costs 21 px, and with seven the lowest ink sat at `y = 719` of 719 — 0 px.

**Rationale.** Cases 13 and 14 are rank-2 core behaviour and were invisible in the crate's own showcase; placement is the
crate's headline spatial feature (three models, a panner, a refusal class of its own). The precision fix is the same
class this loop has fixed at iterations 19, 31, 36 and 40 — **two numbers for one fact** — found by making the first work.

**Evidence.** `runs/2026-11-03-41/`: `research.md` (S1 the default attenuation and its −6 dB halving, S2 the three
formulae, S3 the one spatial predicate, S4 case 14's assertion), `plan.md`, `perf.md`, `eval.md`, `review.md` (three
passes, GREAT), `evidence/{panel,spectrograms,plots,after}`.

| | before | after |
|---|---|---|
| a placed cue in the panel | **none** | `bang … inverse at 8 m -6.02 dB` |
| the log's gain | `at -6.0 dB` | `at -6.02 dB` — the row's number |
| row vs log | absent vs lossy | **the same string** |
| gain at 4 / 8 / 16 m (`render.txt`) | — | 1.000 / 0.500 / 0.250 |
| `bang` vs an identical flat fixture | — | **exactly 6.00 dB apart** (−6.087 vs −12.087 dB peak) |
| `SpatiallyMute` recorded | tests only | `refusal-bang-no-SpatialListener-…txt` |
| log lines, clean table | 7, margin 11 px | **6, margin 9 px** |
| every test | 119 / 126 / 128 + 5 | **120 / 127 / 129 + 5** (one new) |
| perf | 0.025 / 0.148 / 0.490 µs | 0.056–0.064 / 0.276–0.377 / **0.416–1.392** — noise: `perfcheck` runs neither the example nor `line()`, and 256 voices swung 3.3× on one binary |

**Commit.** `4f34525` — *examples: the panel shows a placed sound, and a gain it can be checked against*.

**Top 5 remaining**

1. A `Stopped` line for a parent's despawn — **blocked** on `needs-decision/remove-completes.md` (7 items wait on it).
2. The cue list has no `max_instances` column; the table's per-cue budget is invisible.
3. The `Music` header shows `calm`/`combat` targets but not how many layers are **live**.
4. `docs/media/mixer.gif` captures at ~8 fps; a fade shorter than 120 ms falls between frames.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 40 — 2026-11-03 — the panel names the duck whose depth it shows, and plays a stinger

**Change.**
Two things in `examples/mixer`, one commit, because the second was **found by** the first.

1. **`Stinger` was never written.** `music.stinger` has been in the example's table since it was written and no step ever
   fired one — no import, no step — so the crate's second music feature, and the one duck whose cause is a mechanism
   (`Entry::Ducked { by: "stinger" }`), had never appeared on screen. A `stinger` step now sits in the default script
   before `refuse`, and the shipped still is captured on its frame.
2. **The bus row named the wrong duck.** It printed `SirenMix::ducked` — the **minimum** over every active rule — beside
   the **newest** `Ducked` entry's `by`. One duck hid it; a stinger over a dialogue line puts two on one bus and the row
   read `duck -8.0 dB by stinger` where −8.0 belonged to `dialogue.greeting`. Now the newest entry is named with **its
   own** `by_db` when the depths differ (`duck -8.0 dB (newest: stinger at -6.0)`), and plainly when one rule explains the
   number — including a rule that is **recovering**, which is not a second cause.

`src/` is untouched: no public type, no schema, no message.

**Rationale.** This loop has fixed "two numbers from different sources presented as one fact" at iterations 19, 31, 36
and 38; iteration 31 *introduced* this one by choosing the newest entry, and it stayed invisible because no bus ever had
two ducks at once. The stinger was the item that made it visible — and it was itself a feature a consumer could not see.

**Evidence.** `runs/2026-11-03-40/` — `research.md` (S1 `Stinger`'s −6 dB default, S2 `Ducked.by` naming a mechanism, S3
`ducked` as the minimum, S4 "everything a play decides is readable"), `plan.md`, `perf.md`, `eval.md` (case 13 red before,
green in all three shapes after), `review.md` (three passes, GREAT), `evidence/panel/`.

| | before | after |
|---|---|---|
| `music.stinger` ever played | never — `Stinger` never written | `0.89s stinger ducked Music by -6.0 dB` |
| the `Music` row, two ducks | `duck -8.0 dB **by stinger**` (wrong cause) | `duck -8.0 dB **(newest: stinger at -6.0)**` |
| the `Effects` row, one duck | `duck -4.0 dB by dialogue.greeting` | unchanged |
| one duck, recovering (fresh eyes) | invented a second cause | `duck -2.2 dB by dialogue.greeting` |
| row wrap at 1280 | n/a | none — 4 evenly spaced bands, right edge x=817 |
| the still's margin | 11 px | 11 px |
| the GIF's first frame | 0.66 s, before the stinger | **0.94 s, on it** |

Full gate green: build, 119+5, 126+5 (serde), 128+5 (mixer), 5 doctests, clippy `-D warnings`, fmt, rustdoc, examples,
`readme::`. Perf unchanged in the crate (example only): 0.025 / 0.148 / 0.490 µs per frame at 8 / 64 / 256, load 4.0.
`check_media.py`: 0 problems. CI run **41602744629**: both jobs green on `big-s1` — `fmt + clippy + doc` 13 s,
`build (standalone) + test` 44 s. **A defect in this iteration's own review, found on the fresh-eyes frame**: the long form
invented a second duck for a rule that was merely recovering; fixed before shipping and re-reviewed.

**Commit.** `79d41b9` — *examples: the panel names the duck whose depth it shows, and plays a stinger*.

**Top 5 remaining**

1. A `Stopped` line for a parent's despawn — **blocked** on `needs-decision/remove-completes.md` (7 items wait on it).
2. The cue list has no `max_instances` column; the table's per-cue budget is invisible.
3. The `Music` header row shows `calm`/`combat` targets but not how many layers are **live**.
4. `docs/media/mixer.gif` captures at ~8 fps from a 30 fps window; a fade shorter than 120 ms still falls between frames.
5. No capture shows the panel with a **missing sound** refused (iteration 39 showed `unknown-cue` only).

## Iteration 39 — 2026-11-03 — the panel shows a refusal

**Change.**

- **`examples/mixer.rs` — a `refuse` step**, `Play::cue("no.such.cue")` → `Refusal::UnknownCue`, **in the default script**
  after `duck`. The still now opens its log with `refused \`no.such.cue\`: no cue of that name`.
- **The log's line count follows the `Problems` block**: seven when clean, **four** when it has faults — measured, the
  fault table costs three log lines and seven leaves a 1 px margin.
- The GIF is captured at native 1280; `docs/media/README.md` and a CHANGELOG line.

**Why.** Iteration 38's lead: the panel's claim is *"the log of what played and why"*, and the scripted run **never
refused anything** — `steal` steals, `duck` ducks, `layer` moves, every step plays. So the `Log` block had never drawn a
`Refused` line, the one message the skill calls the most valuable.

**Evidence.** `runs/2026-11-03-39/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{refusal-1280,zoom-refusal,zoom-log-95,faults-final,zoom-tail-faults}.png`.

**Measured.**

| row | before | after |
|---|---|---|
| a refusal in the log | **never** | `refused \`no.such.cue\`: no cue of that name`, at the top |
| the default script | `steal,layer,duck` | `steal,layer,duck,**refuse**` |
| fault table, 7 log lines | — | margin **1 px — clipped** |
| fault table, 5 | — | **1 px** |
| fault table, 4 | — | **11 px** |
| the clean still | 7 lines, 11 px | 7 lines, 11 px |
| the GIF | 960 wide (down-scaled) | **1280, native** |
| every test, both feature sets | — | unchanged (119 / 126 / 128 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.063 / 0.357 / 1.355 µs, load 4.6–4.2; no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **Three things I got wrong and recorded:** (1) I read a clipped
last line in a **crop** and called it the panel's — the window measures 11 px. (2) I computed the fault table's overflow as
−59 px from row heights while the pixel scan said 1 px; the scan was right, my arithmetic was the wrong model. (3) I assumed
the GIF's 960 down-scale caused the wrap; the fault table overflows at 1280 too. **And the GIF's first frame is at 88,
before the refusal at 90** — the still proves the line, the GIF proves the layout.

**Traps for the command.** (1) A showcase can be **complete and still not show its own subject**: three steps, ten
iterations, and none of them ever refused. Ask *"what does this panel claim, and which line of it has never been
rendered?"* (2) **A crop's edge is not the window's** — measure the capture, not the snippet. (3) Row-height arithmetic is
a **model**; when it disagrees with a pixel scan by 60 px, the model is wrong. (4) A **down-scale** changes what fits: a
960-wide GIF wraps lines the 1280 panel does not, so capture at native size. (5) `hs distill` GPU-blocked (thirty-eighth
iteration).

**Commit.** `f40ff6e`, pushed to `main`; CI run 41601941818 is **success** on `big-s1`. Gate: build, test (119 + 5),
serde (126 + 5), mixer (128 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. A gain above 0 dB — `needs-decision/above-unity-gain.md`, recommendation to **not** add it.

**The hunt, twenty-eighth pass.** The panel now shows all five kinds of entry: `played`, `stopped`, `ducked`, `refused`, and
the score. What it has **never** shown is the **stinger** — `MusicState` and `Play` both know one, the example's table has
`music.stinger`, and no step plays it. The next run should stage it and check that the duck it arms (`by "stinger"`) is
visible on the `Music` row, which is the one duck whose cause is not a cue.

**Baseline note.** Nothing was red; the suite had 119 tests. `cues.yaml` case 31 is red by design. **Disk 9.8 GB free** —
the tightest this loop has recorded; a release build would need a clean-up first.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 38 — 2026-11-03 — the header counts what the panel shows

**Change.**

- **`examples/mixer.rs` — the header names the split**: `{cues} cues, {layers} layers, {buses} buses, {voices} voices`, with
  `cues = table.cues.len().saturating_sub(layers)` and `layers` from `music.score`.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` and a CHANGELOG line.

**Why.** Iteration 37's lead was the two images' table difference; **looking** at the still shipped by iteration 36 found
something closer. The header said `9 cues` and the cue list beneath it showed **seven** — iteration 36 moved the score's two
layers into a `Music` block and left the header counting `table.cues.len()`. The panel already had both numbers; it printed
one.

**Evidence.** `runs/2026-11-03-38/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{header-1280,header-faults,zoom-header-faults}.png`.

**Measured.**

| row | before | after |
|---|---|---|
| the clean table's header | **`9 cues`** over 7 rows | **`7 cues, 2 layers`** over 7 rows and a 2-layer block |
| the fault table's header | `9 cues` — by coincidence | `9 cues, 2 layers` over **9** rows |
| a table with fewer cues than layers | would underflow | `(0, 2)`, no panic (probed) |
| every test, both feature sets | — | unchanged (119 / 126 / 128 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| the still's margin | 11 px | **11 px** — the header gained words, not a line |
| frame cost, 8 / 64 / 256 | — | 0.018 / 0.114 / 0.424 µs, load 5.7–4.5; **no `src/` line moved** |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The **fault-table capture is what proves the number is computed**:
a hard-coded `7` would pass the clean still and fail `9 cues` over nine fault rows.

**Traps for the command.** (1) A panel with **two containers of the same noun** (cues in a list, layers in a block) needs the
header to name the **split** — one number cannot describe both. (2) When a change moves rows from one block to another, the
**counts above them** are part of the change, and nothing tests a header. (3) `saturating_sub` where the subtrahend is
caller data: a layer may name a cue the table lacks, and that case is reported elsewhere, not a panic. (4) A second capture
with a **different correct answer** is how to prove a number is computed rather than typed. (5) `hs distill` GPU-blocked
(thirty-seventh iteration).

**Commit.** `2a94b2a` (+ `012c71a`), pushed to `main`. CI run **41601066231 is success** on `big-s1`, covering `HEAD`.
The code commit's own run (`41601006025`) shows **cancelled** with no jobs — the workflow's concurrency group cancelled it
when the docs commit landed 29 s later; that is normal for a push-per-branch group, not a failure. Gate: build, test
(119 + 5), serde (126 + 5), mixer (128 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.
The Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. A gain above 0 dB — `needs-decision/above-unity-gain.md`, recommendation to **not** add it.

**The hunt, twenty-seventh pass.** The panel's five blocks now each have a header number that matches them. What the panel
has **never** shown is a **refusal on screen**: the `Log` block draws `Entry::line()` for every entry, so a `refused` line
appears only if one happens to be in the last seven — and the scripted run never refuses anything. The panel's own claim is
*"the log of what played and why"*; a `Refused` is the "why" that matters most, and it is never demonstrated. The next run
should stage a refusal in the capture (a cue not in the table) and check that the log tail shows it.

**Baseline note.** Nothing was red; the suite had 119 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 37 — 2026-11-03 — the GIF samples a fade by the second, not by the update

**Change.**

- **`examples/mixer.rs` — `SIREN_SHOT_EVERY` is hundredths of a second** (4 = 40 ms), and `capture` keeps a frame when that
  much real time has elapsed. `Capture` gains `kept_at_s: f64`.
- The knob's doc says why: a fade is a duration, and an update is not a time.
- `docs/media/mixer.gif` regenerated (46 frames at 10 fps); `docs/media/README.md` and a CHANGELOG line.

**Why.** Iteration 36's block shows `-> target` only while a layer moves — a fade of `MIX_TAU_S` = **50 ms**, about **6
updates** at the example's ~8.3 ms. The GIF sampled every **12 updates**: **0.5 samples per fade**. The feature appeared by
luck of the phase, and the shipped GIF **was** right by accident (20 frames, the music line changing only across 12–16).

**Evidence.** `runs/2026-11-03-37/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{fade-frame-31,zoom-fade-31}.png`.

**Measured.**

| row | before | after |
|---|---|---|
| `SIREN_SHOT_EVERY` means | updates | **hundredths of a second** |
| samples per 50 ms fade | **0.5** | **1.25** at 40 ms |
| frames carrying the arrow | 1, by luck | **11 consecutive** (`f_0028`–`f_0038`) |
| GIF frames | 20 at 6 fps | **46 at 10 fps** |
| the crop, read | — | `calm -76.2 -> -80.0   combat -6.9 -> -3.0` |
| every test, both feature sets | — | unchanged (119 / 126 / 128 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.059 / 0.322 / 1.219 µs, load 2.8–4.8; no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **A magic constant in my own first fix:** I converted updates to
milliseconds with a literal `8.333`; the crate's rule forbids it, and the knob is a duration instead, so the conversion
disappears. Two probes came back clean on the way: a move to the gain a layer already has is a no-op (`moving 0`), and a
2.4 s fade arrives in 161 frames against the 144 a 60 fps clock predicts.

**Traps for the command.** (1) **A frame count is not a time** — a capture knob measured in updates depends on the machine's
speed, and a GIF is a temporal artefact. (2) When the thing being captured is a **fade**, the sampling period must be shorter
than the fade or the capture is a coin flip; compute the ratio before trusting the result. (3) A screenshot directory is
**evidence**: a showcase that is right by luck is one refactor from being wrong, and a **per-frame ink scan** is how to tell
"the arrow rendered" from "the arrow happened to be in the one frame I looked at". (4) A conversion factor in a system is a
magic constant; change the **unit** instead. (5) `hs distill` GPU-blocked (thirty-sixth iteration).

**Commit.** `10491d1`, pushed to `main`; CI run 41600471552 is **success** on `big-s1`. Gate: build, test (119 + 5),
serde (126 + 5), mixer (128 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. A gain above 0 dB — `needs-decision/above-unity-gain.md`, recommendation to **not** add it.

**The hunt, twenty-sixth pass.** The capture class is now correct in **time**. What it has never checked is **content**: the
GIF's 46 frames and the still are captured from one scripted run, and nothing asserts that the still's `Problems / none` and
the GIF's `<- problem` rows describe the *same* table. The next run should diff the two images programmatically — the still
is the clean table, the GIF is `SIREN_FAULTS=1`, and a reader is told so only in prose.

**Baseline note.** Nothing was red; the suite had 119 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 36 — 2026-11-03 — the panel shows the music score

**Change.**

- **`examples/mixer.rs` — a `Music` block** between `Buses` and `Cues`: `MusicState::names()` with `layer_db`, and
  `-> target_db` while `moving()`. One line for the whole score.
- **The cue list filters out a cue the score owns**, so a layer is shown once, where it can show live gain;
  `music.stinger` stays (a one-shot on `Music`, not a layer).
- **Seven log lines** instead of eight, because the block costs 68 px against the 63 px the three cue rows freed.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` says why seven; a CHANGELOG line.

**Why.** `MusicState` exposes **seven** panel-facing accessors and the panel drew none — deferred **four times** for
height. This iteration made the layout decision instead of routing around it again.

**Evidence.** `runs/2026-11-03-36/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{music-block-1280,final-mid-fade,zoom-final-music}.png`.

**Measured.**

| state | before | after |
|---|---|---|
| the score at rest | absent — two ordinary cue rows | `calm -4.0 dB   combat -80.0 dB` |
| **mid-crossfade** | invisible | `calm -79.9 -> -80.0   combat -3.1 -> -3.0` |
| after the bar | invisible | `calm -80.0 dB   combat -3.0 dB` |
| cue rows | 9 (3 of them layers) | **7**, no layer among them |
| log lines | 8, margin 12 px | **7**, margin **11 px** |
| a row-per-layer block (my first version) | — | margin **1 px — clipped glyph** |
| every test, both feature sets | — | unchanged (119 / 126 / 128 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | median 1.069 µs at 256, load 4.2–5.3; no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **A defect I introduced and measured away:** a row per layer
cost more than the three cue rows it displaced — margin 12 px → **1 px**, a clipped glyph, iteration 32's defect returned.
Two measured fixes (one line, seven log lines) take it to 11 px.

**Traps for the command.** (1) A **heading is expensive** — 30 px + gap, about one and a half cue rows — so "remove three
rows, add a block" is not break-even even when the row count says it is. (2) A static row cannot show a **transition**: the
`-> target` arrow is the whole reason for a music block, and it only exists while `moving()` is non-zero. (3) The example
runs at **~8.3 ms/frame (≈120 fps)**, so a 1.5 s bar is ~180 frames; my first mid-fade captures at 112–175 were **before**
the bar. Capture by state, not by a frame number you guessed. (4) `MusicLayer::new` starts at `SILENT_DB` — `combat -80.0 dB`
is the default, not a bug. (5) `hs distill` GPU-blocked (thirty-fifth iteration).

**Commit.** `a1e3a02`, pushed to `main`; CI run 41599962692 is **success** on `big-s1`. Gate: build, test (119 + 5),
serde (126 + 5), mixer (128 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. A gain above 0 dB — `needs-decision/above-unity-gain.md`, recommendation to **not** add it.

**The hunt, twenty-fifth pass.** The panel now shows buses, the score, the cues that are not layers, the report and the log —
and the **score block has no `next_change_s`**, the one accessor whose doc says *"for a panel that says 'in 0.8 s'"*. It would
be a fifth block, and the column has **11 px** free. The honest next step is not another block: it is to check whether the
**guide's** sentence *"layers that change on the bar"* is now visible end to end in the GIF — regenerate at a smaller frame
step so the 1.5 s bar is inside 20 frames instead of 12.

**Baseline note.** Nothing was red; the suite had 119 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 35 — 2026-11-03 — the report's doc is the whole list, and a layer's bus is a `must`

**Change.**

- **`src/music.rs` — `MusicScore::problems`'s doc lists all nine checks it performs**, each a clause, with the note that the
  plugin enforces the last two and writes the same sentence to the log. It named **four**.
- **`MusicLayer::cue`'s doc says `must`, not `should`**, and what happens otherwise.
- `tests/music.rs::a_score_reports_every_fault_its_doc_lists` walks all nine; a CHANGELOG line; and
  `needs-decision/above-unity-gain.md` for a gain above 0 dB.

**Why.** Iteration 34's log named the sweep: *"grep everything `should` in `src/` and check each has a mechanism."* Five hits,
**two real**: the report's doc (four of nine names) and `MusicLayer::cue` (*"should"* where iteration 34 made it a
**refusal**). Three are comments describing history, true as written.

**Evidence.** `runs/2026-11-03-35/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/nine-checks.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| checks the report's doc names | **4** | **9** |
| `MusicLayer::cue`'s wording | *"should"* — while the plugin refuses | *"must"*, and what happens |
| the nine checks themselves | all present | unchanged — **no executable line moved** |
| an empty cue (absolute claim) | only refuses | unchanged |
| a budget of 2, six unstealable | plays 2, refuses 4, never 3 | unchanged |
| every test, both feature sets | 118 / 125 / 127 + 5 | 119 / 126 / 128 + 5 |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.123 / 0.452 / 1.235 µs, load 9.4–6.6; **nothing executable changed** |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A gate failure in my own doc, caught by
`RUSTDOCFLAGS="-D warnings"`: `[`SirenLog`](crate::SirenLog)` is a redundant explicit target. A probe that lied, caught
rather than shipped: my first sweep row read *"layer cue on another bus → 0 problems"* because the cue I used was **on
`Music`** — the label was wrong, not the code.

**Traps for the command.** (1) **A doc that lists a mechanism's checks must list them all** — it was written when the report
had four and grew to nine without the list following. (2) *"Should"* where the code does nothing and *"should"* where the code
**refuses** are the same failure in opposite directions; grep both. (3) An **absolute claim** in a doc is testable: "only
refuses" and "never a 33rd voice" both held, and both are now known rather than assumed. (4) `RUSTDOCFLAGS="-D warnings"` is
part of the baseline for a reason — it caught a redundant intra-doc link in a doc-only change. (5) `hs distill` GPU-blocked
(thirty-fourth iteration).

**Commit.** `6974c51`, pushed to `main`; CI run 41599158877 is **success** on `big-s1`. Gate: build, test (119 + 5),
serde (126 + 5), mixer (128 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block in the panel** — deferred a **fourth** time. The budget is measured (12 px free, ~134 needed) and the fix
   needs a layout decision: the cue list has nine rows and three are music cues the block would show better. **The next run
   should make that decision, not route around it again.**
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A gain above 0 dB — `needs-decision/above-unity-gain.md`, recommendation to **not** add it.

**The hunt, twenty-fourth pass.** The **`should`/absolute-claim** sweep is now done in `cue.rs`, `music.rs`, `mix.rs` and
`play.rs`: two real finds, both fixed. What has never been swept is **`docs/guide/`** against the code — the guide's
sentences are what a project reads before it writes a table, and iteration 32 found one (the log's eight lines) by accident.
The next run should diff each guide claim against the constant or accessor it names, starting with `buses.md` and `spatial.md`.

**Baseline note.** Nothing was red; the suite had 118 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 34 — 2026-11-03 — a layer is a stem of the `Music` bus

**Change.**

- **`src/music.rs` — `start_missing` refuses a layer whose cue's bus is not `Music`**, through iteration 16's one-notice
  machinery. `MusicScore::problems` gains the same sentence word for word, so the report and the log cannot disagree.
- **The `bad` list carries `(name, cue, sentence)`** rather than re-deriving the wording at the print, so both faults share
  one loop and the reason is the thing carried.
- Two tests (one failing on the base commit, one guard); a CHANGELOG line.

**Why.** `MusicLayer::cue`'s doc says the cue *"should name the `Music` bus"* — advice, and nothing checked. A layer naming a
loop on `Effects` **started there**: `MusicState` reported two layers live while one was mixed on the effects bus,
`MusicLayers` could not move it, and `problems()` said nothing. `play_stingers` already refuses exactly this; the layer path
had neither the check nor the line.

**Evidence.** `runs/2026-11-03-34/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/wrong-bus{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `problems()` | **nothing** | one sentence naming layer, cue and **bus** |
| live voices | `wrong.bus@Effects`, `track.ok@Music` | `track.ok@Music` |
| `MusicState::live()` | **2** | **1** |
| `SirenMix` voices | Effects **1**, Music 1 | Effects **0**, Music 1 |
| a score of two `Music` layers (guard) | starts | **unchanged** |
| every test, both feature sets | 116 / 123 / 125 + 5 | 118 / 125 / 127 + 5 |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | median 1.095 µs at 256, load 8.5–5.3; one bus comparison per **voiceless** layer |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A test defect caught in review: my first version built the good
layer with `layer_cue("track.ok")` — whose sound key is `"track.ok"` — against a sounds list of `["a", "b"]`, so **neither**
layer played and the assertion read `left: []`. The test's table was wrong; the code was right.

**Traps for the command.** (1) A doc that says **"should"** is an invariant nobody wrote: grep the `should`s and check each
has a mechanism or a report. (2) When one path already refuses a fault (the stinger), the sibling path that merely *starts*
is the bug — the quieter mechanism is the harder one to see. (3) A notice list that carries `(name, cue)` and re-derives
the sentence at the print cannot grow a second fault; carry the **sentence**. (4) A test's own table is code: a wrong sound
key made a correct fix look broken. (5) `hs distill` GPU-blocked (thirty-third iteration).

**Commit.** `d7b5bb3`, pushed to `main`; CI run 41598441440 is **success** on `big-s1`. Gate: build, test (118 + 5),
serde (125 + 5), mixer (127 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block in the panel** — `MusicState`'s seven accessors drawn by nothing; deferred **three times** for height
   (12 px free against ~134). It needs a layout decision, and every iteration that routes around it should say so.
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-third pass.** The **"should"** sweep is the new lead and it is cheap: `grep -rn 'should\|ought' src/` and check
each sentence has a mechanism. Two found so far this iteration (the layer's bus, now enforced, and `MusicLayer::cue`'s
remaining wording). The next run should finish that sweep across `cue.rs`, `mix.rs` and `play.rs` — a doc that says *"should"*
where the code says nothing is a defect that has shipped in silence.

**Baseline note.** Nothing was red; the suite had 116 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 33 — 2026-11-03 — a queued change follows the score's tempo

**Change.**

- **`src/music.rs` — `Pending` remembers which boundary it was quantised to (`When`) and the tempo it used**, and
  `move_layers` re-quantises a change whose tempo is stale, before the due filter: `at.max(now)` so a faster tempo cannot
  pull it into the past, and a `tempo` guard so an unchanged score does no work.
- Three tests (one failing on the base commit, two guards); a CHANGELOG line.

**Why.** Iteration 32's lead was a Music block, deferred for height. Probing the **music behaviour the block would have
shown** found something larger: `When::NextBar` promises *"on the next bar, quantised with `next_boundary`"*, and
`move_layers` computed that instant **once** from the tempo at request time. A `MusicScore` is a resource *"the Cauldron
edits"*, so a tempo change while a change waits is the ordinary path — and the crossfade landed **off the beat**, in both
directions.

**Evidence.** `runs/2026-11-03-33/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/tempo{,-before}.txt`.

**Measured** — walking the clock until `target_db` moves, so the number is when the crossfade **happens**:

| case | before | after |
|---|---|---|
| 160 bpm, unchanged | 1.44 s | 1.44 s (a bar is 1.5) |
| 80 bpm, unchanged | 2.944 s | 2.944 s (a bar is 3.0) |
| **160 → 80** before the bar | **1.44 s — mid-bar** | **2.944 s — on the bar** |
| **160 → 240** before the bar | **1.44 s — a bar late** | **0.944 s — on the bar** |
| `When::Now` (guard) | own frame | own frame |
| every test, both feature sets | 113 / 120 / 122 + 5 | 116 / 123 / 125 + 5 |
| every rendered cue | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | — | median 0.543 µs at 256, load 5.7–4.5; the loop runs over **queued changes** (0 or 1), not voices |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The **direction of the error was measured both ways** — a
slowdown made the change early, a speed-up made it late — because one direction alone leaves the sign error untested.

**Traps for the command.** (1) An instant **computed once from a resource** is stale the moment that resource changes; a
queue must keep the input it quantised against, not only the answer. (2) Iteration 14 fixed this for **layers** and missed
the **pending queue** in the same module — when fixing a reconcile, list every structure that cached the old value.
(3) A re-quantise needs a guard on *the input's change*, or it moves a queued item forward every frame; the guard test is
what stops that. (4) Measure *when the effect happens* (walk the clock to the observable), not what the accessor reports —
they agreed here, and that agreement is worth knowing. (5) `hs distill` GPU-blocked (thirty-second iteration).

**Commit.** `397b123`, pushed to `main`; CI run 41597804924 is **success** on `big-s1`. Gate: build, test (116 + 5),
serde (123 + 5), mixer (125 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block in the panel** — `MusicState`'s seven accessors drawn by nothing; needs a layout trade (12 px free
   against ~134). Deferred twice now, with the budget measured both times.
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-second pass.** The **music** module has now paid four times (14, 15, 19, 33), each found by probing a
**resource edited at runtime**. What has never been probed there is `MusicScore::problems` against a score that parses and
is wrong in a way the plugin then acts on: a layer naming a cue on a **non-`Music` bus** (`problems` says a layer *"should
name the Music bus"* — does the plugin refuse, or mix it on `Effects` while the score says music?). That is the next probe.

**Baseline note.** Nothing was red; the suite had 113 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 32 — 2026-11-03 — the panel's log shows the eight lines the crate documents

**Change.**

- **`examples/mixer.rs` — `SirenLog::recent(8)`**, the budget `CAPACITY`'s own doc names: *"a panel shows eight"*. The panel
  asked for ten in a fixed column that fits ~8.5, so the **last log line was clipped mid-glyph** in every capture.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` says why eight; a CHANGELOG line.

**Why.** Iteration 31's lead was a Music block. Probing the **height budget** first — iteration 30's lesson is that the
column is fixed — found the panel already over: **lowest ink at `y = 719` of 720**, one pixel of margin and a sliced glyph,
in the README's own still. The crate had documented the budget and the example used a different number.

**Evidence.** `runs/2026-11-03-32/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{eight-log-1280,readme-before}.png`.

**Measured.**

| row | before | after |
|---|---|---|
| lowest ink in the capture | **y = 719** of 720 — 1 px margin, **clipped glyph** | **y = 708** — 12 px margin |
| log lines shown | 10 | **8** — the documented budget |
| free height for the next item | — | **12 px**, against ~134 for a four-line Music block |
| every test, both feature sets | — | unchanged (113 / 120 / 122 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.065 / 0.353 / 1.056 µs, load 3.3–4.6; the panel formats two **fewer** lines |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **Scope was cut deliberately and recorded:** the **Music
block** — the example ships a two-layer score whose `Layer` step really crossfades `calm → combat` on the bar, and
`MusicState`'s five panel-facing accessors are drawn by nothing — needs ~134 px against 12 free. Shipping a clipped log
*and* a block that pushes it off screen would be two defects for one; the budget is in `eval.md` so the next run starts from
a measurement.

**Traps for the command.** (1) A **column that is full clips silently**: `recent(10)` returns ten entries whether the window
shows them, so only a **pixel scan of the capture** can see a sliced glyph. (2) Before adding a section, measure the free
height from the picture; my content arithmetic (~892 px in 720) was right in direction and wrong in magnitude. (3) A crate
that documents a budget (`"a panel shows eight"`) and an example that uses another number is a defect waiting for a capture.
(4) Regenerating an image does not prove it is right — iterations 29–31 each regenerated this one **with** the clip.
(5) `hs distill` GPU-blocked (thirty-first iteration).

**Commit.** `9967090`, pushed to `main`; CI run 41597034221 is **success** on `big-s1`. Gate: build, test (113 + 5),
serde (120 + 5), mixer (122 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A Music block** — `MusicState`'s `layer_db`/`target_db`/`names`/`moving`/`next_change_s` drawn by nothing, and the
   script performs a crossfade the picture cannot show. Needs a layout decision: ~134 px against 12 free, so the cue list
   or another block must give rows. **The budget is measured.**
2. A `Stopped` line for a parent's despawn — neither backend writes one.
3. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.

**The hunt, twenty-first pass.** The panel class has paid five times (26, 29, 30, 31, 32), and the last four were found by
reading the picture against the code. The **next** defect is already measured rather than guessed: the column has 12 px
free and the music has no view. The run after this one should **choose the rows to give up** — nine cue rows include three
music cues a Music block would show better, which is the natural trade — and prove the result by capture at 1280 **and**
1920, where the extra height means the trade may not be needed at all.

**Baseline note.** Nothing was red; the suite had 113 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 31 — 2026-11-03 — the bus row names the cue that ducked it

**Change.**

- **`examples/mixer.rs` — the bus row says `duck -8.0 dB by dialogue.greeting`** while the duck holds, from the newest
  `Entry::Ducked` naming that bus in the `SirenLog` the panel already holds. The clause is inside the `duck < -0.05` branch,
  so it disappears the frame the duck releases.
- **`tests/mix.rs::the_newest_duck_naming_a_bus_is_the_one_a_panel_shows`** — the rule pinned where the suite can see it:
  a non-duck entry between two ducks does not confuse it, a bus nothing ducked has no name, a newer duck on the same bus
  wins.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` and a CHANGELOG line.

**Why.** Iteration 30's lead asked what the panel could not show. Reading the code answered it: `SirenMix::deepest_duck`
is documented *"for a panel that says what a line of dialogue did"* — and the panel never drew it. Probed further, the bus
row showed **how far** and never **by what**, while `Entry::Ducked` records the cue. A person had to find the log line to
learn which cue lowered the bus, with the answer already in the panel's hand.

**Evidence.** `runs/2026-11-03-31/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{duck-by-1280,duck-by-1920,no-duck-1280}.png`.

**Measured.**

| state | before | after |
|---|---|---|
| Music ducked by a line | `duck -8.0 dB` | `duck -8.0 dB **by dialogue.greeting**`, one line at 1280 and 1920 |
| the duck releases | — | `duck +0.00 -> row shows no duck` (probed) |
| a second duck on the same bus | — | the newest `Ducked` wins (test) |
| a bus nothing ducked | — | no name (test) |
| every test, both feature sets | 112 / 119 / 121 + 5 | 113 / 120 / 122 + 5 |
| every rendered cue | — | **byte-identical** (21/21) |
| frame cost, 8 / 64 / 256 | — | 0.066 / 0.332 / 0.884 µs, load 3.9–5.1; the annotation scans the 256-entry log once per **ducked bus** on a debug view |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The **released** state was reached through a probe rather than
a capture, because the scripted run re-fires a duck every 20 frames — drive the state, not the animation.

**Traps for the command.** (1) A method whose doc names a consumer and which that consumer never calls is either a dead API
or a **missing view**; check which before deleting or keeping. (2) A panel row is a **width budget**: measure the annotated
row (71 chars) against the columns that fit (~114 at 1280) *and* look at the capture. (3) An annotation whose condition is
the same as the fact it annotates cannot go stale — render it **inside** the guard. (4) The example is outside the suite, so
pin the **rule** it uses in `tests/`, or a change to `Entry`/`recent` breaks the panel in silence. (5) `hs distill`
GPU-blocked (thirtieth iteration).

**Commit.** `b4f141b`, pushed to `main`; CI run 41596558047 is **success** on `big-s1`. Gate: build, test (113 + 5),
serde (120 + 5), mixer (122 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it.
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, twentieth pass.** The panel class has now paid four times (26, 29, 30, 31), and every one was found by reading
the code against the picture. What the panel still does **not** show, and the crate records, is the **music's own state**:
`MusicState::layer_db`, `target_db` and `moving()` are drawn by no view, and the panel's `Cues` block lists `track.calm`
and `track.combat` as ordinary cues when they are **layers of a score**. Next probe: run `examples/mixer` with
`MusicPlugin` and a score, and see whether the panel can tell a layer from a one-shot.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 30 — 2026-11-03 — the panel's `Problems` block is bounded so the log survives

**Change.**

- **`examples/mixer.rs` — the `Problems` block shows three sentences**, and when there are more it prints
  `and N more; the log below is what played`. The panel's fixed column has no scroll and the log is its last section.
- **`SIREN_FAULTS=many`** adds four more deliberate faults, so the case reproduces; `SIREN_FAULTS=1` (two faults, the
  README's GIF) stays under the bound and is unchanged.
- `docs/media/mixer.{png,gif}` regenerated; `docs/media/README.md` says why the block is bounded; a CHANGELOG line.

**Why.** Iteration 29's lead (`master_db` at silence) probed **clean**, as did *one answer per play* (6 asks → 2 `Played` +
4 `Refused` = 6, each naming the right thing). The probe that paid came from **looking at a capture**: with six faults the
block pushed the `Log` heading and its ten lines off the bottom of a 1280×720 frame — the section that answers *"why is the
music quiet"*, lost exactly when there was most to read. The log block was already bounded at `recent(10)`; the block
above it was not.

**Evidence.** `runs/2026-11-03-30/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/panel/{many-faults,many-faults-after,two-faults-after,clean-after,zoom-problems-many}.png`.

**Measured.**

| faults | before | after |
|---|---|---|
| six (`SIREN_FAULTS=many`) | six sentences; **`Log` off screen** | three + `and 3 more; …`; **log on screen** |
| two (the README GIF) | both sentences, log present | **unchanged** — the bound does not fire |
| none | `Problems` / `none` | unchanged |
| every test, both feature sets | — | unchanged (112 / 119 / 121 + 5) |
| every rendered cue | — | **byte-identical** (21/21) |
| panel frame time | 8.34–8.84 ms | 8.34–8.84 ms; the bound does strictly **less** work on a fault-heavy table |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first version, caught by looking: the count
line used an em dash and the panel's font has no glyph for it — the capture showed `and 3 more ▯ the log`. It is ASCII now.

**Traps for the command.** (1) A **fixed, unscrollable column** is a budget: every section added takes height from the ones
below, and the last one wins or loses. (2) If the panel bounds one list (`recent(10)`) it must bound the next, or the
unbounded one eats the bounded one's space. (3) **A debug font is not a text editor**: an em dash drew a box; keep panel
text ASCII. (4) The failure was invisible in every clean capture — six faults were needed, and `SIREN_FAULTS=many` exists
so the next run can see it. (5) `hs distill` GPU-blocked (twenty-ninth iteration).

**Commit.** `5905ae4`, pushed to `main`; CI run 41596029576 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **seven** items wait on it (`empty-sound-key.md`,
   `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`, `duck-recovers-entry.md`, the despawn
   record, and now `SIREN_FAULTS=many`'s four faults have no case in `cues.yaml`).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, nineteenth pass.** The panel class has now closed three defects (26's absent block, 29's stale pictures, 30's
overflow), and each was found by **looking**, not by a test. What the panel still cannot show is **`SirenMix::deepest_duck`**
— the bus rows carry a `duck` figure but nothing names *which* duck is deepest or that a duck **finished**; the header
counts cues, buses and voices but not ducks. Next probe: does the panel have any line for a duck that ended, and is the
header's count the one a person needs?

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 29 — 2026-11-03 — the README shows the panel the code draws

**Change.**

- **`docs/media/mixer.png` regenerated** from the clean table — the state a project ships: nine cues, `Problems / none`.
- **`docs/media/mixer.gif` regenerated** with `SIREN_FAULTS=1` — two rows `<- problem`, the block naming both.
- **`docs/media/mixer-panel.gif` deleted** — created by iteration 26, named in no README.
- **Both alt texts rewritten from the pictures**; the GIF's alt describes the animation, not one frame. `README.md` and
  `docs/media/README.md` updated, with the still's and the GIF's regeneration commands.
- **`runs/2026-11-03-29/check_media.py`** — every image a README names exists, and every image in `docs/media` is named
  by one.

**Why.** Iteration 28's lead (a stale `set_voice_db`) probed **clean** — `occupant_mut` compares the packed generation and
the crate's own test already asserts it — as did three more (every `Attenuation` model, a settings swap mid-duck, the
mixer's slot bookkeeping). The probe that paid was the front door: both README images were generated **before** iteration
26's `Problems` block, and the alt text named *"eight cues … eight log lines"* against a shipped table of **nine** cues and
a log of **ten**.

**Evidence.** `runs/2026-11-03-29/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{mixer-png-before.png,mixer-gif-before.gif}`, and `check_media.py`.

**Measured.**

| artefact | before | after |
|---|---|---|
| `mixer.png` | generated **before** the `Problems` block; alt counts wrong | nine cues, `Problems` / `none` |
| `mixer.gif` | same date, same staleness | two rows `<- problem`, block naming both |
| `mixer-panel.gif` | **orphan** | deleted |
| `check_media.py` | — | 6 ok, exit 0; **orphan → exit 1** on a pre-fix reproduction |
| every test, both feature sets | — | unchanged (112 / 119 / 121 + 5) |
| frame cost, 8 / 64 / 256 | — | 0.057 / 0.322 / 1.342 µs, load 3.1–6.6 (the quietest window recorded); no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own alt text, caught by looking: the first
version said *"Dialogue ducking Music by 8 dB"*, but the GIF's first frame shows `Music -2.0 dB` with no duck — the
animation starts at frame 30. Alt text for an animation describes what **happens**.

**Traps for the command.** (1) **Nothing in `cargo test` reads `docs/`**: a stale README picture is invisible to 26
iterations of a loop whose step 6 is "look at the panel". Look at the picture the README shows, not only the one you just
captured. (2) A "regenerate the GIF" step that writes a **new filename** leaves the old one in place and the README
pointing at it — one name per artefact. (3) Alt text must be read off the picture after capture: counts and levels drift
with the panel. (4) **A check that only ever passes proves nothing** — run it against a reproduction of the old state. (5)
`hs distill` GPU-blocked (twenty-eighth iteration).

**Commit.** `a793e10`, pushed to `main`; CI run 41595547980 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
2. **Answer or route around the `CueProblem` question** — **six** items wait on it (`empty-sound-key.md`,
   `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`, `duck-recovers-entry.md`, and the despawn
   record).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, eighteenth pass.** Five probes came back clean this iteration, and the clean list is now long enough to be the
finding: the mixer class, the attenuation models, the settings-swap prunes, the stale-index guard, and the front door.
What has **never** been probed is **`SirenSettings::problems()` against a table it must accept but a host would
misconfigure** — specifically `master_db` at `SILENT_DB` with every bus at 0: does the mix report silence through every
bus, and does `bus_db` say `SILENT_DB` rather than a chain that has one silent stage? Next probe.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Note on the skill's machine facts.** The build-loop text says `1.98.1` is "Bevy's exact floor". The crate's
`rust-version` is **`1.97.1`**, `bevy-0.20.1`'s own `rust-version` is **`1.97.1`**, and `README.md` says MSRV 1.97.1 —
all three agree, so the note is wrong for this dependency. Recorded; the `rust-toolchain.toml` pin at 1.99.0 is what
every `cargo` call here uses and is unaffected.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 28 — 2026-11-03 — a despawned voice frees its mixer slot

**Change.**

- **`src/playback.rs` — `start_pending` keeps the starts it handed to the mixer in a
  `Local<Vec<(Entity, String, usize)>>`** and, once per frame, retains only those still in
  `Query<(Entity, &MixerVoice, &Voice)>`. A start whose entity is gone — its owner despawned — is stopped through the
  trait's existing `stop_voice`, freeing the slot.
- One test behind `mixer`, with its guard in the same test; a CHANGELOG line and a media note.

**Why.** Iteration 27's lead was `Play::on(entity)`. Probed, Bevy's path is fine (its sink dies with the entity); the
crate's mixer **never** called `stop_voice` for a despawned voice, so the `rodio` player sounded for ever with its slot
held — a leak **and** a two-backend divergence the crate's own rule forbids. Invisible on the default path, which is why
27 iterations did not see it.

**Evidence.** `runs/2026-11-03-28/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/mixer-slot{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| owner alive, 10 frames (guard) | `stop_voice []` | `stop_voice []` |
| owner despawned, 10 frames | **`stop_voice []`** — leak | **`stop_voice [0]`** — slot freed |
| `SirenMix::voices_on("Effects")` | 0 | 0 |
| the log after the despawn | `["played engine …"]` | **unchanged — the record gap, named** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.338 µs at 256 | median 0.910 µs, load 10.0–7.4 — a drop this change cannot cause (`perfcheck` has no `Output`, so `start_pending` returns early); reported as measured |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **Two drafts of `research.md`/`plan.md` were wrong**: they
argued the fix needed a new public resource and wrote a decision file instead; probing further showed a `Local` does it
with no public type at all. Both files were rewritten before the code was committed. A component `on_despawn` hook was
considered and rejected — Bevy 0.20 has one, but its `DeferredWorld` cannot reach the `Output` resource.

**Traps for the command.** (1) A component on an entity is **not** a record that outlives it: the crate's own bookkeeping
lived on the voice, so the voice's death erased it. (2) **A leak can hide on the default path** — this one was invisible
without `--features mixer`, which is why the skill's "both feature sets" rule exists. (3) When a fix looks like it needs a
public type, check for a `Local` first: a system parameter is private and often enough. (4) A hook that cannot reach the
resource it must call is not the place; reconcile where the trait object is. (5) `hs distill` GPU-blocked (twenty-seventh
iteration).

**Commit.** `e8a8a32`, pushed to `main`; CI run 41595012489 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (121 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **A `Stopped` line for a parent's despawn** — neither backend writes one; the log keeps `played` and nothing else.
   Needs a new reason, so the public-enum decision.
2. **Answer or route around the `CueProblem` question** — now **six** items wait on it
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`,
   `duck-recovers-entry.md`, and this).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, seventeenth pass.** The mixer class has now paid twice (iteration 17's `AudioPlayer` fallback, and this). Its
remaining untested surface is **`set_voice_db` on a voice whose slot was reused**: the trait takes `(bus, index)` and the
mixer packs a generation, so a stale write should address nothing — case 26 claims that, and `tests/mixer.rs` proves it for
`finished` and `stop_voice`. Whether a **stale `set_voice_db`** is equally harmless is the next probe.

**Baseline note.** Nothing was red; the suite had 112 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 27 — 2026-11-03 — a duck ends its hold when the voice that justified it stops

**Change.**

- **`src/mix.rs` — `DuckClock::source_went_quiet(live, settings, now)`**, called once per frame in `update_mix` before the
  bus loop. A rule whose `source` bus has **no live voice** has its clock rewound **once**, by the hold that had not yet
  elapsed, so the fade is already underway and keeps advancing.
- **`update_mix` counts live voices before the bus loop**, with the same predicate `SirenMix::voices` uses (`not
  Stopping`), so the duck and the panel agree on what a player hears. `duck_db` keeps its signature and is the single
  read; a vestigial `duck_db_with` was deleted.
- Three tests (one failing on the base commit, two guards); a `buses.md` paragraph; a CHANGELOG line; and
  `needs-decision/duck-recovers-entry.md` for the log entry this does **not** add.

**Why.** `Duck`'s own doc says a duck holds because *"a voice starting on `source`"* is playing; `DuckClock` ran the full
hold whatever happened to the source. A line stopped on frame 0 kept `Music` at −8 dB for another 15 frames and spent
0.6 s recovering — **0.85 s of duck for a sound that had already stopped**. The skill names *"a duck that does not
recover"* as rank-1; this was that failure arriving late.

**Evidence.** `runs/2026-11-03-27/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/duck-hold{,-before}.txt`.

**Measured.**

| case | before | after |
|---|---|---|
| the line is **stopped** on frame 0 | `+0.0` at frame **60** | `+0.0` at frame **40** |
| a **loop** keeps playing (guard) | `+0.0` at 60 | unchanged |
| a **second line** at frame 20 (guard) | extended past 70 | unchanged |
| a **one-shot that ends by itself** (guard) | `+0.0` at 60 | unchanged |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.219 µs at 256 | median 1.338 µs, load 16.9–8.5 — a 2.5× spread within one binary; the change is per **rule**, not per voice |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. **One row of four moves**, which is what a correct fix looks
like: only a voice a player can no longer hear releases the hold early, and a one-shot keeps its hold because it is still
live until `retire_voices` despawns it.

**Traps for the command.** (1) An envelope keyed on *when it fired* cannot notice that its justification has gone away;
the clock needs the fact, not just the time. (2) **Clamping** an elapsed time (`max(elapsed, hold)`) pins a fade at the
hold boundary **every frame** so it never advances — the duck held forever, worse than the bug; a **one-time rewind**
moves it forward. (3) Count "what a player hears" with the **same predicate** the panel uses (`not Stopping`), or the
duck and the fader disagree. (4) A fix that leaves a method nothing calls is a vestige; clippy caught `duck_db_with`.
(5) `hs distill` GPU-blocked (twenty-sixth iteration).

**Commit.** `9bf836a`, pushed to `main`; CI run 41594075870 is **success** on `big-s1`. Gate: build, test (112 + 5),
serde (119 + 5), mixer (120 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it** — now **five** items wait on a public-enum decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`,
   `duck-recovers-entry.md`). Asked in iteration 24; not re-asked since.
2. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. Quantise a **retarget** onto the bar (iteration 14).
5. `Refusal` payload naming the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, sixteenth pass.** The duck class is now closed on the **release** side. Probed the **attack** too, before
writing it down — a second, deeper duck firing while the first is halfway through its fade:

```
after the -8 dB duck recovered halfway:  ducked = -2.80
a -12 dB duck fires:                     ducked = -12.00   (deeper wins; a sum would be -20.00)
```

**Clean**, and exactly what `duck_db`'s doc claims (*"two ducks do not stack: the deeper one wins"*). The duck class is
closed at both ends.

**Next probe:** with the duck closed, the surface left is **`Play::on(entity)`** — a play parented to a game entity. The
crate inserts `ChildOf(target)`, and nothing tests what happens when that entity is **despawned while the sound plays**:
does Bevy despawn the voice with it, does the budget still count it, and does `SirenLog` say anything?

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares
`pub mod audio_panel` twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 26 — 2026-11-03 — the panel shows the crate's verdict on its own table

**Change.**

- **`examples/mixer.rs` — a `Problems` block and a row marker.** `draw_panel` read `SirenMix`, `CueTable`, `SirenSettings`
  and `SirenLog` and **not** `problems()`, so a cue on a bus the mix lacks, or with an inverted pitch range, was drawn
  exactly like a healthy one. The panel now lists `table.problems_in(&settings)` (or `none`) and marks a bad cue's row
  `<- problem`.
- **`SIREN_FAULTS=1`** adds two deliberate mistakes to the capture, so the view can be proved; the shipped table stays
  clean. Documented beside the other knobs.
- `docs/media/mixer-panel.gif` regenerated; a CHANGELOG line.

**Why.** Iteration 25's log recorded the gap, verified. The panel's job is the editor's — make a fault visible — and it
could not distinguish a playable cue from an unplayable one. The crate already computes the sentence
(`CueProblem::Display`) and the Cauldron's panel already chains `problems_in`; the crate's own example did not.

**Evidence.** `runs/2026-11-03-26/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{clean-1280,faults-1280,zoom-problems-clean,zoom-problems-faults}.png`, and `docs/media/mixer-panel.gif`.

**Measured.**

| table | before | after |
|---|---|---|
| clean (11 cues, `Ambience` + `Music`) | no `Problems` section | `Problems` / `none`, no row marked |
| `SIREN_FAULTS=1` | the two bad cues drawn as healthy | both rows `<- problem`; two sentences in the block |
| `problems()` instead of `problems_in` | — | would print **five false `UnknownBus`** on the clean table |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| panel frame time | 8.3–13.6 ms | 8.3–9.7 ms, load 7.4–9.6 — the block is not measurable in the spread |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first version, caught by **looking** at the
1280 capture: the row marker was the full `CueProblem` sentence, and it **wrapped onto a second line**, breaking the cue
column's alignment and repeating the block below. One form per idea — the row says three words, the block says the
sentence.

**Traps for the command.** (1) A panel is a **view of the crate's own verdicts**, not a second formatter: if the crate
computes a sentence, the view should show it rather than a re-description that can drift. (2) A marker that repeats a
sentence already on the same screen is two forms of one idea — and in this case it also **broke the layout**, which only
looking catches. (3) `problems_in(&settings)`, not `problems()`: an example with custom buses gets five false sentences
from the default-four wrapper. (4) A capture that proves a view needs a **faulty** input; keep it behind a knob so the
shipped picture stays clean. (5) `hs distill` GPU-blocked (twenty-fifth iteration).

**Commit.** `4ccd29a`, pushed to `main`; CI run 41593125114 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it.** Four variants wait on one decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`); asked in
   iteration 24, not re-asked since.
2. `Refusal::NotALoop { layer, cue }` — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, fifteenth pass.** The panel now shows the crate's verdict and has been looked at in both states. I wrote,
before checking, that the capture never contains a **released** duck — the script ends on one still holding. Probed
directly rather than inferred from a picture:

```
Music duck per frame after dialogue.greeting, default settings (60 fps)
  frame  0:-8.0  10:-8.0  15:-7.9  20:-6.9  30:-4.7  40:-2.6  50:-0.5  55:+0.0  65:+0.0
```

So the duck **does** recover, by frame 55 of the crate's own frame count — 0.25 s hold plus 0.6 s fade, which is the
crate's documented envelope, and the claim in the previous draft of this entry was **wrong**. What the *capture* shows
depends on the script the capture runs, not on the crate: `steal,layer,duck` cycles every `SIREN_KEY_EVERY` = 20 frames
from frame 30, so a `duck` step fires often enough that a released duck is on screen for part of the run.

**Next probe:** the panel's `duck` column shows the *current* duck only. Nothing on screen says a duck **finished** — the
log's `ducked` lines have no matching `recovered` line, so a reader sees the dip and not the release. Whether that is
worth a line is the next question, and it is a `SirenLog` entry, so it is **public API** and goes to a decision file
before it is built.

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 25 — 2026-11-03 — the capture harness runs with no environment at all

**Change.**

- **`examples/mixer.rs` — `capture_config` gives `SIREN_KEYS` the default its doc describes.** Unset used to yield an
  empty step list, and `stage_keys` evaluated `% c.keys.len()` — `% 0` — so `SIREN_FRAMES=140 cargo run --release
  --example mixer` **panicked and wrote no still**. The default is now `steal,layer,duck`; a `SIREN_KEYS` that parses to
  nothing prints one line naming the typo before using it.
- **The panel, captured and looked at for the first time** at 1280×720 and 1920×1080, with a bus/log crop and
  `docs/media/mixer-panel.gif` (18 frames via `SIREN_SHOT_DIR`); a `docs/media/README.md` entry with the commands.
- A CHANGELOG line and a media note.

**Why.** Iteration 24's lead was `examples/mixer`, and running it was the probe. `%` on an integer panics in every
profile, so this was not a debug assertion: the crate's only **visible** harness died on the command its own module doc
gives, which is why every visible change this loop has shipped was evidenced by the headless `examples/render` alone.

**Evidence.** `runs/2026-11-03-25/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`,
`evidence/panel/{mixer-1280,mixer-1920,zoom-buses,zoom-log}.png`, `evidence/panel/default-run{,-before}.txt`, and
`docs/media/mixer-panel.gif`.

**Measured.**

| command | before | after |
|---|---|---|
| `SIREN_FRAMES` only | **panic**, no still | still written at both sizes |
| `SIREN_KEYS=bogus` | panic | one line naming the typo, then the default |
| `SIREN_KEYS=layer` (guard) | works | unchanged |
| no environment at all | panic | 20 s, no panic |
| panel frame time (new figure) | — | 8.3–13.6 ms mean, an example's number, not the crate's |
| crate frame cost, 8 / 64 / 256 | 0.061 / 0.348 / 1.244 µs | unchanged — no `src/` line moved |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The panel **passes the skill's own pass-3 test**: every
number on a bus row (`dialogue.greeting` ducked Music by −8.0 and Effects by −4.0; `engine.idle` `stolen by sword.hit`)
is one the log beneath it explains, so a developer reads it without the code. One crop mistake of mine is recorded rather
than quietly fixed: `zoom-buses.png` starts at `Ambience`, because I offset by 150 px and the Music row *is* at 150.

**Traps for the command.** (1) A documented **optional** knob that is load-bearing is a crash waiting for the default
command. (2) `% len()` on a list built from an env var needs the empty case answered before the modulo, not after.
(3) An example that cannot run cannot be **looked at**, and this loop's step 6 depends on looking — the missing panel
evidence was not laziness, it was a panic. (4) When a crop looks wrong, check the offset before the panel: `zoom-buses`
starts at `Ambience` and the full still has all four buses. (5) `hs distill` GPU-blocked (twenty-fourth iteration).

**Commit.** `97f79ff`, pushed to `main`; CI run 41592625502 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **Answer the `CueProblem` question, or route around it.** Four variants wait on one decision
   (`empty-sound-key.md`, `duplicate-sound-problem.md`, `bad-gain-problem.md`, `not-a-loop-refusal.md`); iteration 24
   asked, and this one did not re-ask. If the owner does not want the break, the next run should pick outside the enum.
2. `Refusal::NotALoop { layer, cue }` — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, fourteenth pass.** The visible-harness class is now open in a way it was not: the panel exists as a picture
and **has never been checked against a table the crate rejects**. Probed before writing it down — `draw_panel`'s four
resources are `SirenMix`, `CueTable`, `SirenSettings` and `SirenLog`, in that order, and **`problems()` is not among
them**:

```rust
fn draw_panel(mix: Res<SirenMix>, table: Res<CueTable>, settings: Res<SirenSettings>, log: Res<SirenLog>, …)
```

So the panel draws a cue on an unknown bus, or one with an inverted pitch range, exactly as it draws a good one. That is
the editor's whole job — making a fault visible — and it is the **next item**, measured rather than guessed.

**The hunt, fifteenth step (recorded, not built):** the Cauldron's `audio_panel.rs` *does* chain `problems_in(&read)` into
its status line, so the editor has the line the crate's own example lacks. The gap is in the example, and it is named
above so the next run does not have to find it again.

**Baseline note.** Nothing was red; the suite had 109 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 24 — 2026-11-03 — a refusal outlives a burst of plays

**Change.**

- **`src/log.rs` — `SirenLog::push` drops the oldest *routine* entry when the buffer is full**, not simply the oldest.
  `Played`, `Stopped` and `Ducked` happen at frame rate; a healthy game produces no `Refused` at all. If every entry is
  a refusal the oldest still goes. The buffer stays `CAPACITY` (256) — same memory, same `recent`/`iter`/`len`.
- Four tests, two failing on the base commit and two guards; a CHANGELOG line and a media note.

**Why.** Iteration 23's lead was the log's cap under a burst. Its **arithmetic** was clean (capped, oldest dropped,
`recent(n)` safe past the buffer, no growth), and so were four other probes. The one that paid asked a different
question: **how long a *refusal* lives**. One `Refused` was evicted by exactly **256 plays** — **0.40 s** at a 32-voice
scene writing 640 entries a second. That is the silent-failure class the log exists to prevent.

**Evidence.** `runs/2026-11-03-24/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/refusal-lifetime{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| one refusal, then plays | evicted after **256 plays** | survives **2000**; leaves at 256 *routine* entries |
| a 32-voice scene at 640 entries/s | **0.40 s** | ≥ 0.40 s, and longer in a quiet scene |
| 600 refusals (guard) | len 256, oldest `c344` | unchanged |
| 1000 mixed pushes (guard) | len ≤ 256 | unchanged |
| cost, 200k routine pushes | — | 11.1 ms (**55 ns** each) |
| cost, 1000 pushes into an all-refusal log | — | 272 µs (**272 ns** each, pathological) |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.391 µs at 256 | median 1.267 µs, load 8.8–13.8 — within the spread; the log is not on `perfcheck`'s path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction in my own evidence, recorded: the first probe
printed "3.12 s" for the after case by falling back to its 2000-play limit — a made-up number for a rule that has no
fixed play count. It now states the rule (256 *routine* entries) and the floor that follows.

**Traps for the command.** (1) A **ring buffer** with one eviction rule treats a routine fact and an exceptional one
alike; the exceptional one is what a person reads the buffer for. (2) Measure the *lifetime* of the entry a diagnostic
depends on, not just the buffer's size and accessors — the arithmetic was right and the retention was wrong.
(3) A probe that reports "survives N" by hitting its own limit has measured nothing; state the rule, not the loop bound.
(4) Five probes came back clean and are written down: the cap, `recent(n)`, `line()` (host-only), `next_boundary` at
awkward tempos, and every no-ears spatial shape. A clean probe is a result. (5) `hs distill` GPU-blocked (twenty-third
iteration).

**Commit.** `200a119`, pushed to `main`; CI run 41591975363 is **success** on `big-s1`. Gate: build, test (109 + 5),
serde (116 + 5), mixer (117 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **`CueProblem` for an empty `sound` key** — `needs-decision/empty-sound-key.md`; it plays and is not reported.
2. **`CueProblem` for a duplicate key** — `needs-decision/duplicate-sound-problem.md` (iteration 22).
3. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
4. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
5. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.

**The hunt, thirteenth pass.** The log class is closed. Four `CueProblem` variants now wait on one public-enum decision,
and the loop should stop re-finding them — the next run should **ask the owner** whether to take the break, or pick
something outside that enum.

Probed `SirenSounds::failed` first, before writing it down:

```
load::<AudioSource>("audio/does_not_exist.wav")
  frame 0: Some(Loading)
  frame 1: Some(Failed(MissingAssetLoader { asset_path: "audio/does_not_exist.wav" }))
  failed(table) = ["missing"]                 <- correct, and it names the key
```

So the **Failed path works** — my first probe had simply not advanced the app, which is this loop's own recurring trap
(a `Loading` state read on frame 0 is not a verdict).

One state is named by neither accessor: a **`Handle::default()`** inserted for a key is *mapped* (so `missing()` skips it)
and never loads (so `failed()` skips it). It is reachable only if a host inserts a placeholder handle, which is not a
project's table doing anything wrong — **noted, not an item.**

**Next probe:** with the log and `SirenSounds` classes closed, the surface left is **`examples/mixer`** — the one visible
harness the loop has never inspected frame by frame, and the item the skill asks about when a change is in the crate:
does it render every bus the table has, and does a bus it does not list appear as zero?

**Baseline note.** Nothing was red; the suite had 105 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 23 — 2026-11-03 — the seed the stream was built from

**Change.**

- **`src/model.rs` — `Rng::ZERO_SEED` and `Rng::seed()`.** `Rng` was a bare `u64` holding the live xorshift state; it now
  also remembers the seed it was **built from**, so `seed()` answers a number that does not move as the stream draws.
- **`src/play.rs` — `SirenChoices::new` records `rng.seed()`**, so `SirenChoices::seed()` — *"the seed this stream was
  built from"* — is true by construction.
- Four tests; a paragraph in `docs/guide/cues.md`; a CHANGELOG line; and `needs-decision/empty-sound-key.md` for a
  separate gap found while probing.

**Why.** `Rng::new(0)` maps a zero state to `0x5EED_51AE_0000_0001`, which is **also**
`SirenSettings::default_seed()`. So `seed: 0` and the default are **one stream**, and `SirenChoices::seed()` reported the
argument — `0` — for a stream that started somewhere else. Two identical streams wearing two different numbers, and the
one that says `0` is a lie. The reproducibility promise is the crate's own stated reason for a seeded stream (*"a sound
that cannot be reproduced cannot be debugged"*), so the report has to name the seed the sound came from.

**Evidence.** `runs/2026-11-03-23/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/seed{,-before}.txt`.

**Measured.**

| settings | before: `seed()` | after |
|---|---|---|
| `seed: 0` | **`0`** — a number the stream did not come from | `6840213216811810817` |
| default seed (guard) | `6840213216811810817` | `6840213216811810817` |
| `seed: 1` (guard) | `1` | `1` |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.31 µs at 256 | median 1.391 µs, load 10.2–17.5 — within the spread; the change is one `u64` written once |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. A defect in my own first attempt is recorded: `Rng::seed()`
initially read the **live** xorshift state, which changes on every draw — the test
`a_streams_seed_does_not_move_as_it_draws` fails against that version and is why it exists.

**Traps for the command.** (1) A **sentinel** that collides with a real value is a lie waiting to be told: `0` mapping to
the default seed's constant means two names for one stream, and the report must name the value the stream holds.
(2) A method documented as *"the seed this stream was built from"* must read something that **does not move** — the live
state is the wrong field. (3) A lead that comes back **clean** is a result: the `Rng` stream is uniform, long, and never
returns 1.0, so the index clamp is defensive — written down so the next iteration does not re-probe it. (4) Check a new
name against **every** other name check: bus names and cue names are trimmed-checked, a slot's `sound` key is not.
(5) `hs distill` GPU-blocked (twenty-second iteration).

**Commit.** `7aff968`, pushed to `main`; CI run 41591211228 is **success** on `big-s1`. Gate: build, test (105 + 5),
serde (112 + 5), mixer (113 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (iteration 21's blocker, unchanged).

**Next 5.**

1. **`CueProblem` for an empty `sound` key** — `needs-decision/empty-sound-key.md`; it plays and is not reported.
2. **`CueProblem` for a duplicate key** — `needs-decision/duplicate-sound-problem.md` (iteration 22).
3. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
4. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
5. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.

**The hunt, twelfth pass.** The **seed** class is now closed: the mapping is named, reported, and pinned as behaviour.
Four `CueProblem` variants are waiting on one public-enum decision, and the loop should stop re-finding them.

Probed the **cue table as a replaced resource** — `CueTable` is a `Resource`, and iterations 13/14 fixed `SirenSettings`
and `MusicScore` replacements, so this was the last one:

```
table swapped for one with no `old`, then StopCue { cue: "old" }
  40 frames later:  []       no voice, no `Stopping`
  no swap, same test:  []    identical
  log: stopped `old`: cue stopped   played `old` (a) on Effects at 0.0 dB
```

**Clean.** A voice outlives the table that named it, fades, and despawns exactly as it does without the swap — which is
right: the voice's own `Voice` record carries what it needs, and nothing reads the table on the way out. My first probe
reported `live 1` only because it counted `&Voice` without excluding `Stopping`, which is the same mistake this loop has
caught before; the corrected probe is the one above.

**Next probe:** the **`SirenLog` cap under a burst** — `CAPACITY` is 256 and `push` evicts the oldest, but nothing asserts
that the entry a panel is *currently drawing* survives a frame that writes 300 entries, or that `recent(n)` with `n`
larger than the buffer returns what it has rather than panicking.

**Baseline note.** Nothing was red; the suite had 101 tests. `cues.yaml` case 31 is red by design.

**Consumer status.** Unchanged: `bevy_grimoire`'s `main` still lacks `AudioPanelPlugin` and declares `pub mod audio_panel`
twice, so the panel test cannot run; `799fad2` waits on local `main`.

---

## Iteration 22 — 2026-11-03 — `AvoidRepeat` avoids the sound, not the slot index

**Change.**

- **`src/model.rs` — `VariantPicker::pick_from(&[&str], avoid, rng)`.** The avoided window is made of **keys**, so two
  slots naming one file are one candidate for repetition. `pick(len, ..)` is unchanged for every caller with no keys,
  and the two agree when keys are distinct (a test seeds both from `Rng::new(7)`).
- **`src/play.rs` — `SirenChoices::pick` calls it** with the cue's slot keys, so the play path gets the fix with no API
  change.
- Four tests; a *"the window is made of sounds"* paragraph in `docs/guide/cues.md`; a CHANGELOG line; and
  `needs-decision/duplicate-sound-problem.md` for the report half.

**Why.** Iteration 21's lead was colliding slot keys. One key → two files is not representable (by design), but the
**same key twice in one cue** is, and the picker could not see it: it refused **indices**, so under the crate's *default*
`Repeat::AvoidRepeat { avoid: 1 }` a cue with `["a", "a", "b"]` played the same recording **three times running**
(`… b a a a b`), and `["a","a","a","b"]` played four. That is the machine-gun footstep `Repeat` exists to kill — the
crate's headline feature — defeated by a copy-pasted line, on by default, with `problems()` silent.

**Evidence.** `runs/2026-11-03-22/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/duplicate-keys{,-before}.txt`.

**Measured.**

| cue, `AvoidRepeat { avoid: 1 }`, 16 plays | before | after |
|---|---|---|
| `["a", "a", "b"]` | **3** adjacent repeats | **0** |
| `["a", "a", "a", "b"]` | **8** | **0** |
| `["a", "b"]` (control) | 0 | 0 |
| `["a"]` (one sound, guard) | 15 | 15 |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | median 1.31 µs at 256 | same binary spread 0.393–1.350 µs across five runs, load 8.6–16.5; the change is once per **play** |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction recorded rather than smoothed: the first
perf run read 2.5× *faster* than iteration 21 — impossible for a change that adds work — so five runs were taken and
the spread reported instead of the flattering sample.

**Traps for the command.** (1) A guarantee about what a **player hears** must key on what they hear: an index is the
crate's internal name for a slot, and two slots can name one recording. (2) When a public method gains a keyed variant,
keep the old one byte-identical and **prove** the two agree on the inputs where they should — that is the no-regression
half. (3) A perf figure from one run on a machine at load 16 is a scheduler measurement; take five and report the spread.
(4) A test file behind a feature gate can run zero tests under a filter that misses — count the result, do not read "ok".
(5) `hs distill` GPU-blocked (twenty-first iteration).

**Commit.** `51e9bf3`, pushed to `main`; CI run 41588637059 is **success** on `big-s1`. Gate: build, test (101 + 5),
serde (108 + 5), mixer (109 + 5), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The
Cauldron's panel test still **cannot** run (blocker above, re-confirmed this iteration: the same two compile errors).

**Next 5.**

1. **`duplicate-sound-problem.md`** — a `CueProblem` for a duplicate key, conditional on `AvoidRepeat` (option B).
2. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
3. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
4. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
5. `load_table`'s path join — a key of `../outside` becomes `audio/../outside`; named here, not built.

**Consumer status.** `bevy_siren` is green. The Cauldron's audio panel test **cannot** pass: `bevy_grimoire`'s `main`
is still `e00d1f1`, still with two `pub mod audio_panel;` and no `AudioPanelPlugin` — iteration 21's blocker, unchanged,
not mine. My consumer commit `799fad2` sits on local `main`, ahead by one, waiting.

**The hunt, eleventh pass.** `SirenSounds` is now closed: one key → one handle is by design, a duplicate in one cue is
fixed here, and `load_table`'s path join is named. Probing the **`Rng` stream** next, before writing it down:

```
seed u64::MAX, 2,000,000 draws:  max next_f32 = 0.999999583,  values >= 1.0: 0
```

So `next_f32` never reaches `1.0`, and the `min(choices.len() - 1)` clamp in `pick` is defensive rather than load-bearing
— **clean**, no item. What remains genuinely unprobed is **`Rng`'s period**: a xorshift64\* with seed `0` still produces a
stream (measured: `0.7037902, 0.34525365, 0.2853828`), so the seed is mixed rather than used raw, but nothing asserts how
many draws it takes to cycle. That is a determinism question, not a sound one, and it is the next probe.

**Baseline note.** Nothing was red; the suite had 97 tests. `cues.yaml` case 31 is red by design.

---

## Iteration 21 — 2026-11-02 — `problems_in`, and the report's blind spot recorded

**Change.**

- **`src/cue.rs` — `CueTable::problems_in(&SirenSettings)`**, the report against **the buses the mix actually has**. It
  asks `settings.bus_names()`, the one answer `answer`'s `UnknownBus` check, `update_mix` and `SirenSettings::problems()`
  all read. `problems()` keeps its documented default-four meaning, and its doc now says what it assumes.
- **`bevy_grimoire` (its own commit)** — the Cauldron's audio panel calls `problems_in(&read)`; it was calling
  `table_read.problems()` while its own bus list, two lines below, was built from the settings.
- `cues.yaml` gains **case 31, `state: red`**, `blocked_on:` the gain decision; `docs/guide/cues.md` stops implying the
  check list is complete; `bad-gain-problem.md` gains the evidence; CHANGELOG and a media note.

**Why.** Iteration 20's lead (`serde`) came back **clean** — `to_ron` writes `NaN`, `from_ron` returns `NaN`, an unknown
field is refused by name. The probe found the report's five readers, of which four ask `bus_names()` and the fifth — the
one **the editor calls** — assumed the defaults. In a project with a `Sfx` bus, `problems()` printed
*"cue \`gunshot\` plays on bus \`Sfx\`, which the mix table does not have"* for a cue the crate **accepts and mixes**.

**Evidence.** `runs/2026-11-02-21/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/{custom-buses,gain-report}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a cue on a custom `Sfx` bus | `problems()` → **false `UnknownBus`**; the panel drew it | `problems_in(&settings)` → **empty**; the panel calls it |
| the same table with no buses listed | both agree — the crate's four | **unchanged** (the guard) |
| `Cue::gain_db: NaN` / slot `NaN` | 0 sentences | **unchanged — blocked** (case 31, red) |
| `Cue::pitch` inverted / `attenuation` negative | 1 sentence each | unchanged — the asymmetry the red case records |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.046 / 0.327 / 0.991 µs | 0.055 / 0.334 / 1.021 µs, load 7.9–16.3 — within noise; `problems_in` is a **host** call |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One correction in my own work: the new test first went into
`tests/table.rs`, which is behind `#[cfg(feature = "serde")]`, so the filter ran **zero** tests and reported success.
Moved to `tests/mix.rs`, it now fails to **compile** on the unfixed tree — the strongest "fails before".

**Traps for the command.** (1) A **convenience wrapper** that assumes a default is a trap the moment the crate supports
the non-default: name it for its assumption (`problems_with`), not for the general verb (`problems`). (2) A report is not
fixed when the *function* is right — check **every caller**, and an editor is a caller. (3) A test file behind a feature
gate silently runs zero tests under a filter that misses: confirm the count, not the word "ok". (4) A lead that comes back
**clean** is a result, not a wasted pass — write down what was probed so the next iteration does not repeat it.
(5) `hs distill` GPU-blocked (twentieth iteration).

**Commit.** `82d227e` (library; CI run 41585200646 is **success** on `big-s1`). **The consumer fix is committed in `bevy_grimoire` as `799fad2` but NOT
pushed**, because `origin/main` there does not compile with `bevy_siren` — proved from a clean worktree of `e00d1f1`:

```
error[E0428]: the name `audio_panel` is defined multiple times   (lib.rs:53 and lib.rs:109)
error[E0425]: cannot find value `AudioPanelPlugin` in module `crate::audio_panel`
```

The root cause, traced with `git log -S`: commit **`ca287d7`** added `app.add_plugins(crate::audio_panel::AudioPanelPlugin)`
to `editor.rs` and **never wrote the plugin** — `git log -S AudioPanelPlugin` finds that call and nothing else, at any
commit, in `audio_panel.rs` or in `lib.rs`. Merge **`e7ba838`** then added a **second** `pub mod audio_panel;` to
`lib.rs`. So `main` has been unable to build with `bevy_siren` since that commit, and it is not a merge artifact: it is
an unfinished feature commit. Proven from a clean `git worktree` of `e00d1f1`, twice, with the same two errors.

I did not repair it. It is someone else's commit on a shared `main`, the loop's rules have a consumer breakage fixed as
its own commit only when it is **my** change's breakage, and the two edits (a missing plugin and a duplicated module) are
a design decision about who owns the panel plugin, not a typo I can settle. The library half is shipped and green; the
consumer half waits on that fix. `799fad2` sits on local `main`, ahead by one, rebase-clean, and will fast-forward the
moment `main` compiles again. **Reported, not worked around.**

**Next 5.**

1. **`CueProblem::BadGain`** — `needs-decision/bad-gain-problem.md`; `cues.yaml` case 31 is red until it lands.
2. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14).

**The hunt, tenth pass.** The **report** class is now closed on every reader: the cue table (this iteration), the mix
table (20), the score (19), and the log (16). What has never been probed is **`SirenSounds`** — the key → handle map a
game loads — against a table whose slot keys collide (`SoundSlot::sound` naming the same file twice in one cue, or the
same key in two cues with different files). That is the next probe.

**Baseline note.** Nothing was red; the suite had 96 tests.

---

## Iteration 20 — 2026-11-02 — the report agrees with the mix about buses

**Change.**

- **`src/mix.rs` — `SirenSettings::problems()` asks `bus_names()`, and names each fault once.** It built its own bus set
  from `self.buses`, so a table with `buses: []` — documented as *"the crate's four … everywhere"* — was reported as
  having **four false missing buses** while the crate mixed it correctly. And a duck fault was pushed once per rule, so
  the crate's own default pair (`Dialogue → Music`, `Dialogue → Effects`) printed the same sentence twice.
- `tests/mix.rs` — four tests, three failing on the base commit, the fourth the default-table guard; a `buses.md`
  paragraph; a CHANGELOG line and a media note.

**Why.** Iteration 19's log ended with a **probe, not a guess**, and it found two faults in one function. The worse one is
the false report: a table the crate **mixes correctly** was reported as four mistakes, which is worse than no report —
it teaches an author to ignore the panel. This is the last caller bypassing `bus_names()` (iteration 10 closed the same
bypass in `update_mix`).

**Evidence.** `runs/2026-11-02-20/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/report{,-before}.txt`.

**Measured.**

| table | before | after |
|---|---|---|
| `buses: []` (the crate's four) | **4** sentences, all false | **0** |
| two buses (a menu mix) | **2** identical for `Dialogue` | **1** |
| one bus + two rules naming `Ghost` | **2** identical | **1**, exact words |
| default table (guard) | 0 | 0 |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.064 / 0.361 / 1.311 µs | 0.046 / 0.327 / 0.991 µs, load 8.12/14.44/19.28 — every row faster, which is the load and not this change; `problems()` is a **host** call, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One defect in my own fix, caught by the **pre-existing**
test: deduplicating by **bus name** collapsed `Nope → Music` and `Music → Nope` — two genuinely different faults about one
absent bus — into one sentence. The key must be the **sentence**: identical sentences are one fault, different sentences
about one bus are two.

**Traps for the command.** (1) A report that **invents** faults is worse than one that misses them: it trains the reader
to ignore it. (2) When a resource has a `bus_names()`-style accessor, every reader must use it — a second set is a second
rule, and the crate's own trap list is about exactly that. (3) Deduplicate a report by the **sentence**, not by its
subject: two rules about one bus can be two faults, and one rule pair about one bus can be one. (4) A **pre-existing**
test is the cheapest specification you have — the old `problems_names_each_mistake` caught the over-merge before it
shipped. (5) `hs distill` GPU-blocked (nineteenth iteration).

**Commit.** `8a41eae`, pushed to `main`; CI run 41584532000 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (96 + 4), serde (103 + 4), mixer (104 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (iteration 18).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, ninth pass.** The **report** class has now paid four times (16, 18, 19, 20) and is closed: `problems()` on
the settings and the score, the log, and the refusals all say what the crate does. Probing the budget next produced
something I could not honestly call a bug, and it is written down here rather than dropped:

**`cues.yaml` case 8 is internally inconsistent.** It authors `engine.idle` with `bus_max_voices: 2`, `mode: Loop` and
**no `max_instances`** — so the cue keeps its default `max_instances: 1` — then expects *"Two voices live, one steal
logged, and no third `Played`"* from three requests in one frame. Measured:

```
three requests, one frame, bus cap 2, max_instances 1     played 3, refused 0, live voices 1
```

The crate is **self-consistent**: each request steals the one previous voice of its own cue (the cue cap, not the bus
cap), so the bus never exceeds 2 and one voice is live — exactly what `max_instances: 1` means. A test that wants two
live voices must author `max_instances: 2`, which `tests/mix.rs::a_full_bus_steals_the_oldest_and_then_refuses` already
does with `cue("hit", "Effects", 0, 4)`. I tried making an unlimited cue respect the bus cap as a fix; the crate's own
test showed that would break a documented behaviour, so it was **reverted** and `src/play.rs` is untouched. The case's
`authored` block should gain `max_instances: 2` — a `cues.yaml` edit, which the loop's rules reserve for a new behaviour,
so it is recorded for the owner rather than made.

**Next probe:** nothing in the budget. The untested surface is the **`serde` round trip against a table that parses and
is wrong in a field no `problems()` reads** — starting with whether `from_ron` accepts a `max_instances` past the
table's `max_voices`, and whether `to_ron(from_ron(x))` is stable for a table with a non-finite number in it.

**Baseline note.** Nothing was red; the suite had 92 tests.

---

## Iteration 19 — 2026-11-02 — a music layer's gain is a caller value like any other

**Change.**

- **`src/music.rs` — `start_layers` normalises a layer's starting `gain_db` and `move_layers` every `MusicLayers`
  target** with iteration 18's `model::finite_or`, so a gain that is not a number is **no gain** (`0.0` dB), not `NaN`
  written into `db`, `target_db` and the voice's `base_db`.
- **`MusicScore::problems` gains one sentence**: `"music layer \`calm\` has a gain that is not a number"`. The score's
  report is `Vec<String>`, unlike `CueTable`'s `Vec<CueProblem>`, so **no public enum changed** — this item is complete
  with no decision file, which is why it outranked the waiting `Refusal` items.
- `tests/music.rs` — four tests, three failing on the base commit, the fourth the `SILENT_DB` guard; a guide section; a
  CHANGELOG line and a media note.

**Why.** Iteration 18 ended with a **measured** lead, and it was real: iteration 18's rule covered the five numbers that
enter `answer` and missed the one that enters `start_layers`. The score's report *already spoke* about a non-finite
number — it named a bad `tempo_bpm` — and said nothing about a bad gain.

**Evidence.** `runs/2026-11-02-19/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/music-gain{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a score layer at `gain_db: NaN` | `layer_db NaN`, `base_db NaN`, sink at **−80 dB**, `problems()` **empty** | `0.0`, `0.0`, `Applied 0.0`, one sentence |
| a `MusicLayers` move to `NaN` | `layer_db NaN`, `target NaN` | `0.0`, `0.0` |
| a layer started at `SILENT_DB` (guard) | −80 dB | **unchanged** — `SILENT_DB` is finite |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.079 / 0.357 / 1.288 µs | 0.064 / 0.361 / 1.311 µs, load 8.07/9.79/13.07 — within noise; the change is once per layer, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two corrections are recorded rather than hidden: the
severity is **silent until moved**, not permanent (`approach(NaN, x)` returns `x`), and the move test first failed
because `MusicLayers::new` defaults to `When::NextBar` and the test ran 0.5 s.

**Traps for the command.** (1) A rule applied to *some* doors is worse than no rule: it makes the crate look consistent
while one field still lies. When adding a rule, enumerate **every** entry point. (2) Check what a report's type is before
designing the fix: `MusicScore::problems` is `Vec<String>` and `CueTable::problems` is `Vec<CueProblem>`, and that
difference is whether a fix needs a decision file. (3) A **finite** sentinel is not a bad number — `SILENT_DB` must
survive any normalisation, and that is a test, not a comment. (4) Before claiming "permanent", probe recovery: the
crate's `approach` already guards a non-finite current value. (5) `hs distill` GPU-blocked (eighteenth iteration).

**Commit.** `17eb4a5`, pushed to `main`; CI run 41579423077 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (92 + 4), serde (99 + 4), mixer (100 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (iteration 18).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, eighth pass.** The non-finite class is now closed at every door a **gain or detune** has: `answer`'s five
numbers (18), a layer's start and a `MusicLayers` target (19). Probing the bus level **before** writing it down, because
the obvious guess was wrong:

```
BusLevel::new("Music").gain_db(NaN)
  SirenSettings::problems()  ->  ["bus `Music` has a gain that is not a number"]   <- reported
  SirenMix::bus_db("Music")  ->  -80.00                                            <- and silent
```

So a bad bus level is **reported and silenced**, which is the opposite of this iteration's case (silent and
unreported). Whether that is right is a judgement, not a defect: a bus is the mix's own protection, and `chain_db`'s
silence is what keeps a `NaN` from making every voice on it audible. **No item here** — and the probe also showed
`problems()` reporting the default duck's `Dialogue` source twice when the table lists two buses, which is a
**duplicate sentence**, and that *is* a small defect worth its own item.

**Baseline note.** Nothing was red; the suite had 88 tests.

---

## Iteration 18 — 2026-11-02 — a number that is not a number

**Change.**

- **`src/model.rs` — `finite_or`, `ratio_to_semitones`, `played_semitones`.** One rule for every non-finite caller value
  the crate accepts: it is the **neutral** value for what it measures, which is what `distance_gain` already does for a
  distance and `docs/guide/spatial.md` already states (*"a `NaN` transform gets a sound at the listener's feet, not a
  `NaN` volume that silences the whole bus"*).
- **`src/play.rs` — `answer` normalises the four numbers that enter from a caller** (a cue's gain, a slot's gain, a
  `Play`'s gain override, a `Play`'s detune) and reports the detune the device is **actually at**.
- `tests/play.rs` — four tests, three failing on the base commit; a *"A number that is not a number"* section in
  `docs/guide/buses.md`; a CHANGELOG line; and `needs-decision/bad-gain-problem.md` for the two stricter answers.

**Why.** Iteration 17's lead (the `AudioOutput` trait's contract) came back **clean**: `Applied` records what the crate
decided, and a backend that ignores it desynchronises nothing the crate reads. The probe that paid was different: the
crate had **two opposite answers to one caller mistake**. A `NaN` distance was the neutral `0.0` m; a `NaN` gain was
silence. A cue authored `gain_db: NaN` — which a RON file can contain — **played silent** while `Played` reported
`-80.00` as though that level were authored, and `problems()` said nothing. Worse, `Played.pitch` reported `NaN` for a
sound `semitones_to_ratio` was already playing at `1.0×`: the meter disagreed with the device, which is the one thing
R6's second half forbids.

**Evidence.** `runs/2026-11-02-18/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/nan{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a cue authored `gain_db: NaN` | `gain -80.00`, **silent**, no problem reported | `gain 0.00`, audible |
| `Play::gain(NaN)` | `gain -80.00` | `gain 0.00` |
| `Play::gain(-∞)` | `gain -80.00` | `gain 0.00` |
| `Play::pitch(NaN)` | `pitch NaN` for a `1.0×` sound | `pitch 0.00` |
| `Play::pitch(1e9)` | `pitch 1000000000.00` for a `16×` sound | `pitch 48.00` |
| a bus at `SILENT_DB` (guard) | silent | **unchanged** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 | 0.077 / 0.370 / 0.713 µs | 0.079 / 0.357 / 1.288 µs, load 8.72/4.85/4.83 — the 256 row sits in a loaded first sample; the change is once per **play**, off the per-frame path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The judgement this iteration had to make — neutralise or
refuse — is written down in `needs-decision/bad-gain-problem.md` with what each costs a consumer.

**Traps for the command.** (1) Two answers to one caller mistake is a bug even when both answers are individually
defensible: find the **stated policy** and make the other path match it. (2) A **meter** that reports the number asked
for rather than the rate that plays is wrong even when the sound is right — `semitones_to_ratio` had the audio correct
and `Started.pitch` re-broke it. (3) `chain_db`'s silence rule is right for a **bus level the crate computed** and wrong
for an **input**; the boundary is "did the crate compute this number". (4) Normalising a bad input is a **behaviour
change** and belongs in a decision file, not in a patch's fine print. (5) `hs distill` GPU-blocked (seventeenth
iteration).

**Commit.** `fdbc1d5`, pushed to `main`; CI run 41578642257 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (88 + 4), serde (95 + 4), mixer (96 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **`CueProblem::BadGain` + the refusal for a bad request** — `needs-decision/bad-gain-problem.md` (this iteration).
3. Re-time a restarted stem to the bar — `needs-decision/restart-on-the-bar.md`.
4. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, seventh pass.** Two structural leads have now come back **clean** (the two-backend answers, the
`AudioOutput` contract). Probing the third before writing it down: **a `MusicLayer` whose `gain_db` is `NaN`** does
reach `MusicState` unfiltered — measured,

```
MusicLayer { name: "calm", cue: "t", gain_db: NaN }
  layer_db = Some(NaN), target_db = Some(NaN)    score.problems() = []
```

so the layer's gain is `NaN` for the rest of the process and nothing reports it. `MusicLayer::gain_db` is the same
class of caller number this iteration normalised, reaching a **different** field by a different door — the one place
`finite_or` was not applied. That is the next item, not a guess: measured, and left unbuilt because this iteration
shipped a complete change.

**Baseline note.** Nothing was red; the suite had 84 tests.

---

## Iteration 17 — 2026-11-02 — the backend flag does not decide ownership alone

**Change.**

- **`src/play.rs` — a voice is mixer-owned only when a mixer is there to own it.** `Backend` is a public resource whose
  flag a host may set, and `spawn_voice` decided *"the mixer owns playback"* from that flag alone. With `Backend(true)`
  and no `Output`, a voice carried **no `AudioPlayer`** and had **no mixer**: it could not sound, so it never ended, so
  it was never retired — it held its budget slot for the life of the process while `Played` said it had played.
  Ownership is now `backend.0 && output.is_some()`.
- `tests/play.rs` — two tests: the fallback (**fails on the base commit**) and the guard (with an `Output`, the voice
  still carries no `AudioPlayer`, so the two paths cannot both play it). Plus a CHANGELOG line and a media note.

**Why.** Iteration 16's lead was the **two-backend** promise. Probed, a **steal** logs identically on both paths and a
scripted **duck** gives identical `bus_db`/`ducked` (min −8.000 dB on each) — the claim holds. The defect was one level
down: not *do the backends answer the same*, but *can a backend answer nothing at all*. It ranks 1 because a leaked slot
is not a missing sound, it is a budget that never comes back — the failure `cues.yaml` cases 20 and 21 were written to
catch, arriving by a route they do not construct.

**Evidence.** `runs/2026-11-02-17/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/backend{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `Backend(true)`, no `Output` | **`AudioPlayer 0`** — silent, unretirable | `AudioPlayer 1`; `Played` answered |
| `Backend(true)`, with `Output` | `AudioPlayer 0` | **unchanged** — the guard row |
| `Backend(false)`, no `Output` | `AudioPlayer 1` | unchanged |
| a steal, both paths | identical `SirenLog` | unchanged |
| a duck, both paths, 90 frames | identical `bus_db`/`ducked`, min −8.000 dB | unchanged |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost | 0.064 / 0.351 / 0.734 µs | 0.077 / 0.370 / 0.713 µs, load 8.25/9.79/14.97 — within noise, and off the path `perfcheck` costs |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One test defect caught at Fresh eyes: the fallback test
asserted "no `AudioPlayer`" without removing the `Output` the plugin inserts when the `mixer` feature opens a device —
a test whose precondition is "no `Output`" must say so.

**Traps for the command.** (1) A **flag** is not a **fact**: `Backend(true)` says "a mixer started", and a resource a
host can set is not evidence that it did. Read the resource the claim is about. (2) A voice that cannot sound cannot
**end**, so it cannot be **retired**: silent loss and a leaked budget slot are one bug, and the leak is the worse half.
(3) When a test's precondition is "this resource is absent", remove it — with a feature on, the plugin may have
inserted it before the test ran. (4) The guard half of a fix (`AudioPlayer 0` with an `Output`) must be its own test,
or a fix that always inserts a player passes the other one. (5) `hs distill` GPU-blocked (sixteenth iteration).

**Commit.** `26b676b`, pushed to `main`; CI run 41572684531 is **success** on `big-s1` (fmt + clippy + doc, then the
standalone build and test). The Cauldron's `audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test
(84 + 4), serde (91 + 4), mixer (92 + 4), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md` (iteration 16's item 1).
2. **Re-time a restarted stem to the bar** — `needs-decision/restart-on-the-bar.md`.
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, sixth pass.** The **two-backend** class is clean for steal and duck, and this iteration closed the "answers
nothing" case. The untested surface left is the **`AudioOutput` trait's own contract**: `set_voice_db` is called by
`update_mix` on **every** voice each frame, but nothing proves a backend that ignores it (as `FakeOutput` does) cannot
desynchronise `Applied`. Next hunt: what `mix.rs`'s `Applied` records versus what the backend was actually told.

**Baseline note.** Nothing was red; the suite had 83 tests.

---

## Iteration 16 — 2026-11-02 — a layer that names a cue which ends

**Change.**

- **`src/music.rs` — `start_layers` does not start a layer whose cue is not a lock `Mode::Loop`.** `MusicScore::problems`
  states the rule (*"names cue `hit`, which is not a loop, so it cannot hold a layer"*); the plugin ignored it. A
  `Mode::Once` cue was started, its voice was retired — a once voice **ends**, and `retire_voices` only skips loops —
  and iteration 15's restart rule asked for it again **every frame**: 10 plays in 10 frames, no `Refused`, no line.
  `start_missing` now reads the cue the layer names **this frame**, skips a cue that is not a loop, keeps the layer in
  `names()`, and writes **one** line in `problems()`' own words. A table edit that makes the cue a loop starts the
  layer on the next frame.
- `tests/music.rs` — three tests, all failing on the base commit; a *"A layer must name a cue that loops"* section; a
  CHANGELOG line; and `docs/loop/needs-decision/not-a-loop-refusal.md` for the `Refusal` variant this does **not** add.

**Why.** The hunt at the end of iteration 15 pointed at the **consumer-side** resources. Probing `SirenChoices` found it
**sound** — a fresh `SirenChoices::new(seed)` reproduces the stream, re-writing the same seed continues it, resetting
the public `rng` alone reproduces it. Probing the rest found three green and one rank-1 red: the asymmetry that made it
rank 1 is that an **unknown** cue was refused with a reason and a **one-shot** cue played at frame rate with no line at
all.

**Evidence.** `runs/2026-11-02-16/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/one-shot-layer{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| a `Mode::Once` layer, its sample ending | **10 plays in 10 frames**, `live()` 1, no line | **0 plays**, `live()` 0, one line naming layer and cue |
| a `Mode::Loop` layer (control) | 1 play, `live()` 1 | unchanged |
| a layer naming an unknown cue (control) | refused, `no cue of that name` | unchanged |
| the table fixed to `Mode::Loop` | never starts | starts on the next frame |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost | 0.064 / 0.351 / 0.734 µs at 8/64/256 | unchanged — one table lookup per **voiceless** layer, no per-voice work |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. One defect in my own measurement caught at Zoom: the first
probe despawned the layer's voice every frame **for every row**, so the loop control also read 10 plays and proved
nothing; an end is now modelled only for a one-shot.

**Traps for the command.** (1) A restart rule turns *silence* into *rate*: once iteration 15 made a dead voice restart,
any cue that ends became a frame-rate retrigger. A retry must know what "ended" means for the thing it retries.
(2) A control row is only a control if the **stimulus** is withheld from it — the first probe withheld nothing. (3)
`problems()` and the plugin are two paths to one rule; when they disagree, the plugin is the one that plays.
(4) Clippy's 7-argument limit on a Bevy system is Bevy's, not the design's: `play_requests`, `answer` and
`start_pending` already carry the allow. (5) `hs distill` still GPU-blocked (fifteenth iteration).

**Commit.** `ad55c66`, pushed to `main`; CI run 41569616373 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (83 + 4), serde (90 + 4), mixer (90 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **`Refusal::NotALoop { layer, cue }`** — `needs-decision/not-a-loop-refusal.md`; the log line is the enforcement
   today, and a host reading `Refused` alone cannot see it.
2. **Re-time a restarted stem to the bar** — `needs-decision/restart-on-the-bar.md` (iteration 15's item 1).
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
5. Quantise a **retarget** onto the bar (iteration 14's item 1).

**The hunt, fifth pass.** The **resource-contract** class is now exhausted in two directions: what a public resource
*holds* (13–15) and what a consumer resource holds about the crate (this one — `SirenChoices` **sound**). The next
untested surface is the **two-backend** promise: "the two backends answer identically", which `tests/mixer.rs` proves for
play and stop. Nothing proves it for a **steal**, a **duck**, or a **music layer**.

**Baseline note.** Nothing was red; the suite had 80 tests.

---

## Iteration 15 — 2026-11-02 — a music layer's voice dies, and now the layer knows

**Change.**

- **`src/music.rs` — `MusicState` reconciles its voices every frame.** It wrote the entity a layer's `Played`
  reported and never looked again, so a stem stopped by the game, stolen by the budget, or refused at its first play
  left `voice(name)` naming a **despawned** entity, `live()` counting it, and the stem silent for the process's life.
  Three rules now: a dead voice is cleared; a stem that died behind the plugin's back is restarted at the layer's own
  gain; a `StopCue` stays stopped and reads `SILENT_DB`; a refused first start is retried **when a voice frees**.
- **`src/play.rs` — a music layer is unstealable whatever its cue authored.** `Voice::unstealable`'s doc says *"music,
  stingers and dialogue are not stolen"*; the mechanism was `priority >= UNSTEALABLE_PRIORITY`, so a layer authored with
  the default `priority: 0` was called music and treated as a footstep. The budget reads the `MusicVoice` the plugin
  already inserts; the newcomer is refused.
- `tests/music.rs` — four tests, all failing on the base commit; a *"When a stem's voice dies"* section and a
  `CHANGELOG.md` line.

**Why.** Iteration 14's advice was to hunt `SirenSettings`; the hunt found a **contract class** one level up — a
resource that mirrors another system's entities. Iteration 12 was one path of it (a handle that outlived its voice);
this is the mirror itself. The worst row is rank 1: a **stem silently stolen by a UI click**, which R4 and the crate's
own research say must never happen.

**Evidence.** `runs/2026-11-02-15/`: `eval.md`, `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/layer-voices{,-before}.txt`.

**Measured.**

| row | before | after |
|---|---|---|
| `StopCue` on a stem | `voice()` `Some(47v0)` — **despawned**; `live()` 1 | `None`, `live()` 0, `layer_db` −80 dB |
| `ui.click` beside a `priority: 0` stem, `max_voices: 1` | `voices ["ui.click"]`, **`track.calm stolen by ui.click`** | `voices ["track.calm"]`, no steals; the click refused |
| a stem whose voice died | `Some(47v0)` → `Some(47v0)`, still dead, `live()` **1** | `Some(47v0)` → `Some(48v0)`, `live()` 1 |
| a refused first start | `live()` **0** forever | `live()` 1; refusals **1 → 1** |
| every rendered cue, the mix | — | **byte-identical** (21/21); `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.135 µs | 0.064 / 0.351 / 0.734 µs |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two defects caught in my own work: the retry compared
against the count of *pending intents* instead of *voices* (so it never retried — probe 27 caught it), and a
game-stopped layer kept reporting the gain of its last playing frame.

**Traps for the command.** (1) A mirror of another system's entities must be reconciled **every frame**; a `voice:
Option<Entity>` is a claim the world can falsify without telling you. (2) Read *why* a voice is stopping while
`Stopping` is still on it — it despawns with the entity, and a frame later the reason is gone. (3) `voice: Option` is
not truthiness: `live()` counted the dead. (4) A retry condition must count the resource it waits for, not the queue
that asks for it. (5) The Cauldron panel test needs ~14 GB; disk hit 99 % and it failed with `ld: errno=28` — the skill's
own warning. `hs distill` is GPU-blocked (fourteenth iteration).

**Commit.** `fd85856`, pushed to `main`; CI run 41566251355 is **success** on `big-s1`. The Cauldron's
`audio_panel_reads_the_crates_file` passes (4 tests). Gate: build, test (80 + 4), serde (87 + 4), mixer (87 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **Re-time a restarted stem to the bar.** A restart is a new play from sample zero and is out of phase. Needs a
   decision about what a restart means after a gain change: `needs-decision/restart-on-the-bar.md`.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A `Refusal` variant separating "never mapped" from "corrupt" — `needs-decision/undecodable-sound.md`.
4. Quantise a **retarget** onto the bar (iteration 14's item 1).
5. A `Refusal` payload that names the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, fourth pass.** The **resource-contract** class has now paid three times: `max_instances`'s sentence (13),
`MusicScore`'s edits (14), `MusicState`'s mirror (15). The remaining unprobed surface of it is the **other direction** —
what a *consumer* resource holds about the crate: `SirenSounds`, `SirenChoices`, and the `Output`/`Backend` a host
installs. `SirenChoices` is a `Resource` a project may reset; check what resetting it mid-stream does to a determinism
promise.

**Baseline note.** Nothing was red; the suite had 76 tests. No case in `cues.yaml` stops a layer's voice or authors a
layer with the default priority.

---

## Iteration 14 — 2026-11-02 — a score edited at runtime reconciles its voices

**Change.**

- **`src/music.rs` — `start_layers` reconciles a changed score's voices, not only its layer list.** A layer **removed**
  from the score had its looping voice left playing forever, ownerless (`retire_voices` skips loops by design); a layer
  **retargeted** to a new `cue` kept the old sample while the new one never started. Both now stop through the crate's
  one fade, marked `Stopping` with the true reason (`"layer removed"` / `"layer retargeted"`), so the log says what the
  score did; a retargeted layer starts its new cue the same frame. A **gain** edit keeps the voice.
- **`MusicState::names()`'s documented score order is restored** after the reconcile — my first version dropped and
  re-appended a retargeted layer and returned `["combat", "calm"]`; the gate caught it.
- `tests/music.rs` — three tests (removal, retarget, gain-keeps-the-voice), two of which fail on the unfixed tree; an
  "Editing the score" section in the music guide; the CHANGELOG and a media note.

**Why.** Iteration 13 named the class that keeps paying: what a **public resource's** contract promises. `MusicScore` is
documented as a file *"a project ships and the Cauldron edits"* — a value that changes at runtime — and the plugin only
ever added to it. Two edits left the score and the sound disagreeing in silence, and one of them (the retarget) plays
the **wrong sample indefinitely**.

**Evidence.** `runs/2026-11-02-14/`: `eval.md` (baseline green; no case edited a score at runtime), `research.md`,
`plan.md`, `perf.md` (the two probes), `review.md`, and `evidence/{before,after}/`.

**Measured.**

| edit | before | after |
|---|---|---|
| remove a layer | `names = ["combat"]`, **2 voices live**, no line | 1 voice, `stopped … layer removed` |
| retarget a layer | the score says `track.b`; **`track.a` plays** and `track.b` never starts | only `track.combat` live; two lines |
| change a gain | eases | same entity, no restart |
| `names()` | `["combat", "calm"]` on the first attempt | `["calm", "combat"]`, score order |
| every rendered cue, the mix, `examples/music` | — | **byte-identical**; *"transitions on the bar: yes"* |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.135 µs | the reconcile runs only on a frame the score changed |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. Two defects found by the gate in my own work: the first fix
logged every stop as `"stop"` (a `StopVoice` carries no reason), and the second broke `names()`'s documented score order.

**Traps for the command.** (1) `StopVoice` carries an **entity and no reason**, so a system that knows *why* it is
stopping something must mark `Stopping` itself rather than route through `stop_requests` — otherwise the log says
`"stop"` and the reason is lost. (2) A reconcile that drops and re-adds an entry silently changes **order**; check every
accessor that documents one. (3) A `retain` that only filters the state is half a reconcile when each entry owns a
voice: the other half is what the voice does. (4) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB
VLLM engine holds the card), twelfth iteration running.

**Commit.** `0bff015`, pushed to `main`; CI run 41560011538 is **success** on `big-s1`. Gate: build, test (76 + 4), serde (83 + 4), mixer (83 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **Quantise a retarget onto the bar.** A cue change restarts immediately; `When::NextBeat` exists for gain changes.
   Needs a decision about the gap between the old stem's fade and the bar.
2. A method to reconcile a cue's live voices with its cap (iteration 13's item 1; new API, decision first).
3. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
4. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
5. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.

**The hunt, third pass.** The handle class is exhausted (iteration 12 closed the only unguarded path; iteration 13 ran
the rest). The **resource-contract** class has now paid twice: `max_instances`' sentence, and `MusicScore`'s. The next
hunt should look at the third resource a project may replace — `SirenSettings` — for a field whose sentence promises
more than the frame does.

**Baseline note.** Nothing was red, and the suite had 73 tests. Cases 15 and 23 drive the score; neither edits it.

---

## Iteration 13 — 2026-11-02 — the cap is a budget on plays, and the docs say so

**Change.**

- **`src/cue.rs` — `Cue::max_instances` now says "may **start**", and why.** It read *"how many of this cue may sound at
  once"*, a standing promise; the crate keeps it at the moment of each play. A table edited under live voices leaves
  them sounding and refuses the next request. `CueTable::insert` — the method a project calls to edit one at runtime —
  points at the same sentence, and `docs/guide/buses.md` says it in a project's words: all three caps are budgets on
  plays, and editing a table under playing sounds is safe.
- `tests/mix.rs` — two tests hold the reading: a lowered cap refuses and does not cut; a raised cap admits. The mutation
  that would adopt the other reading (removing `room_for`'s unstealable guard, the shape a "reconcile by ending"
  implementation has) reddens the first. **No behaviour changed.**

**Why.** Iteration 12's log named the hunt: hold a public handle across a frame and use it as documented. This pass ran
it over every remaining carrier — `Play::on` in both despawn orders, `MusicState::voice`, `SirenChoices`, the `Samples`
handle — and all were safe; the audit of every `commands.entity(..)` confirms iteration 12 closed the only unguarded
path. What the hunt did find was a **documented promise with two readings**, and the written one was the one the crate
does not implement.

**Evidence.** `runs/2026-11-02-13/`: `eval.md` (baseline green; case 7 asserts the cap only at play time), `research.md`,
`plan.md`, `perf.md` (the probe and the mutation), `review.md`, and `evidence/{before,after}/` with both render trees, the
spectrograms and the loudness numbers.

**Measured.**

| sequence | observed | documented now |
|---|---|---|
| two voices at `max_instances: 2`, table edited to 1, third play | refused `VoiceLimit`; **both voices still sounding**; nothing stopped or logged | the cap governs the next play; live voices live out their length |
| cap raised from 1 to 2 under one live voice | the next play is `Played` | unchanged |
| the rejected design | removing `room_for`'s unstealable guard reddens the new test | measured |
| every rendered cue, the mix, and `examples/music` | — | **byte-identical**; *"transitions on the bar: yes"* |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found two defects in my own tests — priority-0
voices meant the lowered cap *stole* rather than refused, and a second test read a `Played` left buffered by the previous
play — both the loop's old traps, both fixed.

**The alternative, rejected with a reason.** Reconciling live voices would despawn a voice the game did not stop, on a
frame it did not choose, and log a stop it did not request: three surprises to replace one, plus rules for which voice
goes and for a table edited twice in a frame. A feature with its own decision, not a doc fix.

**Traps for the command.** (1) A cap that can **steal** never refuses: to test a refusal, the voices must be unstealable
or the test asserts the wrong path. (2) Drain `Played`/`Refused` between plays, or the next assertion counts the
previous request's message. (3) Two readings of one sentence can both be defensible; the repair is to write down which
one the code takes and hold it with a test, not to change the code to match the looser reading. (4) `hs distill` is still
GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), eleventh iteration running.

**Commit.** `c53f030`, pushed to `main`; CI run 41559273528 is **success** on `big-s1`. Gate: build, test (73 + 4), serde (80 + 4), mixer (80 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.**

1. **A method to reconcile a cue's live voices with its cap**, for a project that wants the count enforced — as
   something a game calls, never as a frame the crate chooses. New public API, so a decision file first.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
4. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
5. `Completes` is public and can never be true — `needs-decision/remove-completes.md` (its `Entry` arm can carry item 1
   of iteration 10's note).

**The hunt, after two passes of it.** Iteration 12 found a crash by holding a handle across a frame; this pass ran the
same shape over every other carrier and found nothing, then found a contract gap by asking what a **public field's
sentence** promises. Both hunts are now written into the command: the handle class is exhausted, and the contract class
is the one that keeps paying.

**Baseline note.** Nothing was red, and the suite had 71 tests. Case 7 asserts the polyphony cap only at play time; no
case edits a table under a live voice.

---

## Iteration 12 — 2026-11-02 — a stop for a sound that already ended

**Change.**

- **`src/mix.rs` — `stop_requests` resolves the entity against the live voice query before commanding it.** It took the
  id straight from the `StopVoice` message and called `commands.entity(entity).insert(..)`; `retire_voices` despawns a
  one-shot the frame it ends, and `Played::entity` is documented as the handle a game keeps **for `StopVoice`** — so
  holding it one frame too long hit Bevy 0.20's default error handler, which **panics**:
  `Entity despawned: … its index now has generation 1`. Now a stop for a voice that has gone is a no-op, and a second
  stop for a voice already fading is skipped so a fade is begun once.
- `tests/play.rs` — three tests: the stale handle (**panics inside `app.update()`** on the unfixed tree), a second stop,
  and `StopCue` so the fix cannot narrow the name path. `cues.yaml` case 30; a "Stopping a sound" section in the cues
  guide; the CHANGELOG and a media note.

**Why.** Iteration 11's log said the most productive next pass was **a new cue case that turns something red** — and one
did, in the first probe. This is a crash reachable from the crate's own documented workflow and its own retirement path:
`Played` hands out an id, the crate invalidates it on its own schedule, and using it as documented took the game down.
Every other `commands.entity(..)` in the crate takes an entity from a live query and cannot be dead; this was the one
path where the id came from a **message**.

**Evidence.** `runs/2026-11-02-12/`: `eval.md` (baseline green; the gap — case 23 stops a live loop, no case stops an
ended voice), `research.md`, `plan.md`, `perf.md` (the panic and its absence), `review.md`, and `evidence/{before,after}/`.

**Measured.**

| sequence | before | after |
|---|---|---|
| play a one-shot, let it end, `StopVoice(handle)` | **panic: `Entity despawned … generation 1`** | no-op; nothing logged |
| play a loop, `StopVoice` | fades | unchanged |
| `StopVoice` twice for a fading voice | fade re-armed | skipped; one fade, one line |
| `StopCue` with two voices | both stopped | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.069 / 0.628 / 1.391 µs | one `Query::get` per stop request, an event not a frame cost; load 8.0 vs 2.4 is the difference from iteration 11 |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The gate found two defects in my own work — a leftover import
and a scaffold expression in a test — both fixed; and the commit itself shipped a throwaway probe by mistake, removed in
the commit that follows it (`975f12b`).

**Traps for the command.** (1) **Check `git status` before committing after a hunt**: `examples/probe16.rs` was deleted
from the working tree but staged with `git add -A`, and an example directory is part of the published crate. (2) A
command on an id that came from **outside** the world needs a guard; a command on an id from a **query** does not. Grep
`commands.entity(` and check where each entity came from — that is the whole audit. (3) The regression test's value is
that it **panics**, not that it asserts a value: the unfixed tree fails inside `app.update()`, which no assertion can
express. (4) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), tenth
iteration running.

**Commit.** `d551abb` + `975f12b`, pushed to `main`; CI run 41558456518 is **success** on `big-s1`. Gate: build, test (71 + 4), serde (78 + 4), mixer
(78 + 4), doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.** The five decision files are unchanged; one minor can carry four of them.

1. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
2. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
3. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
4. `Completes` is public and can never be true — `needs-decision/remove-completes.md`.
5. A log line for a position a non-spatial cue ignores — `needs-decision/non-spatial-position.md` (its `Entry` arm can
   land with item 4).

**The hunt that worked, written down.** Iteration 11 found nothing in six probes because it tested what the crate
*decides*; this one found a crash in the first probe because it tested **what a game does with what the crate hands it** —
keep the entity, use it later. That is the class of case the table was missing, and the next hunt should start there:
take a public handle, message or resource, hold it across the frame the crate changes it, and use it as documented.

**Baseline note.** Nothing was red, and the suite had 68 tests. Case 23 stops a **live** loop; nothing stopped a voice
that had **ended**, which is the only sequence that reaches the panic.

---

## Iteration 11 — 2026-11-02 — the rule written the way it reads

**Change.**

- **`tests/mix.rs` — two tests state `UNSTEALABLE_PRIORITY` directly.** It was proved through
  `the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue. Now a **priority-0**
  request against a **priority-200** voice is refused with nothing spawned, the voice still live and no `stolen by`
  line; and a stealable voice still plays beside an unstealable one, so neither can pass vacuously.
- `cues.yaml` case 9 names the stealable-request direction as its observation and cites Gregory §14.5.8.4;
  `CHANGELOG`, media note. **No `src/` file changed at all.**

**Why.** Two things, and the second is the honest one.

First, `DuckClock.started` — iteration 10's next-5 head — **is not a bug**: measured, 50 fires over two rules leave two
entries, because the map is keyed by rule, not by fire, and `duck_db` reads only pairs still in `settings.ducks`.

Second, with that closed and five decision-blocked items left, this pass tested **six** candidate rank-1 defects and
found none: the map above; a stop on the play's own frame (still fades — `update_mix` writes `Applied` before
`stop_requests`); `Played.gain_db` against the audible chain (−11.00 dB both); a stealable request taking an unstealable
voice; a bus cap above the table's (clamped); `SirenLog` at capacity. What remained was legibility, and that is what
shipped.

**Evidence.** `runs/2026-11-02-11/`: `eval.md` (baseline green; the six candidates and their measurements),
`research.md` (**corrected mid-run** — my first premise was wrong), `plan.md` (corrected the same way), `perf.md` (the
mutation), `review.md`, and `evidence/{before,after}/` with the render trees, spectrograms and loudness.

**Measured.**

| candidate | result |
|---|---|
| `DuckClock.started` grows with table swaps | **two** entries after 50 fires over two rules |
| a stop on the play's own frame cuts | fades for 0.48 s; `Applied` is written before `stop_requests` |
| `Played.gain_db` vs the audible chain | −2 −3 −6 = **−11.00 dB**; `Applied` −11.0 |
| a stealable request takes an unstealable voice | refused; nothing stopped |
| a bus cap above the table's | clamped with `.min(total)` |
| `SirenLog` at capacity | keeps 256, drops the oldest |
| **the mutation: `steal` ignores `unstealable`** | **three** tests red — both new ones **and** the old table-budget test |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT — for what this is: a legibility change plus six negative
results, not a repair.

**A premise corrected by measurement.** I wrote, and briefly believed, that the unstealable promise had an untested
half. The mutation disproved it in one run: the existing test reddens too, because its second request then steals its
own older voice. `research.md` and `plan.md` carry the correction rather than the original claim, because the loop's
own rule is that a claim without a measurement is a guess.

**Traps for the command.** (1) A mutation is the cheapest way to find out whether a test covers a rule — and it can
disprove the reason you wrote the test. Run it *before* claiming the gap. (2) A test-only iteration still owes the
evidence rules: the `before` tree here is the after-tree's binary, because `src/` is unchanged, and `perf.md` says so
instead of presenting a lower number (load 2.4 against 4.7) as an improvement. (3) `hs distill` is still GPU-blocked
(`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), ninth iteration running.

**Commit.** `ef2f91b`, pushed to `main`; CI run 41557694429 is **success** on `big-s1`. Gate: build, test (68 + 4), serde (75 + 4), mixer (75 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green.

**Next 5.** All five are public-enum decisions for the owner; one minor can carry them together.

1. **A `Refusal` variant for a stinger on the wrong bus** — `needs-decision/stinger-refusal.md`.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **A log line for a position a non-spatial cue ignores** — `needs-decision/non-spatial-position.md`, whose `Entry` arm
   could land with item 4.

**A note on the loop's run.** Eleven iterations have taken the crate from "a 32-voice budget that silently locks out on
the mixer path" to "every rank-1 defect the suite and the probes can find is closed". The remaining work is not
engineering the loop can do alone: four of the five are one `match` arm each, and the fifth is one field. The next
iteration should start with the owner's answer to any of them, or with a new cue case that turns something red.

**Baseline note.** Nothing was red, and the suite had 66 tests. Six candidates were probed and none was a defect, which
is the finding this entry records.

---

## Iteration 10 — 2026-11-02 — the mix's buses are this frame's buses

**Change.**

- **`src/mix.rs` — `update_mix` iterates the buses `bus_names()` returns, not `settings.buses`.** With `buses: []` the
  crate accepted a cue on `Effects` (that is what `answer`, the panel and `bus_names()` all promise) and then mixed it at
  **−80 dB** — silence with no refusal, on the crate's own default-shaped table.
- **`src/mix.rs` — `SirenMix::db` and `ducked` are pruned to this frame's buses** before the bus loop. A project that
  swapped `SirenSettings` kept every bus from the previous table at its last level: `bus_db` answered for a bus that did
  not exist, `deepest_duck()` named one forever, and a removed bus's duck froze at its deepest. The prune runs before the
  loop, so a bus that **stayed** still reads its previous value and eases instead of steppng.
- `tests/mix.rs` — three tests: the swap (fails on the unfixed tree), the empty list (fails on the unfixed tree), and the
  non-regression a wholesale rebuild would break. `examples/render` prints what `bus_names()` answers for a table and for
  a table of none; `cues.yaml` case 29; the buses guide, the CHANGELOG and a media note.

**Why.** The five next-5 items are all public-enum decisions the owner has not answered, so the pass hunted for a
rank-1 defect instead of idling — and the second half turned up while testing the first, which is the loop's own point
about a test that measures the thing rather than a plausible neighbouring thing. `buses: []` at −80 dB is wrong output,
silent, with no refusal: the exact class this loop ranks first.

**Evidence.** `runs/2026-11-02-10/`: `eval.md` (baseline green; the table's gap — no case swaps the table or uses `[]`),
`research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both render trees, the bus-list line,
the spectrograms and the loudness numbers.

**Measured.**

| measurement | before | after |
|---|---|---|
| `mix.db` after a 4→2 bus swap | all four, ghosts included | `[Effects, Music]` |
| `bus_db("Ambience")`, a bus not in the table | **−3.3 dB** | **−80.0 dB** (`SILENT_DB`) |
| `deepest_duck()` | `Some(("Ambience", −12.0))` forever | `None` |
| `buses: []`, a cue on `Effects` | accepted, **mixed at −80 dB** | accepted, mixed at 0 dB on the crate's four |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.065 / 0.354 / 1.389 µs | two map prunes over ≤ the bus count; not on the per-voice path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The gate found two defects in my own work — a `dt = 0` in a
test of mine (the crate's documented trap, in a test written two iterations after writing it down) and a dead-code
warning on a `mixer`-only helper — both fixed before the verdict.

**Traps for the command.** (1) Disk reached **674 MB** and `ld` failed with `errno=28` — the loop's documented trap,
third time in this project. 15 GB came back from build directories this loop created (`/tmp/seedprobe` 6.6 GB,
`/tmp/siren-pub` 7.4 GB), never from the project's `target/`. (2) A `BTreeMap` written by insertion is a cache with no
owner: `voices` was already rebuilt wholesale, and these two maps beside it were the exception, which is why only they
went stale. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), eighth
iteration running.

**Commit.** `29a431c`, pushed to `main`; CI run 41557018186 is **success** on `big-s1`. Gate: build, test (66 + 4), serde (73 + 4), mixer (73 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `bus_rows` iterates the table's buses, so it was never shown the ghosts — the crate's own accessors
were.

**Next 5.**

1. **`DuckClock.started` has the same staleness** as the mix maps had: keys are `(source, target)` pairs and a pair whose
   buses left the table is inert but never removed. Cheap to prune beside the maps.
2. A `Refusal` variant for a stinger on the wrong bus — `needs-decision/stinger-refusal.md`.
3. A variant that distinguishes "never mapped" from "corrupt file" — `needs-decision/undecodable-sound.md`.
4. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`.
5. `Completes` is public and can never be true — `needs-decision/remove-completes.md`.

**Baseline note.** Nothing was red, and the suite had 63 tests. Every case used the **default** table; none swapped it
or listed no buses, so the two places that disagree about which buses exist were both invisible.

---

## Iteration 9 — 2026-11-02 — a stinger's duck waits for a sound that plays

**Change.**

- **`src/music.rs`, `src/lib.rs` — the duck is armed on `Played`, not on the request.** `play_stingers` wrote the `Play`
  and armed the duck in the same breath while `Play` is answered later in the frame, so a refused stinger left the
  `Music` bus down for a sound nobody heard — and the log said both. A new `arm_stinger_ducks` runs after `Play` and
  before `Mix`, arming the duck for a stinger that was answered and dropping one that was refused; the dip still lands
  on the frame the stinger fires.
- **A cue not in the table is refused as `UnknownCue`**, instead of inheriting `Music` through `unwrap_or("Music")` and
  passing the bus check. **Stinger refusals write a `Refused` message**, not only a log line.
- `tests/music.rs` — four tests: unknown cue, missing sound (both fail on the unfixed tree), a real stinger ducking on
  the frame it fires, and a wrong-bus stinger. `examples/render` gains the unknown-cue refusal; `cues.yaml` case 28;
  `needs-decision/stinger-refusal.md`; the music guide, the CHANGELOG and a media note.

**Why.** Rank 1 at its sharpest: a duck that corresponds to no sound. The log was the proof — *"stinger ducked Music by
-9.0 dB"* on the line above *"refused … no handle …"* — two records of one request disagreeing, which is exactly what
the crate's refusals exist to prevent. Gregory §14.6.3 and the crate's own R3 both say a duck is driven by a voice that
starts; the dialogue path already obeyed that, and stingers were the one exception.

**Evidence.** `runs/2026-11-02-09/`: `eval.md` (baseline green; case 16 proves a stinger ducks and no case asks what
happens when it cannot), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the new refusal file, the spectrograms and the loudness numbers.

**Measured.**

| case | before | after |
|---|---|---|
| cue not in the table | **Music −9.00 dB**, `UnknownCue`, log says it ducked | Music 0.00 dB, `UnknownCue`, **one** line |
| a sound with no handle | **Music −9.00 dB**, log says ducked **and** refused | Music 0.00 dB, refusal only |
| a stinger that plays | −9.0 dB on its frame | unchanged |
| a stinger on another bus | refused, no duck | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.063 / 0.367 / 1.347 µs | not on this path; one empty `Vec` and two drained buffers per frame |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass caught a regression I nearly shipped: the
wrong-bus branch briefly reused `Refusal::UnknownBus`, whose sentence (*"bus `Effects` is not in the mix table"*) is
**false** — `Effects` is in the table. Reverted to the true prose line, and the variant that would say it properly is
`needs-decision/stinger-refusal.md`.

**Traps for the command.** (1) A bug with two independent halves needs **both** reverted to prove the tests: reverting
only the ordering left the unknown-cue test passing, because the table lookup half still refused it. (2) `render.txt`
and the rendered audio cannot show an ordering fix — the panel settles to the same value once the frame is over; the
tests are the evidence, and the log is the *symptom*. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB;
a 14.8 GB VLLM engine holds the card), seventh iteration running.

**Commit.** `64fce41`, pushed to `main`; CI run 41556188200 is **success** on `big-s1`. Gate: build, test (63 + 4), serde (70 + 4), mixer (70 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; the panel prints `SirenLog` lines, so a stinger that cannot play now shows one line where it showed
two — an improvement it gets without a consumer change.

**Next 5.** Four of the five are now public-enum decisions that one minor can carry together:

1. **A `Refusal` variant for a stinger on the wrong bus** — `needs-decision/stinger-refusal.md`.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **A log line for a position a non-spatial cue ignores** — `needs-decision/non-spatial-position.md`, whose `Entry`
   arm could land with item 4.

**Baseline note.** Nothing was red, and the suite had 59 tests. Case 16 proves the duck happens; nothing asked what
happens when the stinger cannot play, which is why the contradiction sat in the log unread for six iterations.

---

## Iteration 8 — 2026-11-02 — a non-spatial cue is never handed to the panner

**Change.**

- **`src/play.rs` — one predicate decides placement.** `spawn_voice` set Bevy's `PlaybackSettings.spatial`, and
  attached a `Transform`, from `request.position.is_some()` alone, while `answer` applied attenuation under
  `cue.spatial && position.is_some()`. A cue authored `spatial: false` and played with `.at(..)` therefore went through
  `rodio`'s panner, which applies `min(1, 1/d²)` as well as a pan: **−38.7 dB at 8 m**, while `Played.gain_db` read
  **0.00 dB**. `Started.spatial` now carries the resolved fact and all three places read it.
- `tests/play.rs` — four tests: the mismatch (fails on the unfixed tree), and the three cases it must not overshoot
  (spatial at a position, spatial without one, spatial without a listener). Two `PlaybackSettings` assertions are gated
  to Bevy's path, because the `mixer` build has no settings component — the two backends must answer identically, and
  the test had only been testing one. `tests/model.rs` pins the panner's own law at 0.0117 linear / −38.7 dB.
- `examples/render.rs` — a `panner:` line giving that law in dB; `cues.yaml` case 27; the spatial guide, the CHANGELOG
  (with who could notice) and a media note; `needs-decision/non-spatial-position.md`.

**Why.** Rank 1 — wrong output — and its worst sub-class: a `Played` whose own `gain_db` disagrees with the sample, so a
game's meter believes a sound is at 0 dB while it plays 38 dB down. The crate's guide already states the principle the
code broke: *"playing it flat, or at the origin, is a different sound from the one the game asked for."* The three
higher-listed next-5 items are all decision-blocked, so this was the only rank-1 item buildable today.

**Evidence.** `runs/2026-11-02-08/`: `eval.md` (baseline green; the fourth combination of the spatial table not covered),
`research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both render trees, the panner line,
the spectrograms and the loudness numbers.

**Measured.**

| request | before | after |
|---|---|---|
| non-spatial cue, `.at(8 m)` | panned, **−38.7 dB audible**, `Played.gain_db` 0.00 | flat at its own gain, no `Transform`, no panner |
| spatial cue, `.at(8 m)` | −6.02 dB, compensated, placed | unchanged |
| spatial cue, no position | flat at its own gain | unchanged (documented) |
| spatial cue, no listener | `SpatiallyMute` | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.063 / 0.354 / 1.343 µs | not on this path; one `bool` computed once per play |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass caught a real defect in my own tests: both
`PlaybackSettings` assertions panicked under `--features mixer`, where the crate's own graph owns playback and there is
no settings component. Gated, and the rule that the two backends answer identically is now honoured by the test rather
than assumed.

**A number worth keeping.** The panner's own law at 8 m on-axis is **0.0117 linear, −38.7 dB** — computed from
`model::backend_spatial_gain`, printed by `examples/render`, and pinned by a test. That is the size of the silence a
"flat" cue was being handed while the log said 0 dB.

**Traps for the command.** (1) `render.txt`'s `panner:` line reads the same in both trees **by design** — it computes the
law, not the routing decision. It is evidence of magnitude; the test is evidence of the fix; confusing them is exactly
the trap iteration 7 hit. (2) A test that reads `PlaybackSettings` only compiles and passes on Bevy's path; under
`--features mixer` it panics on a missing component, which is not a crate bug. (3) `hs distill` is still GPU-blocked
(`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the card), sixth iteration running.

**Commit.** `f75c633`, pushed to `main`; CI run 41555323150 is **success** on `big-s1`, and so is the docs commit that follows it. Gate: build, test (59 + 4), serde (66 + 4), mixer (66 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `PlaybackSettings` is not something the panel reads, and `Played.gain_db` is now true rather than
false for this case, which can only improve what it prints.

**Next 5.**

1. **A log line for an ignored position** — `needs-decision/non-spatial-position.md`, option 2, which shares its `Entry`
   arm with `remove-completes.md`. One edit can land both.
2. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`.
3. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
4. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
5. **The mixer's decode error is discarded** (`Decoder::new(..).ok()?`); carrying it into the refusal would say *why* a
   file failed. Same decision as item 2.

**Baseline note.** Nothing was red, and the suite had 54 tests. Case 14 proves a spatial cue at two distances and case
13 the no-listener refusal; no case reached the fourth combination — `spatial: false` **with** a position — which is the
one that lied.

---

## Iteration 7 — 2026-11-02 — a voice slot is reused, not appended

**Change.**

- **`src/mixer/mod.rs`, `src/model.rs` — the mixer's per-bus slot list is bounded.** `play` pushed a `VoiceTrack` (a
  `rodio::Player` and its queues) per play and never removed one, so a game on the crate's own mixer retained a player
  for every sound it had ever played. It now reuses the first slot whose voice has ended. An index packs a **slot and
  a generation** (`model::pack_voice`, `unpack_voice`, `VOICE_SLOT_BITS`), bumped on each hand-out, and every
  index-taking method goes through one `occupant_mut` so a stale `MixerVoice` is a no-op rather than a cross-wire.
  `MixerBackend::voices()` is documented as *slots*, not sounds played. **No public type or trait signature changed.**
- `tests/mixer.rs` — a test that drives the **real** backend through 200 one-shots and asserts slots bounded, samples
  started and 200 distinct names *together*; it returns early with a sentence where no device exists. `tests/model.rs`
  pins the encoding at both range ends. `tests/helpers.rs` gains a real 64-sample WAV.
- `cues.yaml` case 26; the buses guide, the CHANGELOG and a media note.

**Why.** Iteration 2 found this and deferred it ("a memory leak of one `Player` handle per one-shot … what a second
iteration's 'compact the voice list' item is for"). Five iterations later it was still there, and it is the one cost in
the crate that grows with how long a game is played: the budget bounds **concurrent** voices, and this list counted
**cumulative** ones. Gregory §14.5.8.4 calls the voice count fixed; here it was not. The other rank-1-class items are
all decision-blocked, so this was the only one buildable today.

**Evidence.** `runs/2026-11-02-07/`: `eval.md` (baseline green; the gap between case 21's budget release and the
backend's own list named), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the spectrograms and the loudness numbers.

**Measured** (real audio device, `MixerBackend::open`):

| | before | after |
|---|---|---|
| 400 one-shots, all ended: slots held | **400** | **6** |
| 200 one-shots: slots held | 200 | ≤ 32 asserted (2 measured) |
| 200 one-shots: distinct indices handed out | 200 | 200 — each hand-out is a fresh *name* |
| a stale index's `finished` / gain / stop | moved the slot's new occupant | no-op; `finished` says ended |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.354 / 1.263 µs | not on this path; `perfcheck` drives neither the mixer nor the music |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect in my own work: the first
probe I wrote for `examples/budget` used a fake output handing out its own indices, so it measured the crate's entity
side — never the leak — and would have printed the same line on both trees. Deleted; the measurement that matters came
from the real backend, which this machine can open.

**Traps for the command.** (1) A fake `AudioOutput` cannot measure `MixerBackend`: the leak is the real graph's slot
vector. Test the real backend where a device exists and skip with a sentence where it does not. (2) Bounding slots by
**reusing indices** would be a worse bug than the leak — a stale `MixerVoice` would move another voice's gain — so the
test asserts *slots bounded* and *names distinct* in the same breath; either alone passes vacuously. (3) `hs distill`
is still GPU-blocked (`bge-m3` wants 5000 MB behind a 14.8 GB VLLM engine), fifth iteration running; the rule is
argued from the vendored `rodio` source and Gregory §14.5.8.4, both already cited here.

**Commit.** `32f0ce3`, pushed to `main`; CI run 41554421622 is **success** on `big-s1`. Gate: build, test (54 + 4), serde (61 + 4), mixer (61 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; `MixerBackend::voices()` is unused there, so the one changed accessor reaches no consumer.

**Next 5.**

1. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`; a new
   `match` arm for a consumer, so the owner's call.
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **The mixer's decode error is discarded** at `mixer/mod.rs` (`Decoder::new(..).ok()?`); carrying it into the refusal
   would say *why* a file failed. Same decision as item 1.
5. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question.

**Baseline note.** Nothing was red, and the suite had 53 tests. Cases 20–21 prove the **budget** frees a voice; none
could see the **backend's** own list, which is why five iterations of green runs sat on top of a 400-slot leak.

---

## Iteration 6 — 2026-11-02 — a refused sample names the sound, not the cue

**Change.**

- **`src/playback.rs` — a decode failure names the sound key.** `start_pending` reported
  `MissingSound { sound: voice.cue }` — the cue's name in a field documented as the key a project maps to a file — so a
  corrupt `step_03.wav` sent a person looking for `footstep.grass`. It now recovers the key from the same cue and
  variant `answer` chose (`key_of`), and reads `Res<CueTable>` to do it. No public type, field or message changed.
- **`src/play.rs` — one sentence for the variant's two causes:** *"no handle or undecodable bytes for sound
  `step_03`"*, with the doc saying which two call sites produce it.
- `examples/budget.rs` — a `missing-sound.txt` probe that runs both causes side by side in the real ECS;
  `tests/mixer.rs` — a `refuses_samples` mode on the fake output (the same `None` the real mixer returns when
  `rodio::Decoder::new` errors) and two tests; `tests/play.rs` — the sentence; `cues.yaml` case 25;
  `needs-decision/undecodable-sound.md`; the cues guide, the CHANGELOG and a media note.

**Why.** Top of iteration 5's next-5, and its own class: a refusal confidently naming the wrong subject, in the crate
whose refusals exist to name what is missing. Found by iteration 5's probe, which fed four zero bytes to the mixer and
got `no handle for sound \`line\`` for a cue whose key was also `line` — a coincidence that hid it from every test.

**Evidence.** `runs/2026-11-02-06/`: `eval.md` (baseline green; the table's gap named — case 11 covers only the
missing-handle cause), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the two-cause probe and the cue spectrograms.

**Measured.**

| cause | before | after |
|---|---|---|
| a key with no handle | `sound: "step_03"` (true) | `sound: "step_03"` |
| a key whose bytes do not decode | **`sound: "footstep.grass"`** (the cue) | `sound: "step_03"` |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.059 / 0.329 / 0.827 µs | not on this path; the machine's load was 6.2 against 2.7 in iteration 5, which is the whole difference |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect in my own work: a third test
tried to resolve a default handle's id so a waiting voice would reach the code under test, and `Assets::insert` on that
id does not make `assets.get` succeed — the test proved nothing on any tree and was removed rather than contorted.

**Traps for the command.** (1) A default `Handle::<AudioSource>` **never** resolves, however you insert into `Assets`
afterwards: a test that wants "bytes arrive later" must build a real handle and hold *that*. (2) A cue and its key
having the same name (`line`/`line`, as in `examples/budget`) hides this whole class of bug — the probe's first run
looked correct for exactly that reason. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM
engine holds the card), so the rule is argued from the vendored `bevy_asset`/`rodio` sources and the crate's own R6.

**Commit.** `409eac6`, pushed to `main`; CI run 41553622842 is **success** on `big-s1`. Gate: build, test (53 + 4), serde (60 + 4), mixer (59 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; this change's host effect is a corrected `Entry::Refused` sentence, which the panel prints verbatim.

**Next 5.**

1. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`. A new
   `match` arm for a consumer, so not decided here; option 4 in that file (log the cause) is one line and needs no
   answer.
2. **A payload that also names the cap** — `needs-decision/refusal-names-its-cap.md`.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **The mixer's decode error is discarded** at `mixer/mod.rs:147` (`Decoder::new(..).ok()?`); carrying it into the
   refusal would tell a project *why* the file failed, not only which one. Wants the same decision as item 1.
5. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.

**Baseline note.** Nothing was red, and the suite had 52 tests. Case 11 proves the missing-handle cause; no case could
reach the decode cause through `SirenSounds` alone, which is why iteration 5's probe found it by accident.

---

## Iteration 5 — 2026-11-02 — a refusal names the bus only when the bus refused

**Change.**

- **`src/play.rs` — the refusal names the cap that refused.** `room_for` has three hard caps and returned one
  uninformative `None` from all of them, so `answer` always blamed the cue's bus. It now returns
  `Result<Vec<Entity>, Cap>` with a private `Cap::{Cue, Bus, Table}`; `bus: Some(name)` only for the bus's own cap,
  `bus: None` otherwise — which is what the field's own doc says (*"the bus that had no room, when one did"*).
  No public type, field or message changed.
- `examples/budget.rs` — a **`refusals.txt` probe** that forces each of the three caps in the real ECS and writes the
  payload and the sentence; a real WAV per cue, because dummy bytes are refused by the decoder (`MissingSound`) before
  the budget is ever consulted.
- `tests/mix.rs` — four tests: table and cue caps name no bus (both fail on the unfixed tree), a full bus still names
  it (passes on both, so the fix cannot overshoot), and each payload shape has exactly one sentence.
- `examples/render.rs` — the three `VoiceLimit` sentences; `needs-decision/refusal-names-its-cap.md`; `cues.yaml` case
  24; the buses guide, the CHANGELOG (with the host consequence) and a media note.

**Why.** The loop's first criterion, and worse than a silent failure: a **confidently wrong** one. The probe showed a
table-budget refusal accusing `Ambience` for a cue on `Effects` — a bus with a free slot, not even the cue's own.
Cases 7–9 each asserted `matches!(reason, VoiceLimit { .. })`; the wildcard was the hole, and no case read the payload.

**Evidence.** `runs/2026-11-02-05/`: `eval.md` (baseline green; the table's gap named), `research.md`, `plan.md`,
`perf.md`, `review.md`, and `evidence/{before,after}/` with the two render trees, the refusal texts and the budget
probe.

**Measured.**

| cap that refused | before | after |
|---|---|---|
| the table's `max_voices: 1`, two buses with room | `Some("Ambience")` → ``no voice free on bus `Ambience` `` | `None` → `no voice free` |
| the cue's `max_instances: 1`, its bus at 0 of 8 | `Some("Effects")` | `None` → `no voice free` |
| the bus's `max_voices: 1` | `Some("Effects")` | `Some("Effects")` — unchanged, and now the only one |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | — | 0.022 / 0.096 / 0.667 µs — `room_for` runs per request and only its refusal changed; the harness does not drive it |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect **in my own evidence
harness** (four zero bytes as a sample, so the decoder refused before the budget did — every line read
`MissingSound { sound: "line" }`, and the harness was measuring the wrong subsystem), and it exposed a second real
refusal bug, logged rather than folded in.

**A number worth keeping.** The before tree named a bus in **every** refusal, and the first row names a bus the cue
does not play on. "No voice free" is less informative but true; the diagnostic version needs the cap in the payload,
which is the decision file.

**Traps for the command.** (1) `examples/render` **hardcodes** its refusal strings, so it cannot show a change to a
refusal — the evidence for this iteration is `examples/budget`'s ECS probe, and `render`'s identical refusal files are
expected, not a failure. (2) An app that inserts a dummy `AudioSource` measures `start_pending`'s decoder, not the
budget: use a real WAV. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the
card), and the metadata API returned no abstract for the instance-limiting papers, so the rule is argued from the
crate's own R4 and Gregory §14.5.8.4's *per-group limits*, which this repository already cites.

**Commit.** `3d158ae`, pushed to `main`; CI run 41552664607 is **success** on `big-s1`. Gate: build, test (52 + 4), serde (59 + 4), mixer (56 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; it was not run, and this change's host-visible effect is a different `Entry::Refused` sentence —
the panel prints it verbatim, so it improves without a consumer change.

**Next 5.**

1. **`start_pending` reports a decode failure as `MissingSound { sound: <cue> }`** — a cue's name in a field documented
   as the sound *key*. A project greps its asset list for `footstep.grass` and finds nothing. Found by this
   iteration's probe.
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`, with the three sentences option 2
   would print.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.
5. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.

**Baseline note.** Nothing was red, and the suite had 48 tests. The defect was invisible because every test that
reaches a `VoiceLimit` matched it with `..`: the kind was asserted, the content never.

---

## Iteration 4 — 2026-11-02 — a crossfade takes the time it was given

**Change.**

- **`src/music.rs`, `src/model.rs` — `MusicLayers::fade_s` is honoured.** It was stored in `Pending` and read by
  nothing; `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms.
  `model::tau_for_fade` derives the constant from `approach`'s own definition — `fade_s / ln(1000)`, an arrival within
  one part in a thousand — and each `LiveLayer` carries the fade its own change gave it.
- **`move_layers` holds a layer's gain on a frame with no elapsed time.** `approach` returns its target for `dt <= 0`,
  which is right for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend
  the whole fade in one step. The tests found this; it is fixed in the crate, not in the tests.
- `examples/render.rs` — a **layer-fade plot** (the same move under `MIX_TAU_S` and under an authored 0.8 s) and its
  caption in `render.txt`; `tests/model.rs` and `tests/music.rs` — four tests, one of which fails on the unfixed tree;
  `cues.yaml` case 23; the music guide, the CHANGELOG and a media note.

**Why.** The top of iteration 3's next-5, and the same class as its two predecessors: a number a person wrote whose
sentence the code did not honour, silently. Rank 2 in the loop's order (music transitions and fades) but rank 1 in
practice — the field is public, documented and already in every project's builder chain.

**Evidence.** `runs/2026-11-02-04/`: `eval.md` (baseline green; the one red case is the fade), `research.md`,
`plan.md`, `perf.md`, `review.md`, and `evidence/` with both render trees, the fade plot and its zoom, and the cue
spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| 0.3 s into an authored 0.8 s fade | **−9.13 dB** (settled) | **−13.6 dB** (arriving) |
| the same fade at 0.85 s | −9.0 dB | −9.07 dB → its target |
| `fade_s(0)` at 0.5 s | −9.0 dB | −9.0 dB (default easing unchanged) |
| a layer the change does not name | — | keeps its own fade |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.019 / 0.103 / 0.684 µs | the same — the harness does not drive the music systems |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot**: a
fixed `0 dB` vertical scale that pinned a fade from silence to the floor, a marker labelled "duck fires" on a plot
with no duck, and a t=0 marker drawn on the y axis where its label collided. All fixed before the verdict.

**A number worth keeping.** The derivation is not a fitted constant: `approach` reaches `1 − e^(−t/τ)`, so "arrive
within a thousandth at `fade_s`" is `τ = fade_s / ln(1000)`, and `tests/model.rs` asserts both halves of that —
it has arrived at `fade_s`, and it has **not** arrived at `fade_s / 2`.

**Traps for the command.** (1) A headless test's **first** update has `dt = 0` unless `advance()` ran first, so a test
that writes a message and immediately updates tests the step branch of `approach`, not the fade. Every existing music
test advanced first; the new ones must too. (2) `perfcheck` cannot see a change on the music path — say so in
`perf.md` rather than presenting identical numbers as a measurement. (3) An evidence harness that draws *both* the old
and the new curve prints the same line whichever tree it was built from; that identity is by design and is not
evidence of the fix — the test is.

**Commit.** `951e627`, pushed to `main`; CI run 41551366531 is **success** on `big-s1`. Gate: build, test (48 + 4), serde (55 + 4), mixer (52 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"` and would compile the crate's third predecessor, so it was not run; no public type, field or message
moved.

**Next 5.**

1. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing a `Refusal` payload wants a decision file;
   the log line could be fixed without one, and probably should be.
2. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`. One line to remove in the
   next minor.
3. **A crossfade that a second change interrupts** replaces the target and the fade rather than queueing. Whether a
   game wants queueing, and from which gain it should start, is a design question the log now names.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.
5. **`Rng::next_f32` deliberately discards the low bits** of a xorshift64\* output (the weak ones). A sentence in
   `model.rs` stops the next reader re-deriving it.

**Baseline note.** Nothing was red, and the suite had 44 tests; the fade defect was invisible to it because no test
ever wrote a non-zero `fade_s` and then read the layer before it settled.

---

## Iteration 3 — 2026-11-02 — the seed a project writes, and the hold a stinger promises

**Change.**

- **`src/play.rs`, `src/lib.rs` — `SirenSettings::seed` is the stream.** `SirenChoices` was built once from a crate
  constant and never rebuilt, so a project's `seed:` reached nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. A new `play::seed_choices` system in `SirenSystems::Choose` replaces the stream —
  and every variant history with it — when the mix table's seed changes. It runs before the music starts a layer, so
  a score started on the same frame draws from the new stream too.
- **`src/music.rs` — a stinger's `duck_s` is a hold.** `play_stingers` passed it as `DuckClock::manual`'s *fade*
  argument with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired. It now holds for
  `duck_s` and releases over `DEFAULT_FADE_S` (0.12 s), the release every stop and every steal already uses.
- `examples/render.rs` — a **bus-gain plot** beside the stinger envelope, with both readings of `duck_s` in one run;
  `tests/play.rs` and `tests/music.rs` — five tests, two of which fail on the unfixed tree;
  `docs/loop/needs-decision/stinger-release.md`; `cues.yaml` case 17 rewritten and case 22 added; the cues and music
  guides, the CHANGELOG and a README line.

**Why.** Both are the loop's first-ranked class — wrong output, silent, from a number a person wrote whose sentence
said something else. Case 17 existed to prove reproducibility and could not, because a project could not choose a
stream; the stinger's `duck_s` was documented as a hold and passed as a fade, so the music swelled back underneath the
punctuation mark. Neither needed a new model: `active_duck_db` already had the shape, and the seed field already
existed.

**Evidence.** `runs/2026-11-02-03/`: `eval.md` (baseline green; the four next-5 items checked against the code — one of
iteration 2's was **wrong**, case 9 *does* have tests), `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/` with both render trees, the stinger envelope plot and its zoom, and the cue spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| `seed: 7` vs `seed: 99`, 8 plays | **identical** (variant, gain, pitch) | different from the first play |
| `seed: 7` twice, two processes | identical | identical |
| `Music` halfway through a 0.6 s, −9 dB stinger | **−5.01 dB** | **−8.99 dB** |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.075 / 0.326 / 1.090 µs | 0.023 / 0.135 / 0.508 µs — the machine's load fell 4× between runs; neither fix is on this path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot** (no
legend; the caption reporting the axis top as the peak; the "duck fires" label on the axis, crossing the release
curve) and one in my reading of it — the blue shelf looked like a slope at 900 px until the emitted polyline was
checked frame by frame: frames 39–48 are exactly −9.00 dB. All fixed; the zoom file shows the shelf as a shelf.

**Traps for the command.** (1) An evidence harness that hard-codes the *fixed* shape cannot show the old one — print
**both readings** of a changed argument in one run and let the numbers disagree, rather than rendering "before" from a
reverted tree that the harness does not read. (2) A plot's y-scale must include the cap/depth or a flat shelf sits on
the axis and reads as a curve; check the polyline's y values, not the thumbnail. (3) A `find ~ -iname …` for a paper
takes minutes on this disk — the corpus is on `big`, not here. (4) The home-still embedder was GPU-blocked for all 23
attempts of this iteration (`gpu busy: 4222 MB free < 5000 MB required`); `research.md` records that rather than
pretending a source was read.

**Commit.** `f4fca21`, pushed to `main`; CI run 41549988083 is **success** on `big-s1`. Gate: build, test (44 + 4), serde (51 + 4), mixer (48 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test was not run:
`bevy_cauldron` still pins `bevy_siren` at `rev = "7029b20"`, so it would compile this iteration's *predecessor* and
prove nothing; no public type, field or message moved this time either.

**Next 5.**

1. **`MusicLayers::fade_s` is stored in `Pending` and never read.** Every layer move eases at `MIX_TAU_S` (50 ms), so
   an authored 800 ms crossfade is silently 50 ms. Internal only — carry the fade into `LiveLayer`, or set the easing
   time constant from it — so no decision file is needed.
2. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit; the message shape change wants a decision file.
3. **`needs-decision/remove-completes.md`** — the public `Completes` component cannot ever be true. One line to remove
   it in the next minor, if the owner agrees.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release, shipped this iteration at the crate's
   `DEFAULT_FADE_S`. The field is additive; the default keeps behaviour identical.
5. **`Rng::next_f32` uses the top 24 bits of a xorshift64\* output.** Fine for a variant pick; worth a look before
   anyone uses `SirenChoices::rng` for anything wider, because the low bits of xorshift64\* are the weak ones and this
   implementation discards them deliberately — a sentence in `model.rs` would stop the next reader re-deriving it.

**Baseline note.** Nothing was red at the start, and the suite had 44 tests: the two fixed defects were both invisible
to it, one because no test had ever written a seed and the other because the stinger test read the duck only on the
frame it fired.

---

## Iteration 2 — 2026-11-02 — a voice that ends frees its slot, on both backends

**Change.**

- **`src/mix.rs` — `retire_voices` asks the backend that owns playback.** On the `mixer` backend there is no
  `AudioSink`, and nothing ever set `Completes`, so a `Mode::Once` voice was never retired: it kept its place in the
  voice budget for the life of the process. The system now asks `AudioOutput::finished` — the query the trait already
  declared and `MixerBackend` already implemented, with **zero callers before this commit**.
- **`src/mix.rs` — `update_mix` writes a `MixerVoice`'s own gain.** It used to `continue` past a mixer voice entirely,
  so `Applied` was never written: a music layer moved on the bar stayed at its start gain, and a stop fade began at
  silence and despawned the voice on its first frame — a cut where the crate promises a 120 ms fade. One writer still
  owns both numbers; Bevy's sink gets the whole chain, the mixer's voice stage gets the voice's own gain.
- `examples/budget.rs` (`--features mixer`) — the evidence harness: N one-shots against a cap, a scripted output whose
  voices end after a set life, writing per-frame CSV, a text summary and an SVG plot.
- `tests/mixer.rs` — four tests behind the feature, one of which fails on the unfixed tree.
- `docs/loop/cues.yaml` cases 20 and 21, `docs/guide/buses.md` (both promises made exact),
  `docs/loop/needs-decision/remove-completes.md`, a `CHANGELOG` "Fixed" pair, and a README line.

**Why.** This is the loop's most valuable class of bug — a refusal that should not happen, silent, and in the
backend whose whole point is that a bus is one number. Gregory §14.5.8.4 (*Instance Limiting*): a sound engine
"supports only a fixed number of simultaneous voices … so we must use them wisely", and a voice that stops *"frees
its resources for use by other voices"*. A voice that has produced its last sample and keeps its slot is exactly that
leak. The second defect fell out of the first test: the mixer voice's gain was never written at all.

**Evidence.** `runs/2026-11-02-02/`: `eval.md` (baseline green and why that was the problem; the case table, with
cases 9 and 17 still red), `research.md` (Gregory's lines, `rodio`'s `Player::empty`/`MixerSource`), `plan.md`,
`perf.md`, `review.md`, and `evidence/` with 46 WAVs/text from `examples/render` and four budget runs.

**Measured.**

| scenario | before | after |
|---|---|---|
| 40 one-shots, cap 32, stealable | 40 played, peak **40** live, **0** `ended`, last frame 32 live | 40 played, peak **3**, **40** `ended`, last frame 0 live |
| 6 one-shots, cap 4, priority 180 (unstealable) | **4** played, **2** `VoiceLimit`, **0** `ended`, 4 live forever | **6** played, **0** refused, **6** `ended` |
| a layer moved to −3 dB on the mixer | `Applied` never written; layer stayed at its start gain | `Applied { db: -3.0 }` |
| rendered audio | — | `render.txt` and all 17 WAVs **byte-identical**; cue −24.4 LUFS / −12.1 dBFS, `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.396 µs | 0.065 / 0.332 / 1.233 µs (inside the noise; `perfcheck` does not drive the changed code) |

**The stealable row is the one that matters.** With a stealable cue the leak is *invisible*: every play is answered
`Played`, because the permanently-full budget steals each time — 32 footsteps, then each new one cuts off a footstep
still sounding. The lockout row is what makes it a refusal a reader can see.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found four defects **in my own plot** (a
missing legend entry drawn outside the viewBox, the caption reporting the axis top as the peak, a cap label colliding
with the axis, and a blank raster because CSS variables do not resolve in a rasteriser); all four were fixed before
the verdict.

**Traps for the command.** (1) An evidence plot whose legend is anchored from the right edge silently loses an entry
when the two labels together overflow; anchor from the left and check the `x` extents. (2) A `--features mixer` test
that wants a started voice needs a **resolved** `AudioSource` handle, not `Handle::default()`: `start_pending` gives
a `MixerVoice` only once the bytes exist, so a default handle tests the loading state instead. (3) `errno=28` from
`ld` on this Mac is a full disk, not a code error — `/tmp` held 6.6 GB of my own probe build.

**Commit.** `6a62195`, pushed to `main`; CI run 41545567270 is **success** on `big-s1`. Gate: build, test (41 + 4),
serde (48 + 4), mixer (45 + 4), clippy, fmt, doc, examples, and the README quick start, all green. The mixer-example
link failed once with `errno=28` while the disk was at 100 % — the loop's documented trap, not a code error — and
passed after my own throwaway crate and the editor's build directory were removed.

**The Cauldron was not re-verified, and cannot be by this run.** `bevy_cauldron` pins `bevy_siren` at
`rev = "7029b20"`, so `audio_panel_reads_the_crates_file` compiles the *old* crate: it would prove nothing about this
change. It was attempted anyway and died of `No space left on device` during the 8.3 GB build. The public API did not
move (no type, field or message changed), so the panel's four reads — `SirenMix::db`, `ducked`, `voices_on`, the
`SirenLog` tail — keep their shapes; what changes for the panel is the number in `voices_on`, which is the fix. Its
next pin should move to this commit.

**Next 5.**

1. **Case 9 has no test.** An `UNSTEALABLE_PRIORITY` voice that must refuse rather than be stolen is the rule's other
   half, and the budget's evidence now shows a refusal path worth pinning. It needs a full bus and a dialogue cue.
2. **Case 17 pins the RNG, not the stream.** A determinism test over a whole `Played` sequence (two identical runs,
   equal field for field) is the case's own observation.
3. **`MusicLayers::fade_s` and `Stinger::duck_s` are stored and never read.** A crossfade a project asks for happens
   at `MIX_TAU_S` (50 ms) instead; the field's default is a lie about the shape of the fade.
4. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing the message shape wants a decision file.
5. **`Completes` is public and can never be true.** `needs-decision/remove-completes.md` has the options; the fix
   landed without it, so it is safe to remove in the next minor.

**Baseline note.** Nothing was red at the start. The suite was green with the `mixer` bug present because no test gave a
voice a `MixerVoice` and then let its sample end — which is why the first two items above are about tests rather than
features.

---

## Iteration 1 — 2026-11-02 — the loop's eyes: a render harness and a case table

**Change.** The loop now has a way to judge audio it cannot hear, and none of it touches `src/`:

- `examples/render.rs` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  22.05 kHz mono WAVs plus a `render.txt` of numbers, with no `AudioPlugin` and no `SirenPlugin`: it drives
  `bevy_siren::model`, where every decision's arithmetic lives.
- `examples/perfcheck.rs` — µs per frame and per voice at 8, 64 and 256 voices, an upper bound on `update_mix`.
- `docs/loop/cues.yaml` — 19 fixed cases, each naming its behaviour, its observation and its green condition, with the
  source for every number. The case table is the contract a run is judged against.
- `docs/loop/` — `log.md` (moved from `docs/loop-log.md`), `runs/README.md` and `needs-decision/README.md` describing
  the shape of a run and of a question the loop will not decide itself.
- `.omp/skills/build-loop/SKILL.md` — the loop, written to this brief as a **skill** (the model reads it with
  `skill://build-loop`), with this machine's measured facts in it.

**Why.** The loop's brief demands spectrograms, loudness numbers and panel screenshots before anything ships. The crate
had a panel and a panel is not an ear; nothing could render a cue to a file that `ffmpeg` could measure.

**Evidence.** `runs/2026-11-02-01/`: `eval.md` (the baseline, the case table, loudness), `research.md` (BS.1770-4,
Gregory, the panner), `plan.md`, `perf.md`, `review.md`, and `evidence/` with 23 WAVs and text, four spectrograms and
waveforms, and the duck's gain plot as a line.

**Measured.** 256 voices cost 1.253 µs a frame — 0.0075 % of a 60 fps frame. A cue at −6 dB authors −6.00 dB and reads
−24.4 LUFS (looped past the 0.4 s gating block) at −12.1 dBFS true peak. The duck leaves 0 dB at 1.00 s, reaches
−7.96 dB, and recovers by 1.97 s against a 0.25 s hold and a 0.6 s fade.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT — with one caveat written down: `voices-8.wav` peaks at
3.419 because the harness sums without the voice budget, which is case 19's and the budget's question.

**Traps recorded in the command.** A bus gain is a step function between frames, so one sample per frame is a 4 ms
file; `active_duck_db` takes the time *since the duck fired*, not the score's clock, or the duck never recovers;
`showwavespic` on a gain curve draws an unreadable blob, so a bus plot is a line; and `ebur128` reads −70 LUFS for any
clip shorter than one gating block, whatever its level.

**Commit.** `4e99c64`. Gate: build, test (41 + 4), serde (48 + 4), mixer (41 + 4), clippy, fmt, doc, examples — all
green; nothing was red at the baseline.

**Next 5.**

1. **Case 19:** teach `examples/render` to model the voice budget, or move the case into `perf.md`, so a mix's evidence
   says how many voices it holds.
2. `tests/cues.rs` automating cases 1–4 and 17, so `cues.yaml` and the suite cannot drift.
3. The Cauldron's audio panel should surface the last refusal — a refused audition is a log line today, not a panel row.
4. Occlusion and a low-pass on the `mixer` backend (backlog, iteration 0).
5. Loudness targets per bus: BS.1770-4 is the measure, the target is a `needs-decision/` question.

---

## Iteration 0 — 2026-11-02 — repository setup

**What shipped.** The crate, whole:

- `src/model.rs` — engine-free maths: dB ↔ gain with one silence floor, a linear sum for simultaneous sounds and a
  dB chain for gains in series (a silent stage silences the chain), semitone ratios with a clamp, three distance
  laws and the backend panner's own law divided back out, a duck envelope, an exponential approach, the
  next musical boundary from tempo, a seeded xorshift, a no-repeat variant picker, and voice stealing by lowest
  priority then oldest.
- `src/cue.rs` — `CueTable`, `Cue`, `SoundSlot`, `PitchRange`, `Repeat`, `Mode`, `Attenuation`, and `problems()` in
  sentences. `from_ron`/`to_ron` behind `serde`.
- `src/mix.rs` — `SirenSettings`, `BusLevel`, `Duck`, `DuckClock`, `SirenMix`, `Stopping`, `Applied`,
  `UNSTEALABLE_PRIORITY`; the systems `update_mix`, `apply_stopping`, `retire_voices`, `stop_requests`.
- `src/play.rs` — `Play` → exactly one `Played` or `Refused`; `Voice`, `Compensation`, `Samples`, `MixerVoice`,
  `Completes`, `StopVoice`, `StopCue`, `SirenChoices`; the three caps checked in the order a person asks them.
- `src/playback.rs` — `start_pending`: the one place a voice starts on the crate's own mixer once its bytes load.
- `src/music.rs` — `MusicScore`, `MusicLayer`, `MusicLayers` with `When::{Now, NextBeat, NextBar}`, `Stinger`,
  `MusicState`; layers are voices started through `Play`, so the music has no second spawn path.
- `src/sounds.rs`, `src/log.rs`, `src/output.rs`, `src/mixer/mod.rs` — the key map, the 256-entry log, the
  `AudioOutput` trait and its `rodio` summing graph.
- `src/lib.rs` — `SirenPlugin`, `MusicPlugin`, `SirenPlugins`, `SirenSystems::{Choose, Play, Mix}`.

**Verified.**

| Check | Result |
|---|---|
| `cargo build` (the trimmed feature list a consumer gets) | green |
| `cargo test` | 49 tests, green |
| `cargo test --features serde` | 49+ tests, green |
| `cargo test --features mixer` | 49 tests, green |
| `cargo test --all-features --doc` | green |
| `cargo test --features readme-example --test all readme::` | green (README quick start == crate-root doctest) |
| `cargo build --examples` | green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo run --example music` | transitions at t = 0.00, 4.80 and 7.20 s — every one a multiple of the 2.4 s bar, `transitions on the bar: yes` |

**Captures.** `docs/media/mixer.png` and `docs/media/mixer.gif`, from `examples/mixer` at
`SIREN_FRAMES=240 SIREN_KEYS=steal,layer,duck SIREN_KEY_EVERY=25`, release build, `mean_ms 21.3` (240 frames). The
still reads: Music −9.6 dB with `duck −8.0` and 2 of 12 voices, Effects −3.8 dB with `duck −4.0` and 1 of 2, the cue
table at rest, and a log whose ten lines are the dialogue play, its two duck lines, a steal and its `stolen by`
line. **The captures earned their keep three times**: the first showed a missing glyph (an em dash is not in the
bundled font) and the cue list printed three times (one `Kind::Cues` shared by three nodes); the second showed a
bus at two of two whose steal had already expired; the third showed the music layer at −80 dB because the layer's
level and its cue's trim were both −4 dB. Each was a defect in the example, not in the crate, and each was fixed
before the image shipped.

**One real defect the tests then found.** The capture's steal step asked for three voices in one frame, and all
three played on a bus capped at two: the budget took its snapshot of live voices before the loop, so the frame's
own requests could not see each other. `play_requests` now keeps an owned `Live` list per request, drops what it
stopped and pushes what it started, and
`tests/mix.rs::three_requests_in_one_frame_fill_a_cap_of_two_with_no_third` pins it.

**Cauldron wiring (S-01).** `bevy_grimoire`'s `bevy_cauldron` hosts this crate behind an optional `bevy_siren`
feature, landed as `aea2456` and merged in `d6c6579`, pinned at `7029b20`; `src/audio_panel.rs` reads `<project>/family/bevy_siren.ron` into the crate's own
`SirenSettings` and `CueTable`, draws the bus rows from `SirenMix` and the log tail from `SirenLog`, and auditions the
selected cue with `Play::cue`. `bevy_siren#1` (S-01) and `bevy_grimoire#274` are the two records the two-way street
asks for, both filed in this pass.

Verified on the merged editor tree: `cargo test -p bevy_cauldron --features bevy_siren --test editor` → 1144 passed,
4 failed, and the four are `a_person_finds_and_runs_a_command_in_the_palette`, which fail identically with the
feature off (checked: 0 passed, 4 failed) — they predate this work. `cargo clippy -p bevy_cauldron --features
bevy_siren --all-targets` is clean, and `boot_headless` opens the shipped project on both feature sets: 12 frames,
2048 entities, no panic. The four panel tests read `Played`/`Refused` through a reader system: draining the doubled
message buffer after several updates lost one in the full parallel run, and a host reads every frame, so the test
does too.

**Decided (assumed, owner may override).**

- **Two backends, not one.** `bevy_audio` by default and our own `rodio` graph behind `mixer`, because a bus's gain
  as one number per bus per frame is the point of a bus, and Bevy's path cannot express it — but Bevy's path is the
  one that pans a moving emitter, so it stays the default.
- **The panner's distance law is divided back out.** rodio's `Spatial` multiplies each ear by `min(1, 1/d²)` as well
  as panning, so the crate's own attenuation would be applied twice. `model::backend_spatial_gain` mirrors that
  formula; `tests/model.rs::the_backend_spatial_gain_is_the_panner_distance_law` pins it.
- **`UNSTEALABLE_PRIORITY` is 180.** Music (200), stingers (210) and dialogue (180) keep their voices when effects
  fill the budget. A number, not a flag, so a project can position a cue between them.
- **The default duck is Gregory's pair.** Dialogue lowers music 8 dB and effects 4 dB, hold 0.25 s, fade 0.6 / 0.4 s.
- **`rust-toolchain.toml` pins 1.99.0.** `bevy 0.20` and this crate require 1.97.1; the machine's `stable` was 1.94,
  which cargo refuses. 1.99.0 is installed here and CI's `dtolnay/rust-toolchain@stable` is newer still.

**Backlog, ranked.**

1. **Occlusion and a low-pass on the `mixer` backend.** A wall between the listener and a sound should muffle it, and
   the mixer has no filter in its graph today. Needs a rodio filter source per voice and a ray-cast hook the game
   supplies — the crate must not own physics.
2. **A dialogue queue with subtitles.** `Dialogue` voices are unstealable but unordered: two lines at once play over
   each other. A queue with a `DialogueLine { speaker, text, cue }` and an accessibility subtitle stream is what a
   game actually needs next.
3. **Per-voice reverb zones.** A zone is data (a box and a wet mix); a voice inside it gets a send. The mixer path can
   do this properly; Bevy's path cannot, which is a reason for the `mixer` feature to grow.
4. **`SirenProbe`.** A way to ask "what would this cue play, and at what gain" without playing it, so the Cauldron's
   panel can preview a decision on hover instead of on click. Filed here as an `S-NN` when the editor needs it.
5. **A moving emitter's pan on the `mixer` backend.** rodio's `SpatialPlayer` can be re-positioned; a mixer-owned voice
   cannot today, which is the one thing Bevy's path does that ours does not.

**Release: published.** `cargo publish` from `big` (whose `~/.cargo/credentials.toml` carries the `[registry]` token;
this Mac has none, and the first attempt here was refused for that reason) uploaded `bevy_siren 0.1.0`. The registry
answers 200 for `https://crates.io/api/v1/crates/bevy_siren` with `max_version 0.1.0` and `Ladvien` as owner, and the
tag `v0.1.0` is pushed at `8e14231`. `docs.rs/bevy_siren/0.1.0/bevy_siren/` answers 200 — docs.rs had built it within a
few minutes of the upload, and the bare crate URL lags that by one more sync. The README's own image URLs answer 200.

**CI.** Two things were wrong, and neither was the code. Every sibling repo has a self-hosted runner registered and
this new one had none, so GitHub had nowhere to run; the runner is now deployed (`bevy-coven/bevy_siren 1` in
`home-cloud/gh-runners/repos.txt`, `deploy.sh`, `big-s1 online`). And `run: cargo test … readme::` is **not valid
YAML** — unquoted, `readme::` parses as a mapping and GitHub rejects the whole workflow, which is why every run still
failed in 0 seconds with no job even with a runner present. Quoted, it is a string. Run 41527805041 is the first real
one: **both jobs green on `big-s1`, 4m29s** (`fmt + clippy + doc`, then `build (standalone) + test`).
