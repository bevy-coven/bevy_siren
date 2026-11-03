# research.md — iteration 16: a layer naming a cue that ends machine-guns the play path

## The item

`MusicScore::problems` carries the sentence **"music layer `calm` names cue `hit`, which is not a loop, so it cannot
hold a layer"**. The plugin never reads it. `start_missing` writes a `Play` for every layer with no voice, and a
`Mode::Once` cue's voice ends — so the layer is asked for again on the next frame, and the next:

```
a layer naming a one-shot:  10 plays in 10 frames, Refused 0, no log line saying anything is wrong
a layer naming an unknown cue:  Refused 1, `refused nope: no cue of that name`, live() 0
```

The asymmetry is the bug. An unknown cue is **refused with a reason**; a one-shot cue is **started, restarted every
frame, and never mentioned**. Before iteration 15 the same score was merely silent (the dead voice's entity stayed in
the mirror and nothing retried); the restart rule turned silence into a machine gun.

## Sources

**S1. Khan et al. 2023, *Adaptive Background Music for a Fighting Game*, arXiv:2303.15734** (vendored:
`docs/research/siren.md` S3). Adaptive music is *"a vertical layering technique, where the stems of a single piece are"*
keyed to game state, *"with the same tempo and beat throughout"*. A **stem** is a continuously running part of one
performance. A layer whose cue **ends** is not a stem: it is a one-shot that the layering model has no place for — the
next play starts at sample zero and the part is out of phase with every other layer, which is exactly what S1's
"same tempo and beat throughout" forbids.

**S2. Gregory, *Game Engine Architecture* 3e, §14.5.8.4** (vendored: `siren.md` R4). A loop is a voice with no natural
end; it is stopped by the engine, not by its sample. A `Mode::Once` voice **ends**, and `retire_voices` despawns it —
by design (`retire_voices`' doc: *"a `Mode::Loop` one lives until it is stopped, because a loop has no end to
notice"*). So a layer asking for a once cue is asking for a voice whose end it cannot survive.

**S3. `MusicScore::problems`' own sentence** (`src/music.rs:101`), and `Cue::mode`'s doc. The crate already knows the
rule and states it to a host; the failure is that the **report** is the only place it is enforced.

**S4. `docs/guide/music.md`**, *"the layer at `SILENT_DB` stays in the mix. It costs one silent voice and it keeps every
layer phase-aligned — which is the whole point: an entry that starts from the first beat…"* — the crate's stated reason
a layer is a loop.

## The technique

A layer must start a voice its cue says will not end. Enforced **where the play is decided**, not only in a report:

- `start_missing` reads the table. A layer whose cue is not a lock `Mode::Loop` is **not started**, and the reason goes
  into the log the first time the layer is seen — one line per layer, not one per frame.
- The score stays in `MusicState` and the layer stays in `names()`: a bad cue is a bad **cue**, and the author fixes it
  in the table, at which point the next frame starts the layer. The layer is not silently dropped, which would hide
  the mistake a second way.
- The retrigger is bounded even for a cue that *becomes* a one-shot while it plays: the check is on the cue the layer
  names **this frame**, so a table edit that breaks a live layer stops its restart.

**A new `Refusal` variant** (`NotALoop { layer, cue }`) is the honest message, and it is *public API* — a decision file,
not a unilateral change. This iteration does the part that needs no decision: **do not start, and say so once in the
log**. The log line carries the same words a refusal would.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| adaptive-music stems run continuously, same tempo and beat | — | S1 |
| a loop has no natural end; it is stopped, not ended | — | S2 |
| a `Mode::Loop` voice is skipped by `retire_voices` | — | S2 / `retire_voices`' doc |
| one silent voice per layer keeps the phase | — | S4 |
| one log line per bad layer | — | design default (a line per frame is noise; one per layer is a report) |

## What good sounds like

A score with a `Mode::Once` layer plays the **other** layers and logs one sentence naming the offending layer and cue,
in the words `problems()` already uses. It does not machine-gun, it does not go silent without a word, and the moment
the table's cue becomes a loop the layer starts on the next frame with no further action. Rendered cues are unchanged:
this is a refusal of a score, not a change to a sound.
