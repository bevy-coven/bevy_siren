# `Play::on`: a voice belongs to its owner, and is not its child

*Iteration 119, 2026-11-04. A public component is added and a public system is removed; built under the standing "recommend, record, proceed" rule (`assumed-decision`).*

## What changed in the public surface

- **Added**: `bevy_siren::VoiceOwner(pub Entity)` — a component on a voice that `Play::on(owner)` made. It says whose the voice is.
- **Removed**: `bevy_siren::mix::log_despawned_voices`, a `pub fn` system whose only purpose was to read `ChildOf` as *"the owner despawned this voice"*; `owner::end_and_follow` (crate-private) writes the same `stopped … owner despawned` line itself, so the line is written once and only when the owner is the cause.
- **Changed**: `mix::EndedVoice`, a `pub type` alias used by `retire_voices`, loses its last element (`Option<&ChildOf>`, which was unused). Nobody outside the crate has a reason to name it.
- **Behaviour**: a voice made by `Play::on` is **no longer a child** of its owner — no `ChildOf` on the voice, no `Children` on the owner. The sentences that did not change: the voice ends when the owner is despawned; the log says `owner despawned`; `Play::on`'s signature; `Played`, `Refused` and `Refusal` (no variant is added).
- **Behaviour, the fix**: `Play::at` is a **world** position whatever the owner is (it was composed with the owner's transform); a placed voice on Bevy's path follows its owner's `GlobalTransform` (a lag of at most one frame); an owner not yet spawned is waited for (it crashed the game); an owner already gone ends the sound in its frame (it left a loop for ever); a voice the game despawns is no longer blamed on the owner.

## Why

Run as a game would run it, `ChildOf(owner)` gave four defects (`runs/2026-11-04-119/evidence/before/`): **a panic** in Bevy's command handler for an owner that is allocated and not yet spawned (every `Commands::spawn` returns one); a sound at *owner ∘ `at`* — 12.04 dB under the crate's own law for an owner at the place, and **52.04 dB over the sample** for an owner with no `Transform` (Bevy never propagates a child of a parent that has none, so the voice sat on the listener while the crate's panner compensation stood); a loop that **lives for ever** on an owner that was already gone; and a **false log line** when the game despawned the voice itself. All from one line, because a relationship is three things at once — a lifetime, a transform and a hook — and `Play::on` wanted one.

## Options

| # | option | cost to a consumer | verdict |
|---|---|---|---|
| 1 | **`VoiceOwner` data + one system** that reads the owner's state (`Entities::get_spawned`: alive, pending, gone), ends the voice when it is gone and keeps a placed voice at its owner's side (this change) | a game that queried `ChildOf`/`Children` to find a voice's owner uses `VoiceOwner` instead; `log_despawned_voices` is gone | **chosen**: one mechanism, four defects, no new `Refusal` variant |
| 2 | Keep `ChildOf`; set the voice's *local* transform to the owner's inverse times `at`; guard the insert against a pending owner; add a system for a gone one | none visible | two lifecycles (a hierarchy for an owner with a `Transform`, a data component for the rest), a hook that still runs, and a Transform-less owner still cannot be followed — the same system is needed anyway |
| 3 | Refuse a play on a gone owner with a new `Refusal::OwnerGone` | every consumer's exhaustive `match` on `Refusal` breaks (`needs-decision/undecodable-sound.md` weighed this for causes of `MissingSound`) | no: the sound is answered `Played` and ended in its frame, the way a same-frame steal already is, and the log says why |
| 4 | Stop parenting and do not follow | a moving emitter's voice stays where it was put on Bevy's path | no: `AGENTS.md`'s *"Bevy's path stays the default for moving emitters"* is an advertised use, and the hierarchy gave it by accident |

## Edge cases and what is decided

- **A pending owner is waited for with no bound.** It cannot stay pending past the end of the frame in which its spawn command applies; an id reserved and never spawned is a game's own bug and costs one state check a frame.
- **A gone owner's voice is despawned at once, not faded** — the behaviour the hierarchy gave (a cascade), pinned by `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so`.
- **A play on an owner that is already gone is answered `Played` and ended in that frame**, and takes **nothing** from the budget: no steal, no slot, no duck (`play_requests`'s `doomed`). Whether such a play should be *refused* is option 3's question; recorded, not decided.
- **The follow's lag** is the owner's previous-frame `GlobalTransform` (the crate writes in `Update`, Bevy propagates in `PostUpdate`): at 20 m/s, 0.33 m. An owner with no `Transform` has nothing to follow.
- **Not changed, and carved off**: a spatial cue played `.on(owner)` with **no** `.at(..)` is still not placed at the owner (`tests/play.rs::a_spatial_cue_without_a_position_is_still_flat`); and the compensation of a *moving* emitter is fixed at the play.
- **Cost**: `owner::end_and_follow` is 0.05 µs a frame with 256 voices none of them owned, and 1.5 – 1.6 µs with 256 owned, placed ones (about 6 ns a voice).

## Recommendation

Option 1, as built. Revisit option 3 only if a consumer wants to *know* a play on a gone owner did nothing without reading the log; then a variant that carries the entity is the thing to add, in one place.
