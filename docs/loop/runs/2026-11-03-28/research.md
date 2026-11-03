# research.md — iteration 28: a despawned voice keeps its mixer slot for ever

## The item

`Play::on(entity)` parents the voice to a game entity — *"the entity the sound belongs to: a footstep's foot, an engine's
vehicle"*. Despawning that entity is the **ordinary** way a parented sound ends. On Bevy's path the sound dies with the
entity, because in Bevy 0.20 the `AudioSink` is a component **on the same entity** as the `AudioPlayer` (read in
`bevy_audio-0.20.1/src/audio_output.rs`). On the crate's own mixer the number for a voice lives in [`MixerVoice`], also a
component on that entity — so the despawn destroys the number, and nothing ever calls `stop_voice`:

```
parent despawned, mixer path
  mixer voices started: 1
  stop_voice calls:      []        <- the rodio player keeps sounding, its slot held for ever
```

**A leak, and a two-backend divergence**, which the crate's own rule forbids: *"the two backends answer identically …
only the sample path may differ."* Bevy's path answers "stopped"; the mixer's answers nothing.

Separately, and smaller: **neither** backend writes a `stopped` line for a parent's despawn. The log keeps `played` and
nothing else, which iteration 27's discipline would call the silent-failure class — but it is a *record*, not a runtime
leak, and it needs a new `Entry` reason or a decision, so it is written down and not built.

## Sources

**S1. `bevy_audio-0.20.1/src/audio_output.rs`** — the `AudioSink`/`SpatialAudioSink` is a component on the `AudioPlayer`
entity; `cleanup_finished_audio` only despawns for `PlaybackDespawnMarker`. A parent's despawn takes the child with it
through Bevy's hierarchy, and the sink dies with the entity. That is why Bevy's path is correct and the mixer's is not.

**S2. Gregory, *Game Engine Architecture* 3e, §14.5.8.4** (vendored: `siren.md` R4) — a voice is a **fixed resource** and
*"a stopped voice frees its resources for use by other voices"*. A slot held by a voice nobody can ever stop is the permanent
lockout that rule exists to prevent, arriving by the mixer's own bookkeeping.

**S3. The crate's two-backend rule** (`AGENTS.md`, `tests/mixer.rs`) — *"the two backends answer identically. A test must
pass with and without `--features mixer`; only the sample path may differ."* This is a second answer from the same input.

**S4. `start_pending`'s own doc** — *"This is the only place a voice starts twice-checked. A voice that has a `MixerVoice`
is playing; a voice with `Samples` and no `MixerVoice` is waiting."* The check is on the **entity**; an entity that no
longer exists is in neither class, and the doc's dichotomy has no third arm.

## The technique

`start_pending` keeps a **`Local<Vec<(Entity, String, usize)>>`** of the starts it handed out — entity, bus, mixer index —
and each frame retains only the ones still in `Query<(Entity, &MixerVoice, &Voice)>`. A start whose entity is gone is
stopped through the trait's existing `stop_voice`, so the slot is freed on the frame the despawn is noticed.

- **No new public type:** `Local` is a system parameter, private to the system that owns it.
- **No new trait method:** `stop_voice` is already there.
- **Bounded:** one `retain` over the mixer's live voices per frame — the same shape `retire_voices` uses, and the same
  size as the number of voices a game has.
- **Both backends keep their answer:** Bevy's path never had the leak and is untouched; the mixer now stops what the
  world took.

**Why not a component hook (`on_despawn`)?** Bevy 0.20 has one, but a hook gets `DeferredWorld`, which cannot reach the
`Output` **resource** — so it cannot call `stop_voice`. The reconciliation belongs where the trait object is.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the sink lives on the player's entity; a despawn takes it | — | S1 |
| a stopped voice frees its slot | — | S2 |
| the two backends answer identically; only the sample path differs | — | S3 |
| a voice with `MixerVoice` is playing, without is waiting — and a despawned one is neither | — | S4 |

## What good sounds like

A game despawns a van: on Bevy's path the engine stops and the slot was never held; on the mixer's path it now stops
**on the same frame** and the slot is free for the next engine. Measured: `stop_voice` goes from **never** to exactly one
call, and a voice whose owner is still alive is untouched. No sound changes on either path.
