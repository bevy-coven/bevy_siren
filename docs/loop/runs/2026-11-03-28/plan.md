# plan.md — iteration 28: a despawned voice frees its mixer slot

## The pick, and why it beats the runner-up

**Pick:** on the mixer backend, despawning the entity a `Play::on` sound is parented to **never frees the voice's slot**.
The mixer number lives in a component on that entity, the despawn destroys it, and `stop_voice` is never called — the
`rodio` player keeps sounding with its slot held for ever. Bevy's path is unaffected (its sink is on the same entity and
dies with it), so this is also a **two-backend divergence**, which the crate's own rule forbids.

Beats the runner-up (a `SirenLog` line for the despawn): that is a missing **record**; this is a **leaked resource**, and
it is measurable on one path and absent on the other.

## The design

In `playback::start_pending`:

1. **`starts: Local<Vec<(Entity, String, usize)>>`** — every voice this system handed to the mixer.
2. **`playing: Query<(Entity, &MixerVoice, &Voice)>`** — what the mixer is playing right now.
3. **One `retain` before the start loop**: a start whose entity is not in `playing` was despawned by its owner, so
   `output.stop_voice(bus, index)` runs once and the entry is dropped.

No new public type (`Local` is a system parameter), no new trait method (`stop_voice` exists), no change to Bevy's path.

## Fields

None. No `CueTable`, `SirenSettings` or `MusicScore` change; no public type; no schema.

## Tests

- `a_despawned_voices_mixer_slot_is_freed` (behind `mixer`) — one parented loop voice started, the owner despawned, then
  `stop_voice` is called **exactly once** with its index, and the mix counts none. **Fails on the base commit**
  (`left: []`).
- The **guard is inside the same test**: after ten frames with the owner alive, `stopped` is still empty — the
  reconciliation does not touch a live voice.
- Every existing test on both feature sets must stay green, because this changes the mixer's bookkeeping.

## The carve

- **Not** a `SirenLog` line for the despawn: a record, not a leak; it needs a new `Entry` reason or a decision, and
  `needs-decision/` already carries five. Written down as the next item.
- **Not** a component `on_despawn` hook: Bevy 0.20 has one, but `DeferredWorld` cannot reach the `Output` resource, so it
  cannot call `stop_voice`.
