# Iteration 119 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest pass it could affect.

## Pass 1 — Function

*Against the item's own acceptance — `Play::on` neither panics for an owner not yet spawned, nor loses an owner already gone, nor composes the owner's transform with `at`, nor blames an owner for a voice the game despawned; a placed sound follows its owner; the crate's own mixer never starts a gone owner's voice — and against the cue table: 40 cases (39 held, one new), none red; the 21 rendered WAVs are byte-identical to iteration 118's (21 of 21).*

**Critique, first reading — things that were wrong in my own work:**

1. **The first form of the follow was wrong, and a test I wrote late caught it.** The offset was measured against the owner's `GlobalTransform` the first frame the owner *existed* — and an owner spawned in the same frame has not been propagated (Bevy propagates in `PostUpdate`): it is the default, so the offset was measured against the origin and the sound jumped to *owner ∘ offset* a frame later — **(200, 0, 0)** for an owner at (100, 0, 0), the very double transform this iteration removes, back for the one arrangement that matters most (a placed sound on a projectile spawned with a whoosh). The first full matrix was green because **no test built that arrangement**; `a_placed_sound_on_an_owner_spawned_in_the_same_frame_follows_it_once_it_exists` was red on the first run (*"the sound has not moved: [200, 0, 0]"*), `end_and_follow` now waits while `Ref<GlobalTransform>::is_added()`, and **the matrix was run again on the final tree** (`matrix-final-first.log` is the first, `matrix-final.log` the last).
2. **My first design let a play on a gone owner steal a playing voice.** The red run on the unchanged tree already printed it (`stolen by engine (cue)` in the mixer test's log) and I read past it; the same line in the green run is what made me look. The doomed sound was answered, then ended in its frame — after taking the place of an engine that was sounding. `play_requests` now skips the steal pass, the live list and the duck for a play on a gone owner (`doomed`); `a_play_on_a_gone_owner_takes_nothing_from_a_sound_that_is_playing` is red when that is switched off (M6) and no other is.
3. **Two slips in my own evidence, both caught on re-reading**: one block of the probe's output transcribed wrongly into the *before* file (the `despawn-voice` mode), and an arithmetic line that said *"52.04 dB too loud"* where the truth is two numbers — the sink is written **+52.04 dB** (400× the sample) and that is **80 dB over the crate's law at 100 m**. Both are corrected in `evidence/before/owner-probe-on-the-unchanged-tree.txt`.
4. **The disk reached 0 GB mid-iteration** and `cargo test --features mixer` failed in the linker (*"linking with `cc` failed"*); I read it as a code error for the length of one command before `df` said otherwise. `cargo clean -p bevy_siren` — this crate's artifacts, made this session — freed 6.8 GiB, as the skill prescribes. The cause was mine: debug and release builds of the same crate, and I had not looked at `df` after the first release build.
5. **The level assertion in `a_sound_is_where_it_was_asked_to_be_whatever_its_owner_is` is partly circular**: it uses `model::backend_spatial_gain`, the crate's own mirror of `rodio`'s panner, and the compensation is computed from the same function — so once the voice is at `at` the equality holds by construction. Its real content is the **position** (independent of the model) and that the compensation the crate wrote matches the panner at the voice's *real* position, which is what the double transform broke. No device was involved; **the heard level was not captured**.
6. **The follow's lag was designed, not measured.** The system writes in `Update` and Bevy propagates in `PostUpdate`, so a moved owner's sound is right one update later (by reading the order); the test waits three frames, so *"at most one"* is the design, not an assertion.
7. **A sound on an owner that is already gone is now ended**, where it used to play (as an ownerless one-shot, or a loop for ever). For a loop that is the fix; for `Play::cue("impact").on(bullet)` where the bullet was despawned in the same frame, a one-shot that used to sound **no longer does**, and the log says why (`stopped: owner despawned`). It is the documented rule (*"stops when that entity is despawned"*) applied at the boundary, and it is recorded as option 3 of `needs-decision/play-on-owner.md` — but it is a behaviour change a game could meet, and I did not measure how often.

**What passed.** The probe, before and after, on the unchanged tree and the final one (`evidence/before/owner-probe-on-the-unchanged-tree.txt`, `evidence/after/owner-probe-after.txt`): the panic at `handler.rs:128` for an owner allocated and not spawned (an unordered system and `world.entity_allocator().alloc()`) is gone; the voice is at **(100, 0, 0)** for an owner at the place and for an owner with no `Transform` (it was (200, 0, 0) and (0, 0, 0)); a loop on a gone owner is **0 voices** after its frame (it was 1, for ever); a voice the game despawns is no longer blamed on its owner. **The first six of `tests/owner.rs` are red on the unchanged tree** (`evidence/before/owner-tests-on-the-unchanged-tree.txt`), and the crate's own mixer's output was asked to play 2 where 1 is right. Final matrix (`matrix-final.log`): default **226** (was 217), serde 235, semantic 544, mixer 241 + 20 lib, all-features 572 + 129 lib, doc 6, readme 1, clippy, fmt, rustdoc clean; the one warning is the known `example "mixer"` linker one. **Render**: 21 of 21 WAVs byte-identical to iteration 118's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix.png` read (an onset in the first 40 ms, three steady low lines; no vertical line, hole or shelf after it).

**Mutations** (each applied to the shipped code, the owner tests and `play::a_despawned_owner_stops_its_sound_and_says_so` run — eight owner tests and the existing one at the time — then restored; `evidence/before/mutations.txt`):

| switch | red |
|---|---|
| M1 a pending owner treated as gone | exactly `an_owner_that_is_not_spawned_yet_does_not_crash_the_game` |
| M2 a gone owner's voice never ended | 5 of 9, **including the existing** `play::a_despawned_owner_stops_its_sound_and_says_so` |
| M3 a placed voice never follows | exactly `a_placed_sound_follows_its_owner` |
| M4 the offset not taken in the owner's space | `a_sound_is_where_it_was_asked_to_be_…` and `a_placed_sound_follows_its_owner` |
| M5 the owner system after `start_pending` | exactly `a_gone_owners_voice_is_never_started_on_the_crates_mixer` |
| M6 a doomed play not exempted from the budget | exactly `a_play_on_a_gone_owner_takes_nothing_from_a_sound_that_is_playing` |
| M7 *(a catch)* the first form of the follow | exactly the same-frame-spawn test: `[200, 0, 0]` |

**Verdict: GREAT.** After the fixes above the item's acceptance is met on Bevy's path and on the crate's mixer, and its guards are mutation-checked.

## Pass 2 — Zoom

*The source diff, line by line, as a reviewer who did not write it — `src/owner.rs` (new), `src/play.rs`, `src/mix.rs`, `src/lib.rs`, `src/playback.rs`, `tests/owner.rs` — and every sentence I wrote about it.*

**Critique, first reading:**

1. **The public surface changed in three small ways**: `+ VoiceOwner(pub Entity)` (additive), `− mix::log_despawned_voices` (a `pub fn` system whose only reading of *owner* was `ChildOf`), and `mix::EndedVoice` (a `pub type` used by `retire_voices`) loses its last, unused element. `bevy_grimoire`'s crates contain no `Play::cue(..).on(..)` and no `ChildOf` beside a `Voice` (`grep`, read-only; **`Children` was not searched**). Recorded with options in `needs-decision/play-on-owner.md`; **a game that walked an owner's `Children` to find its sounds finds none** and uses `VoiceOwner`.
2. **`play_requests` is still at Bevy's sixteen**: `backend` and `output` became one tuple parameter and `&Entities` is the new one, and the comment on `files` (*"this is the sixteenth"*) is still true. `Decode`, `OwnerState`, `Follows` and `state_of` are `pub(crate)`; clippy's `private_interfaces` is clean.
3. **One system reads an owner's state in two places** (`play_requests`, for `doomed`; `end_and_follow`, for everything else) through one function, `state_of`, so *pending* and *gone* cannot be told apart two ways.
4. **A gone owner's voice is despawned at once, not faded** — the cascade the hierarchy gave, pinned by an existing test that passes unedited. A play on a gone owner is answered `Played` and ended in its frame, like a same-frame steal; the doc on `Played::entity` says so and says to use `get_entity` or `try_insert`. **Because the despawn is flushed inside the `Play` set, a game system after `Play` that hangs data on that entity with `insert` will panic for such a play** — rarer than the same-frame steal of 118 (it needs a play on an owner that is already gone), and documented, but not nothing.
5. **`Follows` is inserted only for a placed voice on Bevy's path**; on the crate's mixer a placed voice with an owner does not follow (no panner; the distance law is applied once, at the play) — said in `spatial.md`. A pending owner is waited for **with no bound** (an id reserved and never spawned is a game's own bug, and costs one state check a frame). A zero-scale owner has no inverse and its sound waits.
6. **The removed system's per-frame work is gone**: it walked every voice every frame and rebuilt a `BTreeMap`. Not timed — **[INFERENCE]** from reading, stated in `perf.md`.
7. **Docs that said the old thing, corrected on the read** (found by `grep` for `ChildOf`, `parent` and `log_despawned_voices` across `README.md`, `docs/`, `AGENTS.md`, `src/`, `tests/`, `examples/`): `docs/guide/buses.md`'s *"a voice it never parented"*, `playback.rs`'s comment naming the removed system, three existing tests' **doc comments** (`tests/mixer.rs` ×2, `tests/play.rs` ×2 — no code or assertion changed), `Play`/`Played::entity`'s docs. `examples/mixer.rs`'s `Parent` step still reads true (*"a sound hung from an entity, then the entity dropped"*); it was built, **not run** (it opens a window).
8. **The tests' vocabulary**: the pending state is built by `world.entity_allocator().alloc()` and `spawn_at` — deterministic, where an unordered system in a probe reproduces the panic only by the luck of a topological order.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*Not the item's own cases. What would a game that did not read this iteration's notes meet?*

**Critique, first reading:**

1. **A projectile spawned with a whoosh** (`Commands::spawn` and `Play::cue(..).on(e).at(..)` in one system): no panic in any ordering; the sound is at `at` from its first frame and follows the projectile once its `GlobalTransform` exists (one frame later). **Run** (`same-frame unordered`, `pending`, and the ninth test).
2. **A vehicle's engine on a moving owner with a `Transform`**: placed with `.at(vehicle_position)` it is at the vehicle and follows it, pan and all; **the level law does not track** (compensation fixed at the play) — as documented for the mixer, and shared by Bevy's path. Not new.
3. **`.on(owner)` of a spatial cue with no `.at(..)`**: still flat — placed only where it was asked to be (`tests/play.rs::a_spatial_cue_without_a_position_is_still_flat`). The most natural use of `on` for an engine is therefore still not spatial; carved off, with its pin.
4. **A game that moves a voice's `Transform` itself while the voice has an owner**: the crate's follow overwrites it each frame. To move a voice yourself, give it no owner (`VoiceOwner`'s doc).
5. **A game on the `mixer` feature with a device**: an owner gone at the play ends the sound before `start_pending` — the output is not asked (M5); a placed owned voice does not follow. Not run on a real device (the output is a fake in the test).
6. **The first-sight and steady costs**: 0.05 µs a frame for a game that owns nothing, 1.5 – 1.6 µs for 256 owned, placed voices (`perf.md`); `perfcheck` cannot see it, and reads inside the band (0.057 – 0.062 / 0.304 – 0.359 / 1.053 – 1.225 µs).
7. **The README's quick start still does not play a sound as written** — found in iteration 118, unchanged here, **first in the next pass**.
8. **Nobody listened, and there is nothing new to hear**: a refusal or a placement makes no sample the render writes (byte-identical). **The Cauldron's audio panel was not run** (the editor pins `82d227e`, 221 commits back; its 14 GB target does not fit this disk); no API the panel uses changed except the removal of a public system nothing in `bevy_grimoire` names.

**Verdict: GREAT.**

## Prediction scorecard (wrong ones first)

| # | prediction (`research.md`, written before the fix) | result |
|---|---|---|
| — | *not predicted*: a play on a gone owner would steal a playing voice; the follow would measure against an unpropagated `GlobalTransform` | **two gaps, both found by tests** (Pass 1, items 1 – 2) — neither was a wrong number, both were states I had not thought to build |
| P6 | a placed voice follows its owner **within one frame**; a rotated owner turns the offset | **half-checked**: the follow and the rotation hold (`a_placed_sound_follows_its_owner`), the lag is by design and the test waits three frames |
| P3 | the level the player hears equals the law at `at` to 0.01 dB in all three arrangements | held, **by an oracle that shares the model's panner** (Pass 1, item 5) |
| P5 | no existing test is edited | held for code and assertions (five pass unedited); **four comments in two test files were edited** — `tests/mixer.rs` ×2, `tests/play.rs` ×2 (Pass 2, item 7) |
| P1 | a pending owner does not panic; the voice lives and ends with its owner | held (`pending`, `unordered`, `an_owner_that_is_not_spawned_yet_…`) |
| P2 | the voice's position is `at` exactly for an owner at the place, elsewhere, and with no `Transform` | held (error 0) |
| P4 | a loop on a gone owner: 0 voices, 0 counted, *played* then *stopped* | held |
| P7 | the crate's mixer's output is asked to play 0 voices for a gone owner (it was 1) | held (the test's control is 1, the gone owner adds 0; M5 reads 2) |
| P8 | ≤ 2 µs a frame at 256 owned, placed voices; nothing measurable with none | held: 1.5 – 1.64 µs; 0.044 – 0.058 µs |
| P9 | a voice the game despawns is no longer logged as *owner despawned* | held |

## Tool discipline

The user's standing instruction: read with `read`/`grep`, not bash; 4 – 8 round trips per edit become 1. Every file, source and doc was read with `read`, `grep` or `glob` (including Bevy's and `rodio`'s source, GitHub issues through `issue://`), in parallel batches; edits went in as multi-hunk, multi-file `edit` calls with the numbering of the last read, and after each `cargo fmt` I re-read before the next edit. Bash ran **programs**: `cargo` (build, test, clippy, fmt, doc, run, `clean -p bevy_siren`), `ffmpeg`, `git`, `python3` (the repository's transcript renderer), and `cp`/`rm` of files this pass made or of artifacts. Evidence that the tool kept is `artifact://…` copied with `cp`; the two matrices arrived with `wait` and were written with `write`. **The avoidable ones**: `grep -E` pipelines after `cargo test` to keep only the result lines, and four `for` loops that ran the probe's modes in one bash call; the mutation checks were two calls each (a combined restore-and-mutate edit, then the run). I did not count the calls and will not give a number.
