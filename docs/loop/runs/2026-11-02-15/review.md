# review.md — iteration 15, three passes

## Pass 1 — function

**The claim.** A music layer is a **voice**, and `MusicState` is a mirror of voices it does not own. The mirror was
written once (`adopt_layers`) and never reconciled, so it could name a despawned entity, count it live, and leave a stem
silent forever. Three paths, all measured:

| row | source | before | after |
|---|---|---|---|
| the game stops a stem (`StopCue`) | `StopCue`'s doc, *"every voice of a cue"* — the crate was told to stop, so it must know it stopped | `voice()` `Some(47v0)` (**despawned**), `live()` 1 | `None`, `live()` 0, `layer_db` −80 dB |
| a `ui.click` beside a `priority: 0` stem | S3 / R4, *"music, stingers and dialogue are not stolen"*; `Voice::unstealable`'s doc | `voices ["ui.click"]`, **`track.calm stolen by ui.click`** | `voices ["track.calm"]`, no steals; the click is **refused** |
| a stem whose voice died | S1, §14.5.8.4 — *a logical sound outlives its physical voice* | `Some(47v0)` → `Some(47v0)`, still dead, `live()` **1** | `Some(47v0)` → `Some(48v0)`, `live()` 1 |
| a refused first start | the same R4 — a budget that refuses means *virtual*; a virtual sound is promoted when a voice frees | `live()` **0** forever | `live()` 1; refusals **1 → 1** |

**Against S2 (Khan et al.):** adaptive music is *"a vertical layering technique, where the stems of a single piece"* are
the music. A stem silently missing is the music not being the music — and `docs/guide/music.md` already says a silent
layer is *"not stopped; it is mixed"*, which the steal contradicted.

**The carve, stated:** a restarted stem starts from the beginning and is out of phase with the others. Re-timing it to
the bar is a decision the score does not carry and needs a `needs-decision` file; named as item 1 in `log.md`.

**Verdict: GREAT.** All four rows are tests, all four fail on the base commit, and the three paths have one root.

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack,tail}.png`, the loudness table, the two probe files.

**Audio unchanged where nothing died:** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS, `footstep-grass`
−18.4 LUFS / −6.1 dBFS (looped past the gating block), `mix.wav` −4.3 LUFS / +0.2 dBFS — every number identical to
iteration 14, which is the point: this is a **state** fix, and a change to a rendered cue would mean it had touched the
mix.

**The spectrograms** show the same clean attack and tail as iteration 14: no vertical full-band line (a click), no flat
shelf at the noise floor (a denormal tail), no hole.

**Two defects found in my own work, both by the gate:**

1. `adopt_layers` first recorded `awaiting.len()` as the count at a refusal — the number of *pending intents*, not the
   number of *voices*. The retry condition then compared against the wrong quantity and the layer never retried
   (probe 27 showed `live() 0` after the bed freed). Fixed by reading the live count where the answer arrives.
2. The first version of `LiveLayer`'s stop handling left a game-stopped layer's `db` at the gain of its last playing
   frame, so `layer_db` reported music that no voice was producing. Fixed: a stopped layer reads `SILENT_DB`.

**Verdict: GREAT** after the two fixes (both returned to pass 1, whose rows re-ran green).

## Pass 3 — fresh eyes

**A different shape.** The four tests use a **one-layer** score, a **`priority: 0`** cue (not the fixtures'
`priority: 200`), and a **different voice count** (`max_voices: 1`) from every existing music test; the probe uses
`Mode::Once` one-shots against loops. The restart test despawns the entity directly rather than going through a
`StopVoice`, because it must model a death **the plugin did not ask for** — the distinction the whole design rests on.

**Can a developer understand it without the code?** `docs/guide/music.md` gains a *"When a stem's voice dies"* section
with the three causes, what each does, and the phase caveat; `MusicState`'s accessors keep their meanings and `live()`
now means what it says. A panel drawing `live()` and `layer_db` shows the truth on every frame.

**What a fresh reader would still ask:** *why is a restarted stem out of phase?* Because Bevy has no way to re-attach a
sink to a player (`AudioPlayer::despawn_related` — the sound is not resumable), so a restart is a new play from sample
zero. Named, not hidden.

**Verdict: GREAT.**
