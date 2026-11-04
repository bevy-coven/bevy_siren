# Iteration 119 — research: `Play::on` hangs a voice from its owner by making it a child, and a child is the wrong tool

*The finding came first, from a probe (`evidence/before/owner-probe-on-the-unchanged-tree.txt`), as in iterations 116 – 118 — said here rather than hidden. The predictions below are for the fix and were written before it was built.*

## What reading and running found

`Play::on(entity)` is documented as *"the sound belongs to `entity`"*; `Play::at(position)` as *"the sound is at `position` in the world"*; `docs/guide/buses.md:112` as *"a sound `Play::on` hung from an entity stops when that entity is despawned, and says so"*; and `Play`'s own doc example is `.on(foot).at(Vec3::new(2.0, 0.0, 1.0))`. The implementation of *belongs* is one line — `spawned.insert(ChildOf(target))` (`src/play.rs`, `spawn_voice`) — and a child is a hierarchy node: it inherits a transform, it is a member of the parent's `Children`, it is despawned with the parent, and inserting the relationship runs a hook. Run as a game would run them, the four situations the contract names give four defects, all from that line:

| situation | what happens on the unchanged tree |
|---|---|
| the owner is **allocated and not yet spawned** when the play is answered (a game that does `let e = commands.spawn(..).id(); play.write(Play::cue("c").on(e))` in a system unordered against the crate's) | **a panic in Bevy's command handler** (`bevy_ecs-0.20.1/src/error/handler.rs:128`, *"Entity not yet spawned"*): the relationship hook (`relationship/mod.rs:194`) queues a command on an entity that does not exist yet. Reproduced by an unordered system and, deterministically, by `world.entity_allocator().alloc()` |
| `.on(owner).at(world)` | the voice is at **owner ∘ at**: (200, 0, 0) for an owner at (100, 0, 0) and a world position of (100, 0, 0); **(0, 0, 0) — the listener — for an owner with no `Transform`**, because `propagate_parent_transforms` only reaches children of a root that has a `Transform` (`bevy_transform-0.20.1/src/systems.rs:334`) and `sync_simple_transforms` skips entities with `ChildOf`. The crate computed its level and its compensation for 100 m (−27.96 dB, +80.00 dB); the panner sees 200 m (**−12.04 dB** against the law) or ~0 m, where `min(1, 1/d²)` is 1 and the compensation stands (**+52.04 dB** written to the sink, 400× the sample, where the law says −27.96) |
| the owner is **already gone** | no `ChildOf` (Bevy warns and removes it), a `Mode::Loop` voice that lives for ever, counted on its bus, the log saying only *played* |
| the **game despawns the voice** while its owner lives | the log says *`stopped … owner despawned`* — false: `log_despawned_voices` reads `ChildOf` as *"the owner did it"* |

## Sources

| # | source | what it gives |
|---|---|---|
| S1 | Ousterhout, *A Philosophy of Software Design*, 2nd ed., **§10.3 Define errors out of existence** and §10.4 (Unix file deletion) (home-still `ousterhout_philosophy_of_software_design_2nd`) | *"The best way to eliminate exception handling complexity is to define your APIs so that there are no exceptions to handle"*; `unset` should *"ensure that a variable no longer exists"*, so unsetting an absent one is *"its work is already done"*. **"Belongs to"** defined as *"lasts as long as the entity does"* has no exceptions: an owner not spawned yet has not begun, one that is gone is already over |
| S2 | Hunt and Thomas, *The Pragmatic Programmer*, 20th ed., **Tip 38 Crash Early** (home-still `hunt_pragmatic_programmer_20th`) | *"when your code discovers that something that was supposed to be impossible just happened, your program is no longer viable"* — **and a pending owner is not impossible**: `Commands::spawn` makes it the ordinary state of every entity in the frame it is created. A crash there is a defect, not a design |
| S3 | Gregory, *Game Engine Architecture*, 3rd ed., **§14.4.2 – 14.4.3** (home-still, `Game Engine Architecture, Third Edition…`) | a 3D sound source *"emanating from a specific position"*; *"the positions of the source and listener are used for distance-based attenuation"* and *"the orientation of the listener defines a reference frame in which the angular position of the sound is calculated"* — the level and the pan come from **one** position, and the crate's two (its own law, the backend's panner) must be given the same one |
| S4 | Zotkin, Duraiswami and Davis, *Rendering Localized Spatial Audio in a Virtual Auditory Space*, IEEE Transactions on Multimedia, 2004, DOI 10.1109/tmm.2004.827516, the game application | targets *"playing different sounds … their intensities and spatial positions agree with current position of the player in the world. … one of them gets destroyed and the corresponding sound ceases"* — an emitter that tracks its object's current position and ends with it: the semantics `Play::on` documents |
| S5 | Bevy 0.20.1, local: `bevy_ecs/src/entity/mod.rs:846-862` (`Entities::get_spawned`: a stale generation is `Invalid`, a valid unspawned one is `ValidButNotSpawned`; `&Entities` is a `SystemParam`, `system_param.rs:1504`); `bevy_audio/src/audio_output.rs:128-133, 330-344` (an emitter's position is its **`GlobalTransform`**, re-read when it changes); `rodio-0.22.2/src/source/spatial.rs:63-64` (`min(1, 1/d²)` per ear) | **the three states of an owner are distinguishable in a system**, which is what makes the fix safe: *pending* is waited for, *gone* ends the voice |
| S6 | this repository: `AGENTS.md` ("a rodio source cannot be re-positioned … Bevy's path stays the default for moving emitters"), iteration 118's `Played::entity` contract and `needs-decision/audio-output-decode-error.md` | a placed voice that follows a moving owner is an advertised use on Bevy's path, so **removing the hierarchy must not remove the following**; additive public items have precedent (record, recommend, proceed) |

## The technique

**Ownership without hierarchy.** A voice carries `VoiceOwner(Entity)` — data, not a relationship: no hook runs, no transform is inherited, nothing is despawned for it. One system, directly after `play_requests` in `SirenSystems::Play` (so a gone owner's voice is despawned before `start_pending` can start it on the crate's mixer), asks `&Entities` about each owner:

- **spawned** — keep the voice; if it is *placed* (a spatial cue with a position, on Bevy's path) it **follows**: the first frame the owner has a `GlobalTransform`, the voice's offset in the owner's space is fixed (`inverse(owner) · the voice's world position`), and every frame after, its `Transform` is `owner ∘ offset`. `at` stays a **world position**, so the crate's level, the compensation and the panner all see the same place;
- **pending** (`ValidButNotSpawned`) — wait; the command that spawns it is queued and applies before the next frame;
- **gone** (`Invalid`) — write `stopped: owner despawned` and despawn the voice, in the frame it was played if the owner was already gone.

`log_despawned_voices`, whose only reading of *owner* was `ChildOf`, goes: the new system writes the line, so a voice the game despawns is no longer blamed on its owner.

## Parameters

| thing | value | source |
|---|---|---|
| a pending owner | waited for, with no bound | design default: it cannot stay pending past the end of the frame in which its spawn command applies; a reserved id never spawned is a game's own bug and costs one entity-state check a frame |
| the follow's lag | the owner's `GlobalTransform` of the previous frame (≤ 1 frame) | design default; the crate writes in `Update`, Bevy propagates in `PostUpdate` |
| which voices follow | placed ones (`spatial` cue **and** a position) on Bevy's path | S6: the mixer does not pan and applies attenuation once, at the play |
| a gone owner's voice | despawned at once, not faded | the behaviour the hierarchy gave (a cascade); `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` pins it |

## What good looks like

`.on(owner).at(world)` puts the voice at `world`, whatever the owner is and wherever it is, and **moves it with the owner**; a play on an owner spawned in the same frame neither panics nor loses its owner; a play on an owner that is gone ends in its frame and says so; the log never blames an owner for a despawn the game made. The existing owner tests pass **unedited**.

## Predictions (for the fix; written before it was built)

- **P1.** A play on an allocated-not-spawned owner does not panic; once the owner is spawned the voice is alive and owned, and despawning the owner ends it with one `owner despawned` line.
- **P2.** `.on(owner).at(world)` puts the voice's `GlobalTransform` at `world` exactly (error 0 m) for an owner at the same place, for an owner elsewhere, and for an owner with no `Transform` — it was 100 m and 100 m off, and at the listener.
- **P3.** The level the player hears — `Played.gain_db + Compensation.db` plus the panner's `min(1, 1/d²)` at the voice's real distance — equals the crate's law at `at` to within 0.01 dB in all three arrangements; it was −12.04 dB and +80 dB off.
- **P4.** A loop played on a gone owner is despawned in that frame: 0 voices, 0 counted, the log reads *played* then *stopped: owner despawned*.
- **P5.** **No existing test is edited**: the five that read an owner's or a voice's despawn — `tests/play.rs::{a_despawned_owner_stops_its_sound_and_says_so, a_stop_for_an_ended_voice_is_a_no_op}`, `tests/mixer.rs::{a_despawned_voices_mixer_slot_is_freed, a_despawned_owner_is_logged_once}`, `tests/music.rs::a_layer_whose_voice_died_is_restarted` — all pass.
- **P6.** A placed voice follows its owner: translate it by (10, 0, 0) and the voice's world position moves by (10, 0, 0) within one frame; rotate it 90° about Y and an offset of (1, 0, 0) becomes (0, 0, −1).
- **P7.** On the crate's mixer a gone owner's voice is never started (the output is asked to play 0 voices; it was 1).
- **P8.** The new system costs ≤ 2 µs a frame with 256 owned, placed voices and nothing measurable with none.
- **P9.** A voice the game despawns while its owner lives is no longer logged as *owner despawned*.
