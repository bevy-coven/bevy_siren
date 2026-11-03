# review.md — iteration 16, three passes

## Pass 1 — function

**The claim.** A `MusicScore` layer naming a cue that **ends** is not a sound the layering model can hold. The crate
already said so (`MusicScore::problems`: *"names cue `hit`, which is not a loop, so it cannot hold a layer"*), and the
plugin started it anyway — then iteration 15's restart rule asked for it again **every frame**.

| row | source | before | after |
|---|---|---|---|
| a `Mode::Once` layer, its sample ending | S1 (a stem runs throughout, same tempo and beat); S2 (`retire_voices` skips loops, so a once voice is *retired*) | **10 plays in 10 frames**, `live()` 1, log `played hit` | **0 plays**, `live()` 0, one line: `refused hit: music layer calm names cue hit, which is not a loop, so it cannot hold a layer` |
| a `Mode::Loop` layer | S4 (*"one silent voice … keeps every layer phase-aligned"*) | 1 play, `live()` 1 | **unchanged** — 1 play, `live()` 1 |
| a layer naming an unknown cue | `Refusal::UnknownCue` | `refused nope: no cue of that name` | **unchanged** |
| the table is fixed to `Mode::Loop` | the cue the layer names **this frame**, not the one it was created with | never starts | starts on the next frame |

**Against the sources.** S1's *"same tempo and beat throughout"* is only true of a part that keeps running; a one-shot
restarted at frame rate is a drone, not a stem. S2 is the mechanism: `retire_voices` **despawns** a once voice, so a
layer's mirror loses it and the restart rule immediately asks again — the fix had to be *do not ask*, not *retry more
slowly*.

**The asymmetry that made it rank 1:** an unknown cue got a refusal and a line; a one-shot cue played at frame rate
with **no line at all**. A game reading `Refused` saw nothing, and so did a person reading `SirenLog`.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack}.png`, the loudness table, both probe files.

**Audio unchanged, which is the point.** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS,
`footstep-grass` −18.4 LUFS / −6.1 dBFS (looped past the gating block), `mix.wav` −4.3 LUFS / +0.2 dBFS — every number
identical to iterations 14 and 15. The attack spectrogram shows a clean full-band onset that cuts at 0.033 s with no
vertical line at the cut: no click, no denormal shelf, no gap.

**The measured table**, from `evidence/one-shot-layer{,-before}.txt`:

```
before   a layer naming a Mode::Once cue:  10 plays in 10 frames, live() 1
after    a layer naming a Mode::Once cue:   0 plays in 10 frames, live() 0, one line
both     a layer naming a Mode::Loop cue:   1 play,  live() 1
both     a layer naming an unknown cue:     0 plays, refused with its reason
```

The loop and unknown rows are the **controls**: they must not move, and they did not.

**A defect in my own measurement, caught here:** the first probe despawned the layer's voice **every frame for every
row**, so the `Mode::Loop` control also read 10 plays and the contrast proved nothing. A loop has no end to notice;
the probe now models an end only for a one-shot. Re-measured, the control is 1 play.

**Verdict: GREAT** after that probe fix (pass 1 re-ran green).

## Pass 3 — fresh eyes

**A different shape.** A three-layer score, a table edited **while the app runs** (`insert_resource` after the score),
and a cue that changes mode mid-run — the path a Cauldron toggle takes. The "becomes a loop" test is the one a panel
user exercises: fix the cue, and the layer starts without touching the score.

**Can a developer understand it without the code?** The line is `problems()`' own sentence, verbatim, so the report a
host prints and the log a panel reads agree word for word. The layer stays in `names()`, because deleting it would
hide the mistake a second way. `needs-decision/not-a-loop-refusal.md` records the one thing not done and what it costs
a consumer that reads `Refused` alone.

**What a fresh reader would still ask:** *why is this not a `Refusal`?* Because a new variant breaks every consumer's
`match`; the decision file states that, and the log line already carries the sentence.

**Verdict: GREAT.**
