# bevy_siren — notes for agents

Sound for Bevy 0.20: a cue table as data, buses that duck, a voice budget that refuses politely, layered music that
changes on the bar, and a log of what played and why. Its design rules and their sources are in
`docs/research/siren.md`.

README and docs follow https://github.com/bevy-coven/.github/blob/main/DOCUMENTATION.md; a feature is not done until its README line and, if visible, its GIF exist.

## Source of truth

This repository, [`bevy-coven/bevy_siren`](https://github.com/bevy-coven/bevy_siren), is canonical. MIT OR Apache-2.0. A
change a game or the editor needs is made **here**, in its own commit with its own test, and requested here as an issue
labelled `consumer` and `consumer:<project>`; it is never patched into the consumer.

## Build and test

```sh
cargo build                                   # the trimmed feature list a consumer gets
cargo test                                    # one `all` binary, including tests/leaf.rs
cargo test --features serde                   # the RON round trips
cargo test --features semantic                # retrieval, the tool host, and `siren-tools` as a process
cargo test --features mixer                   # the mixer backend's own code paths
cargo test --doc                              # nextest skips doctests; this does not
cargo test --features readme-example --test all readme::
cargo build --examples
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

**`cargo build` is the load-bearing step**, not `cargo test`: the dev-dependency is the full `bevy` umbrella (the
examples need a window), and Cargo unifies features across a build — so under `cargo test` the crate silently receives
every Bevy feature and a hole in its own `[dependencies]` list cannot fail.

## Layout

- `src/model.rs` — every number the crate decides with and **no engine type**: `db_to_gain`, `gain_to_db`, `sum_db`,
  `chain_db`, `semitones_to_ratio`, `distance_gain`, `backend_spatial_gain`, `active_duck_db`, `approach`,
  `next_boundary`, `Rng`, `VariantPicker`, `Candidate`, `steal`. `tests/all.rs::engine_free` scans this file for engine
  names.
- `src/cue.rs` — `CueTable`, `Cue`, `SoundSlot`, `PitchRange`, `Repeat`, `Mode`, `Attenuation`, `CueProblem`,
  `problems()`, `from_ron`/`to_ron`.
- `src/mix.rs` — `SirenSettings`, `BusLevel`, `Duck`, `DuckClock`, `SirenMix`, `Stopping`, `Applied`,
  `UNSTEALABLE_PRIORITY`, and the systems `update_mix`, `apply_stopping`, `retire_voices`, `stop_requests`.
- `src/play.rs` — `Play`, `Played`, `Refused`, `Refusal`, `Voice`, `VoiceOwner`, `Compensation`, `MixerVoice`, `Completes`,
  `Samples`, `StopVoice`, `StopCue`, `SirenChoices`, and `play_requests`.
- `src/owner.rs` — **ownership without hierarchy**: `OwnerState` (the three states of an owner entity, from `Entities::get_spawned`: alive, pending — allocated, its spawn command still queued — and gone), `Follows` (a placed voice's offset in its owner's space), and `end_and_follow`, the system straight after `play_requests` that despawns a voice whose owner is gone (writing `owner despawned`) and keeps a placed voice at its owner's side. `Play::on` gives a voice a `VoiceOwner`, **never a `ChildOf`**.
- `src/playback.rs` — `start_pending`: the only place a voice starts on the crate's own mixer once its bytes have
  loaded, and the **second** answer for bytes that turn out undecodable after the play; `trim_ogg_loops`, which tells **Bevy's** path how long an Ogg loop is (`PlaybackSettings::duration`) once the bytes are there; and `refuse_unplayable_voices`, which refuses and frees a voice whose file the asset server says failed to load or — on Bevy's path — whose bytes the decoder refuses, in `Update`, before Bevy's `PostUpdate` decodes them.
- `src/ogg.rs` — std-only, always compiled: `inspect` (one walk of an Ogg Vorbis file's pages → the length its last granule position declares, or `None` for anything it does not fully understand: chained, cut short,
  not Vorbis, a packet count that cannot account for the granule) and `loop_length`/`take_duration` (the `Duration` that makes `rodio`'s `TakeDuration` emit exactly that many frames: `N·p + 1` ns, **not**
  `frames / rate`). `semantic::decode`, `trim_ogg_loops` and the mixer's `Shape::of` all read it. `ogg::synthetic` (`cfg(test)`) builds Ogg streams by hand.
- `src/output.rs` — `AudioOutput` (the trait a backend implements; **`decode_error` is its defaulted question *can your `play` decode these bytes?*** — a backend whose `play` can refuse must override it), `Output` (the boxed resource), `Backend` (which
  path is live).
- `src/mixer/mod.rs` — behind `#[cfg(feature = "mixer")]`: one `rodio::Mixer` per bus, a `BusGain` and a `VoiceGain` source reading an `AtomicU32`, `MixerBackend` implementing
  `AudioOutput`, and `once`/`looped`/`connect`, the one place a voice's chain is built (**a loop repeats the sample, then applies the gain**; `Shape` is once, or a loop cut to the stream's declared length;
  `OpenSpan` keeps a decoder that reports an empty first span from making an empty loop). Its unit tests pull samples through a real
  `rodio::Player` and bus `Mixer` offline — the only tests that read a sample the crate's own mixer produces.
- `src/music.rs` — `MusicScore`, `MusicLayer`, `MusicLayers`, `When`, `Stinger`, `MusicState`, `MusicVoice`, and the
  systems `start_layers`, `adopt_layers`, `move_layers`, `play_stingers`.
- `src/sounds.rs` — `SirenSounds`: the key → handle map, `load_table`, `missing`, `failed`, **and a cache of what the decoder said about each asset's bytes** (a `Mutex` inside it holding **one memo per asker**, so the systems that ask hold it as a `Res`; a clone starts empty); `load_error`, the one place the crate reads `LoadState::Failed`; and `cannot_play`, **the one question *can this sound be played?*** (`play::answer`, `refusal_sentence` and `refuse_unplayable_voices` all ask it): a file that failed to load with nothing to play, or bytes **the decoder that will play them** refuses, asked once per buffer — `Decode::Bevy` (**the call Bevy's `AudioSource::decoder` makes, minus its `unwrap`**), `Decode::Output` (the mixer's own, through `AudioOutput::decode_error`) or `Decode::Skip` (a mixer voice still waiting for its bytes, which `start_pending` answers for).
- `src/log.rs` — `SirenLog` (256 entries) and `Entry`; **`pushed()` and `since(mark)` are the cursor** a game prints the log with: each entry carries the number it was pushed as, because the ring drops its oldest *routine* entry, not its oldest, and a position (or `len()`) is not a mark.
- `src/semantic/server.rs` — behind `semantic`: the MCP tool host (`Host`, `SPECS`, `read_message`) as a library, so a test
  drives it with no process. `src/bin/siren-tools.rs` is the loop around it (ADR 0006, 0008); `serde_json` is its one
  dependency.
- `examples/semantic.rs` and `tools/export_clap_onnx.py` — the one place the **real** CLAP graphs run: the script makes
  them from a Hugging Face cache and checks itself against `transformers`; the example indexes a folder, ranks a
  description and plays the answer. CI has no model, so **no test runs them** (clippy compiles the example) —
  `docs/guide/semantic.md`, "Try it", is how to run them, and `docs/media/README.md` regenerates the picture.
- `examples/quick_start.rs`, `assets/audio/` and `tests/quick_start.rs` — **the code a reader copies, the two footsteps it plays, and the test that runs it.** The README's Quick start and the crate-root doc's `no_run` block are the example **character for
  character**; `tests/quick_start.rs` brings it in with `include!` and runs its own `setup`, `walk` and `report` on the real asset server over `assets/` (what `DefaultPlugins` serves), Bevy's real `AudioLoader` and the real files, with the **console
  captured** (a `tracing` layer through `bevy::log::tracing_subscriber`: no new dependency); `tests/readme.rs` (behind `readme-example`) holds the README to the example. `assets/README.md` has the files' provenance (CC0) and their `ffmpeg` commands;
  `Cargo.toml`'s `include` leaves `assets/` and `examples/` out of the package.
- `src/semantic/golden.rs`, `src/semantic/eval.rs`, `golden/<set>/`, `tools/fetch_<set>.py` and `examples/goldenbench.rs` — the
  **golden sets** (`#35`, ADR 0009): `esc50` (2 000 recordings, CC BY-NC, so fetched and never vendored) and `kenney` (303 game sounds, CC0,
  fetched and verified all the same), each a labelled library pinned by a manifest, the text queries with their rationales in two wordings,
  the loader that refuses a mistake by line, the metrics (`eval::separation` and `eval::centred` ask whether a floor can tell the
  answerable from the unanswerable), and the benchmark that puts the real model through all of it (`goldenbench --set <name>`).
  `tests/semantic_golden.rs` tests the metrics, the loader and the checked-in sets' own invariants; **no test runs the benchmark** (no
  model, no network in CI). `golden/<set>/baseline.txt` is what a ranking change is measured against, and a hit@1 change under about eight
  points is inside what re-encoding the same clips does. **A new set is a directory of `golden/` plus its fetch tool**, and a test fails
  for a set that has neither.
- `src/lib.rs` — the plugins, `SirenSystems::{Choose, Play, Mix}`, the re-exports, the quick start doctest.

## The loop, as a skill

`.omp/skills/build-loop/SKILL.md` is this project's build loop — one iteration: orient, pick the most valuable item,
research it, build within the crate's rules, test, judge by spectrogram and loudness, three GREAT reviews, ship, log.
Read it with `skill://build-loop` before an iteration. It is a **skill** rather than a slash command because it is the
model's procedure, not a keystroke: `.omp/skills/<name>/SKILL.md` is what the native provider discovers at startup, and
one name has one home. It also carries the four traps that have already cost an iteration each (a bus gain is a step
function between frames; a duck takes time since it fired; a gain curve is a line, not a waveform; `ebur128` reports
−70 LUFS for any clip under its 0.4 s gating block). Add a trap to it when one costs you a pass.

## Rules

- **`src/model.rs` names no engine type.** It is the half that can be tested with no app, and `tests/all.rs` enforces
  it. A `Vec3` there is a bug even when it compiles.
- **Every play answers exactly one of `Played`/`Refused`.** A refusal names what is missing; nothing plays on a refusal.
- **One writer per voice gain.** `update_mix` owns every sink and every mixer voice number; nothing else writes a
  volume. `MusicLayers` moves a *target*, never a sink.
- **The two backends answer identically.** A test must pass with and without `--features mixer`; only the sample path
  may differ. A voice carries `Samples` always, an `AudioPlayer` only when the mixer does not own playback, and a
  `MixerVoice` only once the mixer started it.
- **`dB <= SILENT_DB` is silence**, on the way in and out. Never compare a linear gain to an epsilon.
- **Real time, not the game clock**, for the mix and the music: a menu that pauses the game must not freeze a fade.
- **No `unwrap()`/`expect()` on caller data** in `src/`. A table that cannot parse is a `String`; a distance that is
  `NaN` is at the listener's feet.
- **No game vocabulary in `src/`.** `tests/leaf.rs` forbids a game, the editor and a physics engine by name, and the
  dependency list is closed at `ALLOWED_DEPS` in that file: adding a name takes the sentence that argues for it, in the
  same commit (the file says so, and so does the build loop).

## Traps paid for

- **`AudioPlugin` opens a device**, and adding it twice panics. `SirenPlugin` never adds it; a headless test adds
  `MinimalPlugins + AssetPlugin` and calls `init_asset::<AudioSource>()`.
- **Without an audio device, an `AudioPlayer` entity still exists** and never gets a sink. That is what lets a headless
  test assert on `Played` and `Voice`, and why `retire_voices` must not treat "no sink" as "ended".
- **`Time<Real>` in a headless app only moves on `TimeUpdateStrategy`.** `Time::advance_by` is overwritten by the next
  `TimePlugin` update; `tests/helpers.rs` inserts `ManualDuration` instead.
- **`rodio`'s `Spatial` pan multiplies by `1/d²`.** Applying a cue's own attenuation on top would apply two distance
  laws; `model::backend_spatial_gain` is the panner's law, divided back out.
- **A `Mode::Loop` voice has no end to notice.** `retire_voices` skips loops; they live until stopped.
- **A rodio source cannot be re-positioned inside a `Player`.** That is why the `mixer` path applies spatial
  attenuation at play time, and why Bevy's path stays the default for moving emitters.
- **`rodio`'s `repeat_infinite` stores what it is given and replays it, so anything applied *inside* it is frozen at the first pass.** A looping mixer voice had its `VoiceGain` inside the repeat
  for the crate's whole life: a layer that started silent never came in and a fade was a click, and nothing refused or logged. No test noticed because the mixer's tests record the calls a fake
  `AudioOutput` receives and none read a sample. The gain goes **after** the repeat (`mixer::looped`), and `src/mixer/mod.rs`'s tests pull three passes through a real `Player` and a bus `Mixer`.
  **A claim about sound is held only by a test that reads samples.**
- **Every test in `tests/all.rs` is one process, so a scratch directory named by a label and the process id is shared by every test that picks the label.** Two indexer tests both said `"formats"`, each one's
  `Scratch::new` deleted the other's files, and `--features semantic` went red on a run that had been green three times — a flake in the baseline, found by the matrix and reproduced by running the two
  tests together with `--test-threads=2`. Every `Scratch::new` now adds a counter. **A test that passes alone and fails in the suite is a shared name before it is a bug.**
- **A format on `library::AUDIO_EXTENSIONS` is a claim about files, and `rodio`'s decoder features in `Cargo.toml` are the other half of it.**
  For most of this crate's life `mp3` and `flac` were on the list and not in the features, so a folder of them scanned as audio and then
  failed to decode — and no test noticed, because none had opened a file that was not a WAV. `every_extension_the_scan_calls_audio_has_a_decoder`
  decodes a real fixture of each entry (`tests/fixtures/`, made with `ffmpeg`, commands in `tests/semantic_decode.rs`): add a format to the list
  and the test asks for its fixture. **Check a decoder against a reference through the whole pipeline** (`formats_parity.py` in iteration 112's
  evidence), and **use `libvorbis` (`soundfile`) as the Ogg reference, not `ffmpeg`**, which returns up to 128 frames fewer than the stream declares.
  Vorbis is decoded by **`lewton`** (see the next trap): within 7e-7 of `libvorbis` and **not clamped**. Neither `rodio` Vorbis decoder cuts the padding of an Ogg's last block for every file (`decode` and `ogg::loop_length` do).
- **The `rodio` line in `Cargo.toml` is not this crate's alone: Cargo unifies its features across a game, and `vorbis` (the `symphonia` backend) made a loop of any Ogg *silent*.** `rodio` reports a span of
  length zero until a Vorbis decoder's first sample (the stream's first packet carries no audio), `Buffered` reads `Some(0)` as an empty source, and `repeat_infinite` — Bevy's own loop, `audio_output.rs:165`,
  and the mixer's — produced **0 of 147 456 samples** on a real Ogg: the voice finished at once with `Played` already written. It was in the crate from the first `mixer` commit and no test saw it,
  because every loop test used a WAV. `rodio` compiles `lewton` *out* when `symphonia-vorbis` is on, so the crate's feature also swapped the decoder of a game's **own** `AudioPlayer` (Bevy's `vorbis` is
  `rodio/lewton`). The line now names `lewton`; `tests/ogg_loops.rs::the_duration_plays_exactly_the_streams_length_through_rodio_itself` fails on the old one, `tests/mixer.rs::an_ogg_loop_on_the_real_mixer_sounds_until_it_is_stopped`
  needs a device, and `OpenSpan` protects the mixer's loops in a game that turns on Bevy's `symphonia-vorbis` itself. **A loop test needs a loop of every format the crate says it plays, and must compare a length, not just assert equality.**
- **A handle is more than a handle: the asset server has a load state, and a default handle never has one.** Every headless test's handle is `Handle::default()` (Uuid), which `AssetServer::get_load_state` answers `None` for, so for most of
  this crate's life a file that **failed to load** was a state no test could build: `Played` was written, the voice waited for ever, and the log read *played* / *stolen by `x` (cue)* — a healthy sequence for a sound nobody heard
  (`docs/loop/runs/2026-11-04-116/`). Bevy's own start system never reads the state either (`audio_output.rs:109`). **To test a load in flight or failed deterministically**, register a loader for `.ogg` of your own that **waits at a
  gate your test opens** and then succeeds or fails as told (`tests/failed_load.rs::Gated`) over `tests/fixtures`, so the order of events is the test's — a loader that sleeps makes *"still loading"* a race between a delay and a frame, false on a starved machine. And **remove the `Output`** to force Bevy's path (`Backend(false)` alone is not read).
- **Bevy's `AudioSource::decoder` is `….build().unwrap()`, and a headless app never calls it.** A file that loads and is not audio the build decodes (a Git LFS pointer, an empty file, an Ogg *Opus* file, an ADPCM WAV) was `Played` and then **panicked the game in
  `PostUpdate`** — on the default build, naming no file — for the crate's whole life: the asset loader returns bytes and stops, the crate's mixer's refusal (`start_pending`) is another path, and no test had a device
  (`docs/loop/runs/2026-11-04-117/`). `sounds::decoder_error` makes Bevy's call through `<AudioSource as Decodable>::Decoder` — **no `rodio` dependency in the default build** — once per asset (the cache in `SirenSounds`, keyed by buffer address, length and first
  16 bytes, so an edit is asked again; **one cache, not one per system — with one each a sound's first play built two decoders**), and `tests/undecodable.rs` holds that the crate refuses **exactly** what `AudioSource::decoder()` panics on (`catch_unwind`), which makes it true in every feature set. **Each backend is asked its own decoder** (iteration 118): the mixer's `Decoder::new` through `AudioOutput::decode_error`, a defaulted method (`None`), so the six `tests/mixer.rs` tests with placeholder bytes behind a fake output keep passing untouched — in a second memo, because the two constructions are different calls (they agreed on 14 of 14 files, `docs/loop/runs/2026-11-04-118/evidence/after/decoder-agreement-and-cost.txt`, but an agreement is a measurement, not a contract). **To see a crash a headless app cannot show**, run the real `AudioPlugin` on this Mac's device, one process per file, `catch_unwind` around
  `app.update()` (`evidence/after/undecodable.rs.txt`); a decoder's cost is a property of the format (Vorbis 0.65 – 0.8 ms to construct whatever the length, `hound` WAV 0.1 µs).
- **A `Played` followed in the same frame by a `Refused` and a despawn crashes the game that hangs data on `Played::entity`.** On the crate's own mixer, bytes `rodio` refuses were `Played` in `play_requests` and refused in `start_pending`, the next system: the voice
  entity was despawned in the frame it was announced, and a game system doing `commands.entity(played.entity).insert(..)` — the use that field documents — **panicked in `bevy_ecs`'s command error handler** (*"Entity despawned"*: Bevy 0.20's `insert` is fallible and its default handler panics; only `try_insert`
  is silent), naming the game's system and not the file (`docs/loop/runs/2026-11-04-118/`). The log read coherently and no test asked that a `Played` entity be alive. `AudioOutput::decode_error` asks the backend before the voice exists; **an output whose `play` can refuse must override it**, and one that does not keeps the old
  two answers, which `start_pending` still gives. The invariant is `tests/one_answer.rs`: **each play is answered once, and a `Played` entity that is gone at the end of its frame is explained by a `stopped` log line** — the one way it goes is a steal or a `StopCue` in the frame it was announced (it never sounded: `spawn_voice` gives a voice `Applied { db: SILENT_DB }`, `update_mix` skips a stopping voice, and `apply_stopping` despawns a voice that is already silent; `Played::entity`'s doc says so).
  **Run the documented use of a handle, not only its refusal**: a probe that did what the doc says a game does found this in one run.
- **`Play::on` made the voice a child (`ChildOf`), and a child is the wrong tool.** One line gave four defects, each found only by running the documented use across the states an owner can be in (`docs/loop/runs/2026-11-04-119/`): **a panic** for an owner that is allocated and not yet spawned — every `Commands::spawn` returns one, and the `ChildOf` hook queues a command on an entity that does not exist (`bevy_ecs-0.20.1` `relationship/mod.rs:194`, `error/handler.rs:128`), so a game that spawns a projectile and plays its sound in one system, unordered against the crate's, crashed or not by an arrangement it does not control; a sound at **owner ∘ `at`** (the voice's local `Transform` was the *world* `at`) — 12 dB under the law for an owner at the place and **+52 dB over the sample** for an owner with no `Transform`, which Bevy never propagates, because the crate's compensation (+80 dB for the panner's `min(1, 1/d²)` at 100 m) stands where the panner does nothing; a loop that lived for ever on an owner already gone; and a false `owner despawned` line for a voice the *game* despawned. `VoiceOwner` is data; `end_and_follow` reads `Entities::get_spawned` (three states, so *pending* is waited for and *gone* ends the sound); `at` stays a world position; a placed voice follows by writing its `Transform` from the owner's `GlobalTransform`. **A play on a gone owner is answered and ended in its frame and must take nothing from the budget** (`play_requests`'s `doomed`: no steal, no slot, no duck). **A relationship is a lifetime *and* a transform *and* a hook; use one only when all three are wanted.** To reproduce the pending state in a test, `world.entity_allocator().alloc()` then `world.spawn_at(id, ..)`; `tests/owner.rs` holds the rest, and `evidence/before/mutations.txt` lists the six switches that each redden their own test.
- **A README's code is a program, and a test that compares it with a copy of itself is not a test.** The Quick start never played a sound — not once since the first commit (`22c39a4`, 2026-11-02), which already had it (`docs/loop/runs/2026-11-04-120/`): `main` added neither `walk` nor `hear` (the reader's
  compiler said `function walk is never used` and nobody read it), the keys had no extension so `load_table` asked for `audio/step_01`, `hear` pushed a second log entry stamped `0.00s` into the crate's own log, and a guide sentence sent a reader to the wrong file.
  `tests/readme.rs` compared the README with the crate-root doc, and only under `--features readme-example`: **one found it only by pasting the block and running it on a real window and device.** **Run every block a reader will copy, and read the console they
  read**: the test captures it with a thread-local `tracing` layer (`with_default`), so `Update` is forced single-threaded (`SingleThreadedExecutor`) or a system on a pool thread prints where no one is listening. And **a log is read with a mark, not a position**:
  the obvious cursor — how many entries I have seen, against `len()` — printed 256 of 600 and then went quiet for good (`evidence/before/len-cursor-stalls.txt`); `SirenLog::since` goes by each entry's push number.

## The two-way street, both ways

**A consumer that needs something files it here, and this loop builds it here.** A game or the Cauldron opens an issue
on `bevy-coven/bevy_siren` labelled `enhancement` (or `bug`), `consumer` and `consumer:<project>`, titled `S-NN:
<what>` with the next free `NN` after the highest `S-` title, whose body says why, the proposed API, the edge cases,
and an acceptance a test in this repository can prove. An open `consumer` issue is taken right after bug issues and
ahead of the owner's focus; the fix is made here, in its own commit with its own test, as the general form (a project
is the example, never a name in the API), and the issue is closed only by that commit's `Fixes #N`.

**This crate's own needs go the other way.** When a cue, a bus rule, a music feature or a field of `SirenSettings`
cannot do what the Cauldron's audio panel needs, the request is filed on `bevy-coven/bevy_grimoire` labelled
`enhancement`, `consumer` and `consumer:bevy_siren`, and the editor item waits with `Needs: bevy_grimoire#N`. Never
patch, wrap or vendor the editor here.

Both directions carry the "recommend, record, proceed" rule and the `assumed-decision` label: an unanswered design
question is decided, recorded and built, never a reason to idle.

## Force pushes and deletions need the user's express permission

Never do any of the following unless the user has approved that specific action, by name, in the current conversation.
A loop brief, a skill, a standing instruction such as "commit, push, PR, merge", or an earlier session's approval is
not permission.

- Force-push in any form: `git push --force`, `--force-with-lease`, `--force-if-includes`, a `+` refspec, or pushing
  rewritten history. Deleting a remote branch or tag (`git push --delete`, `gh pr merge --delete-branch`, an API
  delete).
- Throwing away commits or uncommitted work: `git reset --hard`, `--merge` or `--keep`; `git checkout -- <path>` or
  `git restore` over changes you did not make; `git clean`; `git stash drop` or `clear`; `git branch -D`; `git worktree
  remove --force`; `git rebase` or `git commit --amend` on pushed commits; `git filter-repo` or `filter-branch`.
- Deleting files, directories, build caches, target dirs, models, datasets, database rows, issues, PRs, releases or
  cloud resources that you did not create in this session (`rm`, `rm -rf`, `git rm`, `cargo clean`, an API delete).

When one of these seems necessary, stop and ask. Name the exact command, what it destroys and whose work it touches. A
commit that landed on the wrong branch is fixed with a new commit or a question, never by resetting a shared working
tree or force-pushing. Files you created yourself in this session (scratch scripts, temp dirs, your own worktrees) you
may delete.
