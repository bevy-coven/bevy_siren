# research.md — iteration 40: the panel names the wrong duck, and had never shown a stinger

## The item, in two parts

**First, the stinger.** `music.stinger` has been in the example's table since it was written, and **`Stinger` was never
written** — no import, no step. So the panel had never shown the crate's second music feature, nor the one duck whose cause
is not a cue: `Entry::Ducked { by: "stinger" }`, where `by` names a **mechanism**.

**Second, and found by making the first work:** the bus row printed the **deepest** duck's depth beside the **newest**
`Ducked` entry's cause. With one duck they are the same rule, so iteration 31's choice was invisible. A stinger firing over
a dialogue line puts **two** ducks on one bus, and the row read:

```
Music  -9.9 dB   duck -8.0 dB by stinger
```

−8.0 belongs to `dialogue.greeting` (its rule is `by_db: -8.0`), and `stinger`'s is −6.0. **Two numbers from different
sources presented as one fact** — the exact class this loop has fixed at iterations 19, 31, 36 and 38.

## Sources

**S1. `Stinger`'s doc** — *"one hit, on the music bus, ducking what it plays over"*, and `Stinger::new` defaults to
*"ducking the music by 6 dB for a second"*. The example's table carries `music.stinger`, so the feature was authored and
never demonstrated.

**S2. `Entry::Ducked`'s `by` field** — *"What lowered it: a cue's name or `\"stinger\"`"*. The row's `by` is a **cause**, and
the row's `duck` is a **depth**; they must describe the same rule.

**S3. `SirenMix::ducked`** — *"Each bus's current duck in dB"*, the **minimum** over every active rule (`duck_db`'s doc: *"the
deeper one wins"*). Nothing ties it to any one entry.

**S4. R6 (vendored: `docs/siren.md`)** — *"everything a play decides is readable"*. A depth whose cause is named and wrong
is worse than a depth with no cause: it sends a person to the wrong rule.

## The technique

1. **A `stinger` step** — `Stinger::new("music.stinger")`, in the default script before `refuse`.
2. **The row names the entry whose depth it prints.** It finds the newest `Ducked` for the bus and compares **its** `by_db`
   with the mix's depth:
   - within 0.5 dB → that rule *is* the deepest: `duck -4.0 dB by dialogue.greeting`;
   - otherwise → the depth is another rule's: `duck -8.0 dB (newest: stinger at -6.0)`, naming both so neither number is
     orphaned;
   - no entry → the depth alone, as before.
3. **Measured against the row's width**: the two-duck clause is 52 characters and the row spans x 24..817 of 1280, so it
   fits — the four bus bands are still evenly spaced (196, 227, 260, 292), no wrap.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a stinger ducks what it plays over | −6 dB for 1 s by default | S1 |
| `Ducked.by` names a cue **or** `"stinger"` | — | S2 |
| `SirenMix::ducked` is the **minimum** over active rules | — | S3 |
| a depth's named cause must be the depth's cause | — | S4 |
| the two-duck clause on the `Music` row | 52 chars, x 24..817 | measured |

## What good sounds like

A panel whose `Music` row reads `duck -8.0 dB (newest: stinger at -6.0)` while a stinger plays over a dialogue line — the
deepest number and the newest cause, each labelled — with the log's two top lines naming the stinger's firing and its
duck. A single-duck bus keeps the short form. No sound changes.
