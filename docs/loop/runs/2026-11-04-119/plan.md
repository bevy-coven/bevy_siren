# Iteration 119 — plan: a voice belongs to its owner without being its child

## Ranked

| # | item | rank | why it is (not) this pass's |
|---|---|---|---|
| 1 | **`Play::on` makes the voice a child of its owner (`ChildOf`), and a child is the wrong tool**: a panic when the owner is allocated and not yet spawned; a sound placed at *owner ∘ at* (12.04 dB quiet, or +52.04 dB over the sample for a Transform-less owner); a loop that lives for ever when the owner is already gone; a false *owner despawned* line when the game despawns the voice | bug — a crash on a documented use, a wrong level and pan, a leak, and a lie in the log, from one line | **this one**: reproduced and deterministic (`pending`); one mechanism fixes all four; no new `Refusal` variant; the crash depends on an arrangement a game does not control |
| 2 | The README / crate-root quick start does not play a sound as written (keys with no extension; `walk` and `hear` never scheduled; `tests/readme.rs` compares text over a `no_run`) | docs / front door | a loud failure since 116 (every play is refused with its cause in the log). **First in the next pass**; evidence `runs/2026-11-04-118/evidence/before/front-door-probe-on-the-unchanged-tree.txt` |
| 3 | The layers of a score start together (3 of 100 warm-cache starts; design in `runs/2026-11-04-116/plan.md`) | behaviour | carry-over |
| 4 | A person listens | owed | carry-over |

**Why 1 beats 2**: a crash in the game, a level 400× too loud and a leak on the crate's headline use of `on`, against a front-door defect that already fails loudly. Item 1 *absorbs* the third carry-over from 118 (`Play::on` for a vanished owner) and finds three more defects of the same cause.

## Design

**`VoiceOwner(pub Entity)`** — a new public component in `src/play.rs`, re-exported at the crate root — replaces `ChildOf(target)` in `spawn_voice`. It is data: no hook runs, no transform is inherited, nothing is despawned for it.

**`owner::end_and_follow`** (new `src/owner.rs`, `pub(crate)`) runs **directly after `play_requests`** in the `SirenSystems::Play` chain — so a voice on a gone owner is despawned before `start_pending` can start it on the crate's own mixer. For each voice with a `VoiceOwner` it asks `&Entities::get_spawned(owner)`:

- `Ok` (spawned): keep; if the voice carries `Follows` (a placed voice, on Bevy's path) and the owner has a `GlobalTransform`: the first time, fix `offset = inverse(owner) · the voice's world position`; every frame after, write `Transform.translation = owner ∘ offset` (only when it changed);
- `Err(ValidButNotSpawned)` (the spawn command is queued): wait;
- `Err(Invalid)` (gone): write `Entry::Stopped { why: "owner despawned" }` and `despawn` — in the frame the voice was played if the owner was already gone.

`Follows(Option<Vec3>)` is crate-private, inserted for `started.spatial && !mixer_owns && target.is_some()` (the mixer does not pan and attenuates once, at the play). **`at` stays a world position**, so the crate's own law, its compensation and Bevy's panner see the same place.

**Removed** (clean cutover): `mix::log_despawned_voices` and its registration (`lib.rs`) — its only reading of *owner* was `ChildOf`, and the new system writes the line itself; `ChildOf` from `mix::EndedVoice` (an unused `_owner`). **The public surface**: `+ VoiceOwner`, `− mix::log_despawned_voices`, `mix::EndedVoice` loses its last element. Recorded in `needs-decision/play-on-owner.md` (options, costs, the recommendation, *recommend, record, proceed*).

**Docs**: `Play::on`/`Play::at`, `docs/guide/buses.md`, `AGENTS.md` (layout and a trap), `CHANGELOG.md`, the `mix.rs` doc of the removed system's neighbours, one comment in `tests/play.rs` that says a *parented* voice.

## Tests (written first; red on the unchanged tree)

New module `tests/owner.rs` (declared in `tests/all.rs`):

1. `an_owner_that_is_not_spawned_yet_does_not_crash_the_game` — `world.entity_allocator().alloc()`, `Play::on`, a frame, `spawn_at`, frames, despawn: no panic; the voice lives while the owner does and ends with it with one line.
2. `a_sound_on_an_owner_that_is_already_gone_ends_in_its_frame_and_says_so` — a loop: 0 voices, 0 counted, *played* then *stopped: owner despawned*.
3. `a_sound_is_where_it_was_asked_to_be_whatever_its_owner_is` — `TransformPlugin`, a listener, a spatial cue: the voice's `GlobalTransform` is `at` exactly, for an owner at the same place, an owner elsewhere and an owner with no `Transform`; and the level the player hears (`gain_db + compensation + the panner's min(1, 1/d²)` at the voice's real position) equals the crate's law at `at`.
4. `a_placed_sound_follows_its_owner` — translate, then rotate 90°.
5. `a_game_that_despawns_a_voice_is_not_told_the_owner_did` — no *owner despawned* line.
6. (`mixer` and default) `a_gone_owners_voice_is_never_started_on_the_crates_mixer` — a fake output asked to play 0 voices.

**No existing test's code is edited** (P5): the five that read an owner's or a voice's despawn must pass as they are.

## Carved off

- A spatial cue played `.on(owner)` with **no** `.at(..)` is not placed at the owner (`spatial = cue.spatial && position.is_some()`; `tests/play.rs::a_spatial_cue_without_a_position_is_still_flat` pins *"heard by everyone at the cue's own gain"*): a behaviour change, its own decision.
- The compensation of a **moving** emitter is fixed at the play (it divides out the panner's law at the play-time distance); the pan tracks, the level law does not. Documented in `spatial.md` for the mixer; Bevy's path shares it.
- Item 2 above, with its evidence.

## What happened to the plan

- **A play on a gone owner took a playing voice's place.** Not in the plan: the red run on the unchanged tree already showed it (`stolen by engine (cue)` in the mixer test's log) and I read past it until the green run made the same line appear with the fix in. `play_requests` now computes `doomed` — the owner is gone — and skips the steal pass, the live list and the duck for such a play; `a_play_on_a_gone_owner_takes_nothing_from_a_sound_that_is_playing` is red when that is switched off and no other is. Two tests were added to the six planned (`a_voice_names_its_owner_and_is_not_its_child`, and that one).
- **The follow measured against an owner that had not been propagated.** Not in the plan either: the ninth test, written after the first full matrix for the arrangement that matters most — a placed sound on a projectile spawned with a whoosh — was **red on the first form**: *"the sound has not moved: [200, 0, 0]"*. A `GlobalTransform` added since the system last ran is still the default, so the offset was measured against the origin and the sound jumped to owner ∘ offset a frame later. `end_and_follow` now waits while `Ref<GlobalTransform>::is_added()`. The matrix was run again on the final tree.
- **Disk.** The disk reached 0 GB between the tests and the release builds (a linker `errno=28` under `cargo test --features mixer`, nothing in the code); `cargo clean -p bevy_siren` (this crate's own artifacts, made this session) freed 6.8 GiB, as the skill prescribes.
