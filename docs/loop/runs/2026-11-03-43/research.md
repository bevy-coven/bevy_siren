# research.md — iteration 43: a sound whose owner was despawned says it stopped

## The item

`Play::on(entity)` parents the voice to a game entity with `ChildOf(target)`, so despawning that entity — the ordinary
way a sound on a vehicle, a torch or an enemy ends — takes the voice with it. **Neither backend said so.**

- The crate's own mixer freed the slot in `start_pending` since iteration 7 — measured then, because the mixer number
  lived in a component on the despawned entity and the `rodio` player otherwise sounded for ever — and wrote **no line**.
- Bevy's path was worse: the voice entity is simply gone, so `retire_voices` (which runs during `Mix`, one frame *before*
  Bevy applies the despawn) never sees it.

Measured: `tests/mixer.rs::a_despawned_voices_mixer_slot_is_freed` asserted the slot was freed and
`voices_on("Effects") == 0`, and never read `SirenLog`. The panel's log's last word about such a sound stayed `played`
while the mix counted one fewer voice.

## Sources

**S1. `Play::on`'s doc** — *"parent the voice to `entity`, as a game does for an engine on its vehicle"*. The parent's
despawn is the **ordinary** end of such a sound, not an edge case.

**S2. `retire_voices`** — writes `Entry::Stopped { why: "ended" }` when a sample ends. Every other end is logged:
`"ended"`, `"stop"`, `"cue stopped"`, `"stolen by X (bus|cue|table)"`. A despawn is the one end with no line.

**S3. `Entry::Stopped`'s doc** — *"A voice stopped, and why"*. The vocabulary is open-ended by construction: a new reason
is a sentence, not an enum variant, so this needs no decision file.

**S4. Bevy 0.20.1, `bevy_ecs-0.20.1/src/lifecycle.rs`** — `RemovedComponents<T>` is a message reader over component
removals, *"Despawning an entity counts as removing all of its components."* The idiom for "this entity went away".

**S5. R6 (vendored: `docs/siren.md`)** — *"everything a play decides is readable"*. A sound that ended is a decision.

## The technique

`log_despawned_voices`, a system after `SirenSystems::Mix`, keeps a `Local<BTreeMap<Entity, (String, bool)>>` of every
voice it saw — its cue name and **whether it had a `ChildOf`** — and reads `RemovedComponents<Voice>`. A voice that
vanished and was **parented** earns one `Entry::Stopped { why: "owner despawned" }`; a voice a game despawned without
parenting does not, because `Play::on` is the only path that inserts `ChildOf`.

Three details, each paid for:

- **The mixer's own line was removed.** Writing it in `start_pending` *and* here put `owner despawned` in the log twice
  on the mixer — the iteration-43 test caught it. One despawn, one sentence, both backends.
- **The name is recorded before the despawn**, because `Voice` is gone by the time the removal is read.
- **`ChildOf` gates it.** `tests/play.rs::a_stop_for_an_ended_voice_is_a_no_op` pins that a game despawning its own
  unparented voice logs nothing, and the first draft reddened it.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a parented voice's ordinary end | the owner's despawn | S1 |
| every other end is logged | `"ended"`, `"stop"`, `"cue stopped"`, `"stolen by …"` | S2 |
| the new reason | `"owner despawned"` | **design default** — it names the mechanism, as `"ended"` names the sample |
| only `Play::on` inserts `ChildOf` | — | S1, verified by the reddened test |
| the line appears one frame after the play | measured 0.32 → 0.34 s | the capture |

## What good sounds like

`0.67s played \`engine.idle\` (engine_loop) on Effects at -14.00 dB` followed by
`0.68s stopped \`engine.idle\`: owner despawned`, with the bus column one voice lower — the log and the mix agreeing on
the one end path that had no line.
