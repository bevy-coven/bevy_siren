# research.md — iteration 31: the bus row says how far, and not by what

## The item

The panel's bus row read:

```
Music       -9.4 dB   duck -8.0 dB   2 / 12 voices
```

**How far**, never **by what**. A person watching has to find the matching `Ducked` line in the log to learn which cue
lowered the bus — while `Entry::Ducked` exists to say exactly that, and `SirenMix::deepest_duck`'s own doc reads *"for a
panel that says what a line of dialogue did"*. The panel it was written for did not use it.

After:

```
Music       -9.5 dB   duck -8.0 dB by dialogue.greeting   2 / 12 voices
```

which is the log line's own claim (`1.57s dialogue.greeting ducked Music by -8.0 dB`) repeated where a person is looking.

## Sources

**S1. `Entry::Ducked`'s fields** — `bus`, `by_db`, and `by`: *"What lowered it: a cue's name or `\"stinger\"`"*. The crate records
the answer; the panel showed two of the three fields.

**S2. `SirenMix::deepest_duck`'s doc** — *"The bus ducked furthest this frame, **for a panel that says what a line of dialogue
did**."* A method written for this panel and unused by it is either a dead API or a missing view; this iteration makes it
the latter.

**S3. R6 (vendored: `docs/research/siren.md`)** — *"everything a play decides is readable"*. A duck is a decision the mix
makes every frame, and *which cue caused it* is the part a person needs to act on.

**S4. Iteration 30's trap** — a panel is a **fixed column**: a row that wraps breaks the alignment beneath it. The measured
row is **50 characters today and 71 with the annotation**, against roughly **114 columns** that fit at 1280×720, so the
annotation fits with room to spare — and the capture is the proof, not the arithmetic.

## The technique

The bus row, while `mix.ducked` is negative for that bus, finds the **newest** `Entry::Ducked` naming it in the `SirenLog`
the panel already holds:

```rust
log.recent(SirenLog::CAPACITY).find_map(|entry| match entry {
    Entry::Ducked { bus: b, by, .. } if b == bus => Some(by.as_str()),
    _ => None,
})
```

- **No new public field.** `SirenMix` can answer *how far* but not *by what*; `SirenLog` can answer *by what*, and the panel
  already holds it. The crate's own rule — one writer per number, one record per decision — is kept.
- **No staleness:** the annotation is rendered **inside** the `duck < -0.05` branch, so it disappears the frame the duck
  does. Measured: `during: duck -8.00 by line`, `after: duck +0.00 -> row shows no duck`.
- **A test on the rule, not on the view.** The example sits outside the suite; `tests/mix.rs` pins *"the newest `Ducked`
  naming a bus is the one a panel shows"*, so a change to `Entry` or `recent` cannot break the panel silently.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `Ducked` carries the cue that lowered the bus | `by` | S1 |
| `deepest_duck` exists for a panel that says what a line did | — | S2 |
| everything a play decides is readable | — | S3 |
| a bus row has ~114 columns at 1280×720; the annotated row is 71 | 71 | S4, measured |

## What good sounds like

A panel whose Music row reads `duck -8.0 dB by dialogue.greeting` while a line plays, and `-9.5 dB` with no duck clause
when it stops — the same words the log uses, in the place a person's eye already is. No sound changes.
