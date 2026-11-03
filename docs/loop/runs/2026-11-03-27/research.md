# research.md — iteration 27: a duck outlives the voice that justified it

## The item

`Duck`'s own doc: *"a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it recover over
`fade_s`"* — a duck is a rule about a **voice**. `DuckClock` keys its envelope on `(source_bus, target_bus) -> started`
and never learns that the voice stopped, so the full hold runs whatever happens to the source. Measured:

```
line played on Dialogue, then StopCue on frame 0, 60 fps
  frame  0  duck -8.00  voices 1
  frame  5  duck -8.00  voices 0     <- the sound is gone
  frame 15  duck -7.92  voices 0     <- hold still running
  frame 25  duck -5.79  voices 0
  frame 55  duck +0.00  voices 0
```

**0.85 s of duck for a sound that had already stopped** — 0.25 s of hold plus 0.6 s of fade. The skill names *"a duck that
does not recover"* as a rank-1 defect; this is the same failure arriving **late** rather than never.

## Sources

**S1. `Duck`'s own doc and the guide** — `docs/guide/buses.md`: *"a voice starting on `source` lowers `target` by
`by_db` for `hold_s`, then lets it recover over `fade_s`"*. The hold is bounded by the **voice**, and the doc says so.

**S2. Gregory, *Game Engine Architecture* 3e, §14.6.3** (vendored: `siren.md` R3, the crate's own rule) — a duck is
*"an envelope, not a volume write"*, evaluated every frame from real time. The envelope's *shape* is real time; its
**justification** is the source voice.

**S3. R3's own sentence in `siren.md`** — *"A voice starting on a **source** bus lowers a **target** bus by `by_db` for
`hold_s`, then recovers over `fade_s`."* Nothing about the voice ending, because the rule was written for the shape and
never for the edge.

**S4. `SirenMix::voices_on`** — *"voices a player can still hear"*, which excludes a voice already fading out. That is the
predicate a duck's justification should share: what a player hears, not what the entity table holds.

## The technique

`DuckClock::source_went_quiet(&live, &settings, now)` — called once per frame in `update_mix`, before the bus loop:

- for every rule whose `source` bus has **no live voice** (`live[source] == 0`), rewind that rule's `started` back by
  the hold that had **not yet elapsed**;
- **once only**: if the hold has already elapsed, leave it alone — rewinding again would restart the fade forever;
- a duck that fired this frame and whose source is already quiet begins its fade now.

`live` is counted in `update_mix` from the same predicate as `SirenMix::voices` (not `Stopping`), so the duck and the
panel agree on what is playing.

**Why not clamp the elapsed time in `duck_db`?** Tried first and measured: `since = max(elapsed, hold)` pins the fade at
exactly `hold_s` every frame, so it **never advances** and the duck holds forever — worse than the bug. The probe caught
it (`db=-8.00` with `since=0.250` on every frame). A one-time rewind moves the fade forward instead of freezing it.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a duck holds for `hold_s` because a **voice** is playing | 0.25 s default | S1, S3 |
| an envelope is evaluated every frame from real time | — | S2 |
| "a player can still hear" excludes a fading voice | — | S4 |

## What good sounds like

A line that stops mid-sentence releases the music **from that frame**, fading over the authored `fade_s` — 0.6 s for
`Music`, 0.4 s for `Effects` — instead of completing a hold the sound no longer justifies. A line that keeps playing
holds for its full `hold_s`; a second line mid-hold extends it. Measured after: stopped → `+0.00` by frame 40 (was 55);
playing → full hold; second line → extended.
