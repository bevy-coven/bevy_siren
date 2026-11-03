# research.md — iteration 15: a music layer's voice dies and the layer never notices

## The item

`MusicState` is a **mirror**: `start_layers` writes a `Play`, `adopt_layers` records the `Played::entity`, and
`move_layers` writes that voice's gain. The mirror is only ever **added to**. Nothing in the music plugin listens for a
voice it owns being stopped — by the game (`StopCue`), by the budget (`Stopping { why: "stolen by …" }`), or by a refused
first play. Three probes on the crate as it stands:

| edit | observed |
|---|---|
| `StopCue { cue: "music.calm" }` | live voices **0**, but `MusicState::voice("calm")` still returns the **despawned** entity, `live()` 1, `layer_db` −80 dB, and it never returns |
| a `ui.click` on the same bus with `max_voices: 1` | `stopped 'music.calm': stolen by ui.click` — the **stem is stolen by a UI click** |
| a stinger | no steal (the crate's `UNSTEALABLE_PRIORITY` protects it on paper) — but the same class |
| the layer's first play refused | `live()` 0 forever: `awaiting` was never cleared, so the layer is never retried |

## Sources

**S1. Gregory, *Game Engine Architecture*, 3e, §14.5.8.4** (vendored: `docs/research/siren.md` R4). A sound engine
supports a **fixed** number of voices; a voice is *"a hardware resource, and a stopped voice frees its resources for use
by other voices"*. A voice that stops is **not** the sound still playing — which is precisely what `MusicState` was
asserting. Gregory's engine keeps a *virtual channel* per logical sound: a sound that loses its physical voice is
**still a logical sound**, and the engine may make it physical again when a voice frees.

**S2. Khan et al. 2023, *Adaptive Background Music for a Fighting Game*, arXiv:2303.15734** (vendored: `siren.md` S3).
Adaptive music is *"a vertical layering technique, where the stems of a single piece are"* keyed to game state — the
stems **are** the music. A stem that stops for a reason the score did not decide is the music failing to be the music:
the player hears one fewer instrument, and the state says every instrument plays. S2's own design keeps every stem
running (a silent stem stays in the mix) exactly because a stem that restarts is out of phase with the others — which is
why the crate's guide says a silent layer is *"not stopped; it is mixed"*.

**S3. `UNSTEALABLE_PRIORITY` = 180, `docs/research/siren.md` R4 / the parameter table:** *"music, stingers and dialogue
are what a player listens to"*. The **intent** is documented; the **mechanism** is `priority >= 180` on the cue. A layer
authored with the crate's default `priority: 0` is therefore *called* music and *treated as* a footstep. The sentence in
`Voice::unstealable`'s doc is true of the value, false of the claim.

**S4. Bevy 0.20 `AudioSink` / `AudioPlayer::despawn_related`** (local source, `bevy-0.20.1/src/audio/audio.rs`): an
entity whose `AudioSink` is gone is an entity whose sound has ended; the sound is not resumable. There is no API that
re-attaches a sink to a live `AudioPlayer`, so a stem that was stopped is **restarted**, not resumed — and starting it
from the beginning is what S2 says breaks the phase alignment.

## The technique

A mirror of another system's resources must be **reconciled every frame**, not written once:

1. **Notice a dead voice.** A layer's voice is dead when its entity is gone (`despawned`) or `Stopping` (fading out
   because the budget or the game stopped it). `MusicState` clears it — so `voice()` never names an entity the world
   does not have and `live()` is true.
2. **Refuse the steal, not just the label.** A voice that **is a music layer** is unstealable *whatever* priority its cue
   authored. That is the crate's own R4 intent made a mechanism: `room_for` must know a voice is a stem. The cheapest
   honest way is for the music plugin to mark its adopted voice, which the crate already does — `MusicVoice { layer }`
   is on every adopted entity. `room_for`'s `Live` row gains `is_layer` from that component, and `Candidate`'s
   `unstealable` is `voice.unstealable || is_layer`. No public type changes.
3. **Retry a refused layer.** `awaiting` is an intent to start, and an intent that produced `Refused` is spent: the
   layer must be re-asked next frame while it has no voice. The current code pushes the intent and never removes it, so
   a layer whose first play was refused is blocked for the life of the process. The reconcile in (1) fixes this by keying
   the retry on `voice: None` and clearing `awaiting` when the answer is a `Refused`.

**What the crate does *not* do:** re-time a restarted stem to the bar, or keep a stopped stem's phase. S2 makes both
desirable and both need a decision about what a restart means after a game's explicit `StopCue` (a stop the game asked
for is *meant*, so restarting it would be wrong). Carved off; the rule here is the narrow one: **a voice the plugin did
not ask to stop, and that stopped anyway, is restarted; a voice the game stopped with `StopCue` is left stopped.**

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a stopped voice frees its slot | — | S1, §14.5.8.4 |
| a logical sound outlives its physical voice | — | S1 (virtual channels) |
| every stem of a piece is the piece | — | S2 |
| unstealable: music, stingers, dialogue | 180 | S3 / `siren.md` R4 |
| a stem that stops and restarts is out of phase | — | S2; `docs/guide/music.md` (*"keeps every layer phase-aligned"*) |
| restart delay | 0 frames | design default (the crate retries on the next frame; no source gives a number) |

## What good sounds like

A stem that lost its voice comes back **within one frame of the slot freeing**, at the gain the score says it should be
at — not at its authored starting gain, and not at silence. A stem the game explicitly stopped stays stopped, and a
panel reading `live()` agrees with the voices in the world at every frame. A UI click, a stinger and a footstep never
take a stem: the log never says `stolen by` with a `MusicVoice`'s cue, whatever priority the score's layer authored.
